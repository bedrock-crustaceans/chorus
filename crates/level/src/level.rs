use crate::BlockUpdatedMessage;
use crate::generator::dimension::Dimension;
use crate::storage::{LevelData, LevelStorage};

use bevy_ecs::message::MessageWriter;
use bevy_ecs::prelude::Resource;
use bevy_ecs::system::{Res, ResMut, SystemId};

use chorus_core::schedule::JobQueue;
use glam::IVec3;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::error;

#[derive(Resource)]
pub struct PollGenerationJob(pub SystemId);

#[derive(Resource)]
pub struct Level {
    pub name: String,
    pub seed: i64,
    pub dimensions: HashMap<i32, Dimension>,
    pub spawn: IVec3,
    pub storage: Option<Arc<LevelStorage>>,
}

impl Level {
    pub fn queue_poll_generation(job: Res<PollGenerationJob>, mut jobs: ResMut<JobQueue>) {
        jobs.push(job.0);
    }

    pub fn poll_generation(mut level: ResMut<Level>, job: Res<PollGenerationJob>, mut jobs: ResMut<JobQueue>) {
        let mut keep_going = false;
        for dimension in level.dimensions.values_mut() {
            if !dimension.tick().is_empty() {
                keep_going = true;
            }
            if dimension.has_pending_generation() {
                keep_going = true;
            }
        }

        if keep_going {
            jobs.push(job.0);
        }
    }

    pub fn save(&mut self) -> usize {
        let saved = self.dimensions.values_mut().map(Dimension::save).sum();
        if let Some(storage) = &self.storage {
            let data = LevelData {
                name: self.name.clone(),
                seed: self.seed,
                spawn: self.spawn,
            };
            if let Err(err) = storage.write_level_data(&data) {
                error!("failed to write level data: {err}");
            }
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
