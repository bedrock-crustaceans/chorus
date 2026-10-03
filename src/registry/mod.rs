use crate::config::Config;
use crate::level::Level;
use crate::level::dimension_type::DimensionType;
use crate::level::generator::dimension::Dimension;
use crate::level::generator::r#impl::overworld::{Java, OverworldGenerator};
use crate::level::level::PollGenerationJob;
use crate::registry::block_registry::BlockRegistry;
use crate::resource::ResourcePacks;
use bevy_app::{App, Plugin, Startup, Update};
use bevy_ecs::prelude::{Commands, IntoScheduleConfigs, Res};
use command_registry::CommandRegistry;
use item_registry::ItemRegistry;
use std::collections::HashMap;
use tracing::info;

pub mod command_registry;

pub use chorus_block::block_registry;
pub use chorus_item::item_registry;

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
                init_level.after(BlockRegistry::init),
            ),
        )
        .add_systems(Update, Level::queue_poll_generation);
    }
}

pub fn init_level(mut commands: Commands, registry: Res<BlockRegistry>, config: Res<Config>) {
    let generator = OverworldGenerator::<Java>::new(config.level_seed as i64, &registry);
    let spawn = generator.find_spawn();

    info!("overworld spawn at {spawn}");

    commands.insert_resource(Level {
        dimensions: HashMap::from_iter([(0, Dimension::new(DimensionType::Overworld, generator))]),
        spawn,
    });
}
