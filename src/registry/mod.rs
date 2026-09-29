use crate::level::level::{Level, PollGenerationJob};
use crate::registry::block_registry::BlockRegistry;
use crate::resource::ResourcePacks;
use bevy_app::{App, Plugin, Startup, Update};
use bevy_ecs::prelude::IntoScheduleConfigs;
use command_registry::CommandRegistry;
use item_registry::ItemRegistry;

pub mod block_registry;
pub mod command_registry;
pub mod item_registry;
pub mod structure_registry;

pub struct Registry;

impl Plugin for Registry {
    fn build(&self, app: &mut App) {
        let poll_generation_job = app.world_mut().register_system(Level::poll_generation);
        app.insert_resource(PollGenerationJob(poll_generation_job));

        app.add_systems(
            Startup,
            (
                BlockRegistry::init,
                CommandRegistry::init,
                ResourcePacks::load,
                ItemRegistry::init,
                Level::init.after(BlockRegistry::init),
            ),
        )
        .add_systems(Update, Level::queue_poll_generation);
    }
}
