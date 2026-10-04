use crate::config::Config;
use crate::level::Level;
use crate::level::dimension_type::DimensionType;
use crate::level::generator::dimension::Dimension;
use crate::level::generator::r#impl::overworld::{Java, OverworldGenerator};
use crate::level::level::PollGenerationJob;
use crate::level::storage::{LevelData, LevelStorage};
use crate::registry::block_registry::BlockRegistry;
use crate::resource::ResourcePacks;
use bevy_app::{App, Plugin, Startup, Update};
use bevy_ecs::prelude::{Commands, IntoScheduleConfigs, Local, Res, ResMut};
use chorus_core::schedule::{Tick, TickSet};
use command_registry::CommandRegistry;
use item_registry::ItemRegistry;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tracing::{error, info, warn};

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
        .add_systems(Update, Level::queue_poll_generation)
        .add_systems(Tick, (autosave, compact_level).in_set(TickSet::Last));
    }
}

pub const WORLDS_DIRECTORY: &str = "worlds";
const AUTOSAVE_INTERVAL_TICKS: u32 = 1200;
const COMPACTION_INTERVAL_TICKS: u32 = 6000;

fn compact_level(level: Option<Res<Level>>, mut ticks: Local<u32>) {
    *ticks += 1;
    if *ticks < COMPACTION_INTERVAL_TICKS {
        return;
    }
    *ticks = 0;
    if let Some(storage) = level.as_ref().and_then(|level| level.storage.as_ref()) {
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

pub fn init_level(mut commands: Commands, registry: Res<BlockRegistry>, config: Res<Config>) {
    let storage = match LevelStorage::open(Path::new(WORLDS_DIRECTORY).join(&config.level_name), &registry) {
        Ok(storage) => Some(Arc::new(storage)),
        Err(err) => {
            error!("failed to open level \"{}\", it will not be saved: {err}", config.level_name);
            None
        }
    };
    let stored = storage.as_ref().and_then(|storage| match storage.read_level_data() {
        Ok(data) => data,
        Err(err) => {
            warn!("failed to read level data, starting from config: {err}");
            None
        }
    });

    let seed = stored.as_ref().map_or(config.level_seed as i64, |data| data.seed);
    if stored.is_some() && seed != config.level_seed as i64 {
        warn!(
            "level \"{}\" was created with seed {seed}, ignoring level_seed {} from the config",
            config.level_name, config.level_seed
        );
    }
    let generator = OverworldGenerator::<Java>::new(seed, &registry);
    let spawn = stored.as_ref().map_or_else(|| generator.find_spawn(), |data| data.spawn);

    match &storage {
        Some(storage) if stored.is_some() => info!("loaded level \"{}\" from {}, spawn at {spawn}", config.level_name, storage.path().display()),
        _ => info!("created level \"{}\" with seed {seed}, spawn at {spawn}", config.level_name),
    }

    let mut overworld = Dimension::new(DimensionType::Overworld, generator);
    if let Some(storage) = &storage {
        overworld = overworld.with_storage(storage.clone());
        let data = LevelData {
            name: config.level_name.clone(),
            seed,
            spawn,
        };
        if let Err(err) = storage.write_level_data(&data) {
            error!("failed to write level data: {err}");
        }
    }

    commands.insert_resource(Level {
        name: config.level_name.clone(),
        seed,
        dimensions: HashMap::from_iter([(0, overworld)]),
        spawn,
        storage,
    });
}
