use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tracing::error;

use crate::chunk::Chunk;
use crate::dimension_type::DimensionType;
use crate::generator::error::PhaseFailure;
use crate::generator::phase::Phase;
use crate::generator::phase_graph::PhaseGraph;

pub trait Generator: Send + Sync + Sized + 'static {
    type Terminal: Phase<Self, Output = Chunk>;
}

pub trait WorldGenerator: Send + Sync {
    fn request_chunk(&mut self, x: i32, z: i32, priority: u32);
    fn cancel_chunk(&mut self, x: i32, z: i32);
    fn tick(&mut self) -> Vec<(i32, i32, Result<Chunk, Arc<PhaseFailure>>)>;
    fn has_pending_work(&self) -> bool;
}

pub struct Dimension {
    pub dimension_type: DimensionType,
    generator: Box<dyn WorldGenerator>,
    chunks: HashMap<(i32, i32), Chunk>,
    modified: HashSet<(i32, i32)>,
}

impl Dimension {
    pub fn new<G: Generator>(dimension_type: DimensionType, generator: G) -> Self {
        Self {
            dimension_type,
            generator: Box::new(PhaseGraph::new(generator)),
            chunks: HashMap::new(),
            modified: HashSet::new(),
        }
    }

    pub fn id(&self) -> i32 {
        self.dimension_type.id()
    }

    pub fn name(&self) -> &'static str {
        self.dimension_type.name()
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    pub fn request_chunk(&mut self, x: i32, z: i32, priority: u32) {
        if self.chunks.contains_key(&(x, z)) {
            return;
        }
        self.generator.request_chunk(x, z, priority);
    }

    pub fn request_chunks(&mut self, positions: &[(i32, i32)]) {
        for &(x, z) in positions {
            self.request_chunk(x, z, 0);
        }
    }

    pub fn cancel_chunks(&mut self, positions: &[(i32, i32)]) {
        for &(x, z) in positions {
            if !self.chunks.contains_key(&(x, z)) {
                self.generator.cancel_chunk(x, z);
            }
        }
    }

    pub fn tick(&mut self) -> Vec<(i32, i32)> {
        let mut generated = Vec::new();
        for (x, z, result) in self.generator.tick() {
            match result {
                Ok(chunk) => {
                    self.chunks.insert((x, z), chunk);
                    generated.push((x, z));
                }
                Err(failure) => error!("failed to generate chunk ({x}, {z}) in {}: {failure}", self.name()),
            }
        }
        generated
    }

    pub fn unload_chunks(&mut self, keep: impl Fn(i32, i32) -> bool) -> usize {
        let before = self.chunks.len();
        self.chunks.retain(|&(x, z), _| keep(x, z) || self.modified.contains(&(x, z)));
        before - self.chunks.len()
    }

    pub fn has_pending_generation(&self) -> bool {
        self.generator.has_pending_work()
    }

    pub fn get_chunk(&self, x: i32, z: i32) -> Option<&Chunk> {
        self.chunks.get(&(x, z))
    }

    pub fn get_block(&self, x: i32, y: i32, z: i32, layer: usize) -> Option<i32> {
        self.chunks.get(&(x >> 4, z >> 4)).and_then(|chunk| chunk.get_block((x & 0xF) as u8, y, (z & 0xF) as u8, layer))
    }

    pub fn set_block(&mut self, x: i32, y: i32, z: i32, layer: usize, block_id: i32) -> bool {
        let position = (x >> 4, z >> 4);
        let Some(chunk) = self.chunks.get_mut(&position) else { return false };
        let changed = chunk.set_block((x & 0xF) as u8, y, (z & 0xF) as u8, layer, block_id);
        if changed {
            self.modified.insert(position);
        }
        changed
    }
}
