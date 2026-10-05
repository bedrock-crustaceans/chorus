use crate::BlockUpdatedMessage;
use crate::dimension_type::DimensionType;
use crate::generator::dimension::{Dimension, Generator};
use crate::storage::{LevelData, LevelStorage};
use chorus_block::block_registry::BlockRegistry;
use chorus_core::config::ChunkSaving;

use bevy_ecs::message::MessageWriter;
use bevy_ecs::prelude::Resource;
use bevy_ecs::system::{Res, ResMut, SystemId};

use chorus_core::schedule::JobQueue;
use glam::IVec3;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

const IDLE_POLL_WAIT: Duration = Duration::from_micros(500);

#[derive(Resource)]
pub struct PollGenerationJob(pub SystemId);

#[derive(Resource)]
pub struct Level {
    pub name: String,
    pub seed: i64,
    pub spawn: IVec3,
    pub start_count: i64,
    dimensions: HashMap<i32, Dimension>,
    storage: Option<Arc<LevelStorage>>,
    chunk_saving: ChunkSaving,
    is_new: bool,
}

impl Level {
    #[tracing::instrument(name = "level", skip_all)]
    pub fn open(path: impl AsRef<Path>, name: impl Into<String>, seed: i64, registry: &BlockRegistry, compression_level: u8) -> Self {
        let (path, name) = (path.as_ref(), name.into());
        let storage = match LevelStorage::open(path, registry, compression_level) {
            Ok(storage) => Arc::new(storage),
            Err(err) => {
                error!("failed to open level \"{name}\" at {}, it will not be saved: {err}", path.display());
                return Self::in_memory(name, seed);
            }
        };
        let stored = storage.read_level_data().unwrap_or_else(|err| {
            warn!("failed to read the level data of \"{name}\", starting it over: {err}");
            None
        });

        let mut level = Self::in_memory(name, seed);
        if let Some(data) = stored {
            if data.seed != seed {
                warn!("\"{}\" was created with seed {}, ignoring seed {seed}", level.name, data.seed);
            }
            info!("loaded \"{}\" from {}", level.name, path.display());
            level.seed = data.seed;
            level.spawn = data.spawn;
            level.start_count = data.start_count;
            level.is_new = false;
        } else {
            info!("created \"{}\" at {} with seed {seed}", level.name, path.display());
        }
        level.start_count += 1;
        level.storage = Some(storage);
        level
    }

    pub fn in_memory(name: impl Into<String>, seed: i64) -> Self {
        Self {
            name: name.into(),
            seed,
            spawn: IVec3::ZERO,
            start_count: 0,
            dimensions: HashMap::new(),
            storage: None,
            chunk_saving: ChunkSaving::default(),
            is_new: true,
        }
    }

    pub fn is_new(&self) -> bool {
        self.is_new
    }

    pub fn set_chunk_saving(&mut self, chunk_saving: ChunkSaving) {
        self.chunk_saving = chunk_saving;
        for dimension in self.dimensions.values_mut() {
            dimension.set_chunk_saving(chunk_saving);
        }
    }

    pub fn storage(&self) -> Option<&Arc<LevelStorage>> {
        self.storage.as_ref()
    }

    pub fn insert_dimension<G: Generator>(&mut self, dimension_type: DimensionType, generator: G) -> &mut Dimension {
        let mut dimension = Dimension::new(dimension_type, generator);
        if let Some(storage) = &self.storage {
            dimension = dimension.with_storage(storage.clone());
        }
        dimension.set_chunk_saving(self.chunk_saving);
        self.remove_dimension(dimension_type.id());
        self.dimensions.entry(dimension_type.id()).or_insert(dimension)
    }

    pub fn remove_dimension(&mut self, id: i32) -> Option<Dimension> {
        let mut dimension = self.dimensions.remove(&id)?;
        dimension.save();
        Some(dimension)
    }

    pub fn has_dimension(&self, id: i32) -> bool {
        self.dimensions.contains_key(&id)
    }

    pub fn dimensions(&self) -> impl Iterator<Item = &Dimension> {
        self.dimensions.values()
    }

    pub fn dimensions_mut(&mut self) -> impl Iterator<Item = &mut Dimension> {
        self.dimensions.values_mut()
    }

    #[tracing::instrument(name = "level", skip_all)]
    pub fn save_level_data(&self) {
        let Some(storage) = &self.storage else { return };
        let data = LevelData {
            name: self.name.clone(),
            seed: self.seed,
            spawn: self.spawn,
            start_count: self.start_count,
        };
        if let Err(err) = storage.write_level_data(&data) {
            error!("failed to write level data: {err}");
        }
    }

    pub fn queue_poll_generation(job: Res<PollGenerationJob>, mut jobs: ResMut<JobQueue>) {
        jobs.push(job.0);
    }

    pub fn poll_generation(mut level: ResMut<Level>, job: Res<PollGenerationJob>, mut jobs: ResMut<JobQueue>) {
        let mut progressed = false;
        let mut pending = false;
        for dimension in level.dimensions.values_mut() {
            progressed |= !dimension.tick().is_empty();
            pending |= dimension.has_pending_generation();
        }

        if !progressed && pending {
            std::thread::sleep(IDLE_POLL_WAIT);
        }
        if progressed || pending {
            jobs.push(job.0);
        }
    }

    pub fn save(&mut self) -> usize {
        let saved = self.dimensions.values_mut().map(Dimension::save).sum();
        self.save_level_data();
        saved
    }

    #[tracing::instrument(name = "level", skip_all)]
    pub fn save_blocking(&mut self) -> usize {
        let saved = self.save();
        if let Some(storage) = &self.storage
            && let Err(err) = storage.flush_blocking()
        {
            error!("failed to write level to disk: {err}");
        }
        saved
    }

    pub fn unsaved_count(&self) -> usize {
        self.dimensions.values().map(Dimension::unsaved_count).sum()
    }

    pub fn dimension(&self, id: i32) -> Option<&Dimension> {
        self.dimensions.get(&id)
    }

    pub fn dimension_mut(&mut self, id: i32) -> Option<&mut Dimension> {
        self.dimensions.get_mut(&id)
    }

    pub fn overworld(&self) -> &Dimension {
        self.dimensions.get(&0).expect("overworld dimension not initialised")
    }

    pub fn overworld_mut(&mut self) -> &mut Dimension {
        self.dimensions.get_mut(&0).expect("overworld dimension not initialised")
    }

    pub fn get_block(&self, dim: i32, x: i32, y: i32, z: i32, layer: usize) -> Option<i32> {
        self.dimension(dim)?.get_block(x, y, z, layer)
    }

    pub fn set_block(&mut self, dim: i32, x: i32, y: i32, z: i32, layer: usize, block_id: i32, writer: &mut MessageWriter<BlockUpdatedMessage>) -> bool {
        let changed = match self.dimension_mut(dim) {
            Some(d) => d.set_block(x, y, z, layer, block_id),
            None => return false,
        };
        if changed {
            writer.write(BlockUpdatedMessage {
                dimension_id: dim,
                x,
                y,
                z,
                layer,
                block_id,
            });
        }
        changed
    }
}
