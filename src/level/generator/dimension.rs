use std::collections::HashMap;

use crossbeam_channel::{Receiver, Sender};

use crate::level::chunk::Chunk;

use crate::level::generator::phase::{ErasedOutput, Phase};
use crate::level::generator::phase_graph::{PhaseGraph, RequestHandle};
use crate::level::generator::pos::ChunkPos;

pub trait Generator: Send + Sync + Sized + 'static {
    type Terminal: Phase<Self, Output = Chunk>;
}

pub trait WorldGenerator: Send + Sync {
    fn request_chunk(&self, x: i32, z: i32);
    fn tick(&mut self) -> Vec<(i32, i32, Chunk)>;
}

struct PhasedGenerator<G: Generator> {
    graph: PhaseGraph<G>,
    requests: RequestHandle<G>,
    finished_tx: Sender<(ChunkPos, ErasedOutput)>,
    finished_rx: Receiver<(ChunkPos, ErasedOutput)>,
}

impl<G: Generator> WorldGenerator for PhasedGenerator<G> {
    fn request_chunk(&self, x: i32, z: i32) {
        self.requests.request::<G::Terminal>(ChunkPos::new(x, z), Some(self.finished_tx.clone()));
    }

    fn tick(&mut self) -> Vec<(i32, i32, Chunk)> {
        self.graph.tick();

        self.finished_rx
            .try_iter()
            .map(|(cell, output)| (cell.x, cell.z, (*output.downcast::<Chunk>().unwrap()).clone()))
            .collect()
    }
}

pub struct Dimension {
    pub id: i32,
    pub min_sub_chunk_y: i8,
    pub max_sub_chunk_y: i8,
    generator: Box<dyn WorldGenerator>,
    chunks: HashMap<(i32, i32), Chunk>,
}

impl Dimension {
    pub fn new<G: Generator>(id: i32, min_sub_chunk_y: i8, max_sub_chunk_y: i8, generator: G) -> Self {
        let graph = PhaseGraph::new(generator);
        let requests = graph.requests();
        let (finished_tx, finished_rx) = crossbeam_channel::unbounded();

        Self {
            id,
            min_sub_chunk_y,
            max_sub_chunk_y,
            generator: Box::new(PhasedGenerator::<G> {
                graph,
                requests,
                finished_tx,
                finished_rx,
            }),
            chunks: HashMap::new(),
        }
    }

    pub fn name(&self) -> &'static str {
        match self.id {
            1 => "nether",
            2 => "the_end",
            _ => "overworld",
        }
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    pub fn sub_chunk_count(&self) -> usize {
        (self.max_sub_chunk_y as i32 - self.min_sub_chunk_y as i32 + 1) as usize
    }

    pub fn request_chunk(&self, x: i32, z: i32) {
        if self.chunks.contains_key(&(x, z)) {
            return;
        }
        self.generator.request_chunk(x, z);
    }

    pub fn request_chunks(&self, positions: &[(i32, i32)]) {
        for &(x, z) in positions {
            self.request_chunk(x, z);
        }
    }

    pub fn tick(&mut self) -> Vec<(i32, i32)> {
        self.generator
            .tick()
            .into_iter()
            .map(|(x, z, chunk)| {
                self.chunks.insert((x, z), chunk);
                (x, z)
            })
            .collect()
    }

    pub fn get_chunk(&self, x: i32, z: i32) -> Option<&Chunk> {
        self.chunks.get(&(x, z))
    }

    pub fn get_chunk_mut(&mut self, x: i32, z: i32) -> Option<&mut Chunk> {
        self.chunks.get_mut(&(x, z))
    }

    pub fn get_block(&self, x: i32, y: i32, z: i32, layer: usize) -> Option<i32> {
        self.chunks.get(&(x >> 4, z >> 4)).and_then(|chunk| chunk.get_block((x & 0xF) as u8, y, (z & 0xF) as u8, layer))
    }

    pub fn set_block(&mut self, x: i32, y: i32, z: i32, layer: usize, block_id: i32) -> bool {
        match self.chunks.get_mut(&(x >> 4, z >> 4)) {
            Some(chunk) => chunk.set_block((x & 0xF) as u8, y, (z & 0xF) as u8, layer, block_id),
            None => false,
        }
    }
}
