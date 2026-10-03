use crate::JobQueue;
use crate::config::Config;
use crate::level::BlockUpdatedMessage;
use crate::level::dimension_type::DimensionType;
use crate::level::generator::dimension::Dimension;
use crate::level::generator::r#impl::overworld::{Java, OverworldGenerator};
use crate::registry::block_registry::BlockRegistry;
use bevy_ecs::message::MessageWriter;
use bevy_ecs::prelude::{Commands, Resource};
use bevy_ecs::system::{Res, ResMut, SystemId};
use glam::IVec3;
use std::collections::HashMap;
use tracing::info;

#[derive(Resource)]
pub(crate) struct PollGenerationJob(pub(crate) SystemId);

#[derive(Resource)]
pub struct Level {
    pub dimensions: HashMap<i32, Dimension>,
    pub spawn: IVec3,
}

impl Level {
    pub fn init(mut commands: Commands, registry: Res<BlockRegistry>, config: Res<Config>) {
        let generator = OverworldGenerator::<Java>::new(config.level_seed as i64, &registry);
        let spawn = generator.find_spawn();
        info!("overworld spawn at {spawn}");

        let mut level = Level { dimensions: HashMap::new(), spawn };
        level.dimensions.insert(0, Dimension::new(DimensionType::Overworld, generator));

        commands.insert_resource(level);
    }

    pub(crate) fn queue_poll_generation(job: Res<PollGenerationJob>, mut jobs: ResMut<JobQueue>) {
        jobs.push(job.0);
    }

    pub(crate) fn poll_generation(mut level: ResMut<Level>, job: Res<PollGenerationJob>, mut jobs: ResMut<JobQueue>) {
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
