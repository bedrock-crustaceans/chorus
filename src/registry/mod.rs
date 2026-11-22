use crate::core::config::LevelConfig;
use crate::core::config::{AppConfigExt, Interval};
use crate::level::Level;
use crate::level::dimension_type::DimensionType;
use crate::level::level::{PollGenerationTask, ReleaseGenerationTask};
use crate::registry::block_registry::BlockRegistry;
use crate::resource::ResourcePacks;
use bevy_app::{App, Plugin, Startup, Update};
use bevy_ecs::prelude::{Commands, IntoScheduleConfigs, Local, Res, ResMut, SystemSet, World};
use chorus_core::schedule::{Priority, Scheduler, Tick, TickSet};
use chorus_level::generator::dimension::Dimension;
use chorus_worldgen::OverworldSettings;
use command_registry::CommandRegistry;
use glam::IVec3;
use item_registry::ItemRegistry;
use std::path::Path;
use std::process::exit;
use std::time::Instant;
use stored::{Outcome, choose, report};
use tracing::{error, info};
use worldgen::{WorldgenConfig, store_settings, stored_settings};

pub mod command_registry;
pub mod stored;
pub mod worldgen;

pub use chorus_block::block_registry;
pub use chorus_item::item_registry;
use chorus_worldgen::bedrock::end::EndGenerator;
use chorus_worldgen::bedrock::nether::NetherGenerator;
use chorus_worldgen::bedrock::overworld::OverworldGenerator;

pub struct Registry;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LevelStartup {
    Open,
    Dimensions,
    Defaults,
}

impl Plugin for Registry {
    fn build(&self, app: &mut App) {
        app.init_config::<WorldgenConfig>();
        Scheduler::register::<PollGenerationTask, _>(app.world_mut(), Priority::Background, Level::poll_generation);
        Scheduler::register::<ReleaseGenerationTask, _>(app.world_mut(), Priority::Background, Level::release_generation);

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
fn interval_elapsed(last: &mut Option<Instant>, interval: Interval) -> bool {
    let Some(interval) = interval.get() else { return false };
    let last = last.get_or_insert_with(Instant::now);
    if last.elapsed() < interval {
        return false;
    }
    *last = Instant::now();
    true
}

fn compact_level(level: Option<Res<Level>>, settings: Res<LevelConfig>, mut last: Local<Option<Instant>>) {
    let Some(storage) = level.as_ref().and_then(|level| level.storage()) else { return };
    storage.step_compaction();
    if interval_elapsed(&mut last, settings.database.auto_compaction) {
        let _ = storage.schedule_compaction();
    }
}

fn autosave(world: &mut World, mut last: Local<Option<Instant>>) {
    if !interval_elapsed(&mut last, world.resource::<LevelConfig>().autosave) {
        return;
    }
    crate::actor::storage::save_all_actors(world);
    if let Some(mut level) = world.get_resource_mut::<Level>()
        && level.unsaved_count() > 0
    {
        let saved = level.save();
        let _tag = tracing::info_span!("level").entered();
        info!("autosaved {saved} chunks");
    }
}

pub fn open_level(mut commands: Commands, registry: Res<BlockRegistry>, settings: Res<LevelConfig>) {
    let path = Path::new(WORLDS_DIRECTORY).join(&settings.name);
    let mut level = Level::open(path, &settings.name, settings.seed.value(), &registry, settings.database.compression);
    level.set_chunk_saving(settings.chunk_saving);
    level.set_retention_budget(settings.retention_budget);
    level.set_lookahead(settings.lookahead);
    level.set_chunk_cache(settings.chunk_cache);
    commands.insert_resource(level);
}

pub fn add_default_dimensions(mut level: ResMut<Level>, registry: Res<BlockRegistry>, worldgen: Res<WorldgenConfig>, mut server_state: ResMut<crate::server::ServerState>) {
    server_state.set_unique_id_base(level.start_count);
    if !level.has_dimension(DimensionType::Overworld.id()) {
        let (settings, outcome) = choose(stored_settings(&level), level.is_new(), worldgen.settings(), OverworldSettings::default());
        report(&level.name, "generator settings", &outcome);
        if matches!(outcome, Outcome::Created) {
            store_settings(&mut level, worldgen.layers());
        }
        let generator = OverworldGenerator::with_settings(level.seed, &registry, &settings).unwrap_or_else(|err| {
            error!("cannot build the overworld: {err}");
            exit(1);
        });
        let is_new = level.is_new();
        let dimension = level.insert_dimension(DimensionType::Overworld, generator);
        let spawn = is_new.then(|| find_spawn(dimension)).flatten();
        if let Some(spawn) = spawn {
            level.spawn = spawn;
        }
    }
    if !level.has_dimension(DimensionType::Nether.id()) {
        let generator = NetherGenerator::new(level.seed, &registry);
        level.insert_dimension(DimensionType::Nether, generator);
    }
    if !level.has_dimension(DimensionType::End.id()) {
        let generator = EndGenerator::new(level.seed, &registry);
        level.insert_dimension(DimensionType::End, generator);
    }
    let _tag = tracing::info_span!("level").entered();
    info!("\"{}\" spawn at {}", level.name, level.spawn);
    level.save_level_data();
}

fn find_spawn(dimension: &mut Dimension) -> Option<IVec3> {
    let job = dimension.start_locate("spawn", (0, 0), 0)?;
    loop {
        dimension.tick();
        if let Some((_, located)) = dimension.take_located().into_iter().find(|(id, _)| *id == job) {
            let located = located?;
            return Some(IVec3::new(located.x, located.y.unwrap_or(0), located.z));
        }
        std::thread::yield_now();
    }
}
