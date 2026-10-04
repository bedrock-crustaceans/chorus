use crate::config::Config;
use crate::level::Level;
use crate::level::dimension_type::DimensionType;
use crate::level::generator::r#impl::overworld::{Java, OverworldGenerator};
use crate::level::level::PollGenerationJob;
use crate::registry::block_registry::BlockRegistry;
use crate::resource::ResourcePacks;
use bevy_app::{App, Plugin, Startup, Update};
use bevy_ecs::prelude::{Commands, IntoScheduleConfigs, Local, Res, ResMut, SystemSet};
use chorus_core::schedule::{Tick, TickSet};
use command_registry::CommandRegistry;
use item_registry::ItemRegistry;
use std::path::Path;
use tracing::info;

pub mod command_registry;

pub use chorus_block::block_registry;
pub use chorus_item::item_registry;

pub struct Registry;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LevelStartup {
    Open,
    Dimensions,
    Defaults,
}

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
                open_level.in_set(LevelStartup::Open),
                add_default_dimensions.in_set(LevelStartup::Defaults),
            ),
        )
        .configure_sets(Startup, (LevelStartup::Open, LevelStartup::Dimensions, LevelStartup::Defaults).chain().after(BlockRegistry::init))
        .add_systems(Update, Level::queue_poll_generation)
        .add_systems(Tick, (autosave, compact_level).in_set(TickSet::Last));
    }
}

pub const WORLDS_DIRECTORY: &str = "worlds";
const AUTOSAVE_INTERVAL_TICKS: u32 = 1200;
const COMPACTION_INTERVAL_TICKS: u32 = 6000;

fn compact_level(level: Option<Res<Level>>, mut ticks: Local<u32>) {
    if let Some(storage) = level.as_ref().and_then(|level| level.storage()) {
        storage.step_compaction();
    }
    *ticks += 1;
    if *ticks < COMPACTION_INTERVAL_TICKS {
        return;
    }
    *ticks = 0;
    if let Some(storage) = level.as_ref().and_then(|level| level.storage()) {
        storage.schedule_compaction();
    }
}

fn autosave(level: Option<ResMut<Level>>, mut ticks: Local<u32>) {
    *ticks += 1;
    if *ticks < AUTOSAVE_INTERVAL_TICKS {
        return;
    }
    *ticks = 0;
    if let Some(mut level) = level
        && level.unsaved_count() > 0
    {
        let saved = level.save();
        info!("autosaved {saved} chunks");
    }
}

pub fn open_level(mut commands: Commands, registry: Res<BlockRegistry>, config: Res<Config>) {
    let path = Path::new(WORLDS_DIRECTORY).join(&config.level.name);
    commands.insert_resource(Level::open(path, &config.level.name, config.level.seed.value(), &registry, config.level.compression_level));
}

pub fn add_default_dimensions(mut level: ResMut<Level>, registry: Res<BlockRegistry>) {
    if !level.has_dimension(DimensionType::Overworld.id()) {
        let generator = OverworldGenerator::<Java>::new(level.seed, &registry);
        if level.is_new() {
            level.spawn = generator.find_spawn();
        }
        level.insert_dimension(DimensionType::Overworld, generator);
    }
    info!("level \"{}\" spawn at {}", level.name, level.spawn);
    level.save_level_data();
}
