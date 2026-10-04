use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tracing::{error, warn};

use crate::chunk::Chunk;
use crate::dimension_type::DimensionType;
use crate::generator::error::PhaseFailure;
use crate::generator::phase::Phase;
use crate::generator::phase_graph::PhaseGraph;
use crate::storage::{LevelStorage, StorageResult, spawn_io};
use chorus_core::config::ChunkSaving;
use crossbeam_channel::{Receiver, Sender};

pub trait Generator: Send + Sync + Sized + 'static {
    type Terminal: Phase<Self, Output = Chunk>;
}

pub trait WorldGenerator: Send + Sync {
    fn request_chunk(&mut self, x: i32, z: i32);
    fn set_focus(&mut self, focus: &[(i32, i32)]);
    fn cancel_chunk(&mut self, x: i32, z: i32);
    fn tick(&mut self) -> Vec<(i32, i32, Result<Chunk, Arc<PhaseFailure>>)>;
    fn has_pending_work(&self) -> bool;
}

pub struct Dimension {
    pub dimension_type: DimensionType,
    generator: Box<dyn WorldGenerator>,
    chunks: HashMap<(i32, i32), Chunk>,
    storage: Option<Arc<LevelStorage>>,
    chunk_saving: ChunkSaving,
    persist: HashSet<(i32, i32)>,
    available: Vec<(i32, i32)>,
    unloaded: Vec<(i32, i32)>,
    loading: HashSet<(i32, i32)>,
    cancelled: HashSet<(i32, i32)>,
    loads: (Sender<LoadResult>, Receiver<LoadResult>),
}

type LoadResult = (i32, i32, StorageResult<Option<Chunk>>);

impl Dimension {
    pub fn new<G: Generator>(dimension_type: DimensionType, generator: G) -> Self {
        Self {
            dimension_type,
            generator: Box::new(PhaseGraph::new(generator)),
            chunks: HashMap::new(),
            storage: None,
            chunk_saving: ChunkSaving::default(),
            persist: HashSet::new(),
            available: Vec::new(),
            unloaded: Vec::new(),
            loading: HashSet::new(),
            cancelled: HashSet::new(),
            loads: crossbeam_channel::unbounded(),
        }
    }

    pub fn with_storage(mut self, storage: Arc<LevelStorage>) -> Self {
        self.storage = Some(storage);
        self
    }

    pub fn set_chunk_saving(&mut self, chunk_saving: ChunkSaving) {
        self.chunk_saving = chunk_saving;
    }

    pub fn persist_chunk(&mut self, x: i32, z: i32) {
        if self.storage.is_some() {
            if let Some(chunk) = self.chunks.get_mut(&(x, z)) {
                chunk.mark_dirty();
            } else {
                self.persist.insert((x, z));
            }
        }
        self.request_chunk(x, z);
    }

    pub fn take_available(&mut self) -> Vec<(i32, i32)> {
        std::mem::take(&mut self.available)
    }

    pub fn take_unloaded(&mut self) -> Vec<(i32, i32)> {
        std::mem::take(&mut self.unloaded)
    }

    pub fn is_persistent(&self) -> bool {
        self.storage.is_some()
    }

    pub fn unsaved_count(&self) -> usize {
        if self.storage.is_none() {
            return 0;
        }
        self.chunks.values().filter(|chunk| chunk.is_dirty()).count()
    }

    pub fn save(&mut self) -> usize {
        let dirty: Vec<(i32, i32)> = self.chunks.iter().filter(|(_, chunk)| chunk.is_dirty()).map(|(&position, _)| position).collect();
        let saved = dirty.into_iter().filter(|&position| self.save_chunk(position)).count();
        if let Some(storage) = &self.storage {
            storage.schedule_flush();
        }
        saved
    }

    fn save_chunk(&mut self, position: (i32, i32)) -> bool {
        let (Some(storage), Some(chunk)) = (&self.storage, self.chunks.get_mut(&position)) else {
            return false;
        };
        match storage.save_chunk(self.dimension_type, chunk) {
            Ok(()) => {
                chunk.mark_saved();
                true
            }
            Err(err) => {
                error!("failed to save chunk ({}, {}) in {}: {err}", position.0, position.1, self.name());
                false
            }
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

    pub fn request_chunk(&mut self, x: i32, z: i32) {
        if self.chunks.contains_key(&(x, z)) {
            return;
        }
        let Some(storage) = &self.storage else {
            self.generator.request_chunk(x, z);
            return;
        };
        self.cancelled.remove(&(x, z));
        if !self.loading.insert((x, z)) {
            return;
        }
        let (storage, sender, dimension_type) = (storage.clone(), self.loads.0.clone(), self.dimension_type);
        spawn_io(move || {
            let _ = sender.send((x, z, storage.load_chunk(dimension_type, x, z)));
        });
    }

    pub fn request_chunks(&mut self, positions: &[(i32, i32)]) {
        for &(x, z) in positions {
            self.request_chunk(x, z);
        }
    }

    pub fn set_focus(&mut self, focus: &[(i32, i32)]) {
        self.generator.set_focus(focus);
    }

    pub fn cancel_chunks(&mut self, positions: &[(i32, i32)]) {
        for &(x, z) in positions {
            if self.loading.contains(&(x, z)) {
                self.cancelled.insert((x, z));
            } else if !self.chunks.contains_key(&(x, z)) {
                self.generator.cancel_chunk(x, z);
            }
        }
    }

    pub fn tick(&mut self) -> Vec<(i32, i32)> {
        let mut generated = Vec::new();
        while let Ok((x, z, result)) = self.loads.1.try_recv() {
            self.loading.remove(&(x, z));
            let cancelled = self.cancelled.remove(&(x, z));
            match result {
                Ok(Some(chunk)) => {
                    self.chunks.entry((x, z)).or_insert(chunk);
                    self.persist.remove(&(x, z));
                    generated.push((x, z));
                }
                Ok(None) if !cancelled => self.generator.request_chunk(x, z),
                Ok(None) => {}
                Err(err) => {
                    warn!("failed to load chunk ({x}, {z}) in {}, regenerating it: {err}", self.name());
                    if !cancelled {
                        self.generator.request_chunk(x, z);
                    }
                }
            }
        }
        for (x, z, result) in self.generator.tick() {
            match result {
                Ok(mut chunk) => {
                    chunk.mark_saved();
                    if self.persist.remove(&(x, z)) || self.chunk_saving == ChunkSaving::All {
                        chunk.mark_dirty();
                    }
                    self.chunks.insert((x, z), chunk);
                    generated.push((x, z));
                }
                Err(failure) => error!("failed to generate chunk ({x}, {z}) in {}: {failure}", self.name()),
            }
        }
        self.available.extend(generated.iter().copied());
        generated
    }

    pub fn unload_chunks(&mut self, keep: impl Fn(i32, i32) -> bool) -> usize {
        let persistent = self.storage.is_some();
        let unloading: Vec<(i32, i32)> = self
            .chunks
            .iter()
            .filter(|&(&(x, z), chunk)| !keep(x, z) && (persistent || !chunk.is_dirty()))
            .map(|(&position, _)| position)
            .collect();
        let mut unloaded = 0;
        let mut staged = false;
        for position in unloading {
            if self.chunks[&position].is_dirty() {
                if !self.save_chunk(position) {
                    continue;
                }
                staged = true;
            }
            self.chunks.remove(&position);
            self.unloaded.push(position);
            unloaded += 1;
        }
        if staged && let Some(storage) = &self.storage {
            storage.schedule_flush();
        }
        unloaded
    }

    pub fn has_pending_generation(&self) -> bool {
        self.generator.has_pending_work() || !self.loading.is_empty()
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
        chunk.set_block((x & 0xF) as u8, y, (z & 0xF) as u8, layer, block_id)
    }
}
