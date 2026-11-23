mod common;

use std::sync::Arc;
use std::time::{Duration, Instant};

use chorus_level::dimension_type::DimensionType;
use chorus_level::generator::dimension::Dimension;
use chorus_level::storage::LevelStorage;
use chorus_worldgen::bedrock::overworld::OverworldGenerator;
use common::seeds::GENERAL_1;

fn dimension(storage: &Arc<LevelStorage>, registry: &chorus_block::block_registry::BlockRegistry) -> Dimension {
    Dimension::new(DimensionType::Overworld, OverworldGenerator::new(GENERAL_1, registry).expect("builds")).with_storage(storage.clone())
}

fn load(dimension: &mut Dimension, cells: &[(i32, i32)]) {
    dimension.request_chunks(cells);
    let start = Instant::now();
    while cells.iter().any(|&(x, z)| dimension.get_chunk(x, z).is_none()) {
        dimension.tick();
        std::thread::yield_now();
        assert!(start.elapsed() < Duration::from_secs(300));
    }
}

#[test]
fn pending_fluid_ticks_survive_unloading_and_reloading_from_disk() {
    bevy_tasks::AsyncComputeTaskPool::get_or_init(bevy_tasks::TaskPool::default);
    let registry = common::registry();
    let path = std::env::temp_dir().join(format!("chorus-fluid-persist-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    let storage = Arc::new(LevelStorage::open(&path, &registry, 1).expect("opens"));
    let cells: Vec<(i32, i32)> = (-3..=3).flat_map(|x| (-3..=3).map(move |z| (x, z))).collect();

    let mut first = dimension(&storage, &registry);
    load(&mut first, &cells);
    for &(x, z) in &cells {
        first.persist_chunk(x, z);
    }
    let before = first.pending_fluid_ticks();
    assert!(before > 0);
    assert_eq!(first.unload_chunks(|_, _| false), cells.len());
    assert_eq!(first.pending_fluid_ticks(), 0);
    storage.flush_blocking().expect("flushes");
    drop(first);

    let mut second = dimension(&storage, &registry);
    load(&mut second, &cells);
    assert_eq!(second.pending_fluid_ticks(), before);
    drop(second);
    let _ = std::fs::remove_dir_all(&path);
}
