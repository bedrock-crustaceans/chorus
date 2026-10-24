use std::collections::{HashMap, HashSet, VecDeque};
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

#[derive(Clone, Debug, Default)]
pub struct GenerationStats {
    pub nodes: usize,
    pub waiting: usize,
    pub discarded: usize,
    pub completed: usize,
    pub retained: Vec<(&'static str, usize)>,
    pub started: Vec<(&'static str, u64)>,
}

pub trait WorldGenerator: Send + Sync {
    fn request_chunk(&mut self, x: i32, z: i32);
    fn set_focus(&mut self, focus: &[(i32, i32)], radius: i32);
    fn cancel_chunk(&mut self, x: i32, z: i32);
    fn tick(&mut self) -> Vec<(i32, i32, Result<Chunk, Arc<PhaseFailure>>)>;
    fn has_pending_work(&self) -> bool;
    fn release(&mut self) -> bool {
        false
    }
    fn retention_report(&self) -> String {
        String::new()
    }
    fn stats(&self) -> GenerationStats {
        GenerationStats::default()
    }
    fn set_retention_budget(&mut self, _budget: f64) {}
    fn set_lookahead(&mut self, _lookahead: i32) {}

    #[cfg(feature = "debug")]
    fn debug_graph(&self) -> Option<&dyn crate::generator::debug::DebugGraph> {
        None
    }
}

pub struct Dimension {
    pub dimension_type: DimensionType,
    generator: Box<dyn WorldGenerator>,
    chunks: HashMap<(i32, i32), Arc<Chunk>>,
    storage: Option<Arc<LevelStorage>>,
    chunk_saving: ChunkSaving,
    persist: HashSet<(i32, i32)>,
    available: Vec<(i32, i32)>,
    unloaded: Vec<(i32, i32)>,
    loading: HashSet<(i32, i32)>,
    cancelled: HashSet<(i32, i32)>,
    loads: (Sender<LoadResult>, Receiver<LoadResult>),
    cold: HashMap<(i32, i32), Arc<Chunk>>,
    cold_order: VecDeque<(i32, i32)>,
    cold_capacity: usize,
    chunk_cache: f64,
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
            cold: HashMap::new(),
            cold_order: VecDeque::new(),
            cold_capacity: 0,
            chunk_cache: 0.0,
        }
    }

    pub fn with_storage(mut self, storage: Arc<LevelStorage>) -> Self {
        self.storage = Some(storage);
        self
    }

    pub fn set_chunk_saving(&mut self, chunk_saving: ChunkSaving) {
        self.chunk_saving = chunk_saving;
    }

    /// Keeps chunks that left every focus's view for a while, up to this multiple of the chunks in view, so one that is walked back to does not generate again.
    pub fn set_chunk_cache(&mut self, chunk_cache: f64) {
        self.chunk_cache = chunk_cache.max(0.0);
    }

    pub fn cached_count(&self) -> usize {
        self.cold.len()
    }

    fn keep_cold(&mut self, position: (i32, i32), chunk: Arc<Chunk>) {
        if self.cold.insert(position, chunk).is_none() {
            self.cold_order.push_back(position);
        }
        self.trim_cold();
    }

    fn trim_cold(&mut self) {
        while self.cold.len() > self.cold_capacity {
            let Some(oldest) = self.cold_order.pop_front() else { break };
            self.cold.remove(&oldest);
        }
    }

    pub fn persist_chunk(&mut self, x: i32, z: i32) {
        if self.storage.is_some() {
            if let Some(chunk) = self.chunks.get_mut(&(x, z)) {
                Arc::make_mut(chunk).mark_dirty();
            } else {
                self.persist.insert((x, z));
            }
        }
        self.request_chunk(x, z);
    }

    pub fn has_available(&self) -> bool {
        !self.available.is_empty()
    }

    pub fn has_unloaded(&self) -> bool {
        !self.unloaded.is_empty()
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
                Arc::make_mut(chunk).mark_saved();
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
        if let Some(chunk) = self.cold.remove(&(x, z)) {
            self.cold_order.retain(|&position| position != (x, z));
            if self.loading.insert((x, z)) {
                let _ = self.loads.0.send((x, z, Ok(Some(Arc::unwrap_or_clone(chunk)))));
            }
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

    pub fn set_focus(&mut self, focus: &[(i32, i32)], radius: i32) {
        let view_chunks = std::f64::consts::PI * (radius as f64).powi(2);
        self.cold_capacity = (self.chunk_cache * view_chunks) as usize * focus.len();
        self.trim_cold();
        self.generator.set_focus(focus, radius);
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
                    self.chunks.entry((x, z)).or_insert_with(|| Arc::new(chunk));
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
                    self.chunks.insert((x, z), Arc::new(chunk));
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
            if let Some(chunk) = self.chunks.remove(&position) {
                self.keep_cold(position, chunk);
            }
            self.unloaded.push(position);
            unloaded += 1;
        }
        if staged && let Some(storage) = &self.storage {
            storage.schedule_flush();
        }
        unloaded
    }

    pub fn retention_report(&self) -> String {
        self.generator.retention_report()
    }

    pub fn set_retention_budget(&mut self, budget: f64) {
        self.generator.set_retention_budget(budget);
    }

    pub fn set_lookahead(&mut self, lookahead: i32) {
        self.generator.set_lookahead(lookahead);
    }

    pub fn generation_stats(&self) -> GenerationStats {
        self.generator.stats()
    }

    #[cfg(feature = "debug")]
    pub fn debug_graph(&self) -> Option<&dyn crate::generator::debug::DebugGraph> {
        self.generator.debug_graph()
    }

    pub fn release_generated(&mut self) -> bool {
        self.generator.release()
    }

    pub fn has_pending_generation(&self) -> bool {
        self.generator.has_pending_work() || !self.loading.is_empty()
    }

    pub fn get_chunk(&self, x: i32, z: i32) -> Option<&Chunk> {
        self.chunks.get(&(x, z)).map(|chunk| &**chunk)
    }

    pub fn chunk_handle(&self, x: i32, z: i32) -> Option<Arc<Chunk>> {
        self.chunks.get(&(x, z)).cloned()
    }

    pub fn holds_chunk(&self, x: i32, z: i32, chunk: &Arc<Chunk>) -> bool {
        self.chunks.get(&(x, z)).is_some_and(|held| Arc::ptr_eq(held, chunk))
    }

    pub fn get_block(&self, x: i32, y: i32, z: i32, layer: usize) -> Option<i32> {
        self.chunks.get(&(x >> 4, z >> 4)).and_then(|chunk| chunk.get_block((x & 0xF) as u8, y, (z & 0xF) as u8, layer))
    }

    pub fn set_block(&mut self, x: i32, y: i32, z: i32, layer: usize, block_id: i32) -> bool {
        let position = (x >> 4, z >> 4);
        let Some(chunk) = self.chunks.get_mut(&position) else { return false };
        Arc::make_mut(chunk).set_block((x & 0xF) as u8, y, (z & 0xF) as u8, layer, block_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::error::PhaseError;
    use crate::generator::phase::{Phase, PhaseInputs};
    use crate::generator::pos::ChunkPos;
    use bevy_tasks::{AsyncComputeTaskPool, TaskPool};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    static GENERATED: AtomicUsize = AtomicUsize::new(0);
    const WATCHED: i32 = 1000;

    struct Toy;

    impl Generator for Toy {
        type Terminal = ChunkPhase;
    }

    struct ChunkPhase;

    impl Phase<Toy> for ChunkPhase {
        type Output = Chunk;

        fn run(_: &Toy, cell: ChunkPos, _: &mut PhaseInputs<Toy>) -> Result<Chunk, PhaseError> {
            if cell.x == WATCHED {
                GENERATED.fetch_add(1, Ordering::Relaxed);
            }
            Ok(Chunk::new(cell.x, cell.z, -4, 24, 0, 1))
        }
    }

    fn load(dimension: &mut Dimension, x: i32, z: i32) {
        dimension.request_chunk(x, z);
        let start = Instant::now();
        while dimension.get_chunk(x, z).is_none() {
            dimension.tick();
            assert!(start.elapsed() < Duration::from_secs(30), "the chunk never loaded");
        }
    }

    #[test]
    fn a_chunk_that_just_left_the_view_comes_back_without_generating_again() {
        AsyncComputeTaskPool::get_or_init(TaskPool::default);
        let mut dimension = Dimension::new(DimensionType::Overworld, Toy);
        dimension.set_chunk_cache(1.0);
        dimension.set_focus(&[(WATCHED, 0)], 4);
        load(&mut dimension, WATCHED, 0);
        dimension.take_available();
        let generated = GENERATED.load(Ordering::Relaxed);

        assert_eq!(dimension.unload_chunks(|_, _| false), 1);
        assert_eq!((dimension.chunk_count(), dimension.cached_count()), (0, 1));
        assert_eq!(dimension.take_unloaded(), vec![(WATCHED, 0)]);

        load(&mut dimension, WATCHED, 0);
        assert_eq!(GENERATED.load(Ordering::Relaxed), generated, "it came from the cache");
        assert_eq!(dimension.cached_count(), 0);
        assert_eq!(dimension.take_available(), vec![(WATCHED, 0)], "actors are loaded for it again");
    }

    #[test]
    fn the_cache_is_bounded_and_empties_when_nobody_is_in_view() {
        AsyncComputeTaskPool::get_or_init(TaskPool::default);
        let mut dimension = Dimension::new(DimensionType::Overworld, Toy);
        dimension.set_chunk_cache(0.1);
        dimension.set_focus(&[(0, 0)], 10);
        for x in 0..40 {
            load(&mut dimension, x, 0);
        }
        dimension.unload_chunks(|_, _| false);
        assert_eq!(dimension.cached_count(), (0.1 * std::f64::consts::PI * 100.0) as usize);

        dimension.set_focus(&[], 10);
        assert_eq!(dimension.cached_count(), 0);
    }
}
