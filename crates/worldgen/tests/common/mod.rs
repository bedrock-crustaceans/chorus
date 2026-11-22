#![allow(dead_code)]

pub mod end;
pub mod harness;
pub mod nether;
pub mod seeds;
pub mod supply;

use std::collections::HashMap;

use chorus_block::block_registry::BlockRegistry;
use chorus_block::r#impl::DEFINITIONS;
use chorus_level::dimension_type::DimensionType;
use chorus_level::generator::dimension::Dimension;
use chorus_worldgen::bedrock::overworld::OverworldGenerator;
use chorus_worldgen::shared::hash::{FNV_OFFSET_BASIS, fnv1a_mix};

pub fn registry() -> BlockRegistry {
    let mut registry = BlockRegistry::new();
    registry.register_all(DEFINITIONS.iter().copied());
    registry
}

pub fn with_ring(order: &[(i32, i32)]) -> Vec<(i32, i32)> {
    let mut all: std::collections::HashSet<(i32, i32)> = order.iter().copied().collect();
    for &(x, z) in order {
        for dx in -1..=1 {
            for dz in -1..=1 {
                all.insert((x + dx, z + dz));
            }
        }
    }
    let mut all: Vec<(i32, i32)> = all.into_iter().collect();
    all.sort_unstable();
    all
}

pub fn generate(seed: i64, order: &[(i32, i32)], registry: &BlockRegistry) -> Dimension {
    bevy_tasks::AsyncComputeTaskPool::get_or_init(bevy_tasks::TaskPool::default);
    let generator = OverworldGenerator::new(seed, registry).expect("the generator builds");
    let mut dimension = Dimension::new(DimensionType::Overworld, generator);
    let settle_order = with_ring(order);
    dimension.request_chunks(&settle_order);
    let mut done = 0;
    while done < settle_order.len() {
        done += dimension.tick().len();
        std::thread::yield_now();
    }
    dimension.settle_fluids(chorus_level::fluid::SETTLE_TICKS);
    dimension
}

pub const CENTERS: [(i32, i32); 8] = [(0, 0), (48, -100), (44, 200), (-36, 108), (-156, -92), (44, 52), (-92, -144), (100, 68)];
pub const RADIUS: i32 = 2;

pub fn positions() -> Vec<(i32, i32)> {
    CENTERS
        .iter()
        .flat_map(|&(cx, cz)| (-RADIUS..=RADIUS).flat_map(move |dx| (-RADIUS..=RADIUS).map(move |dz| (cx + dx, cz + dz))))
        .collect()
}

pub fn fingerprints(seed: i64, order: &[(i32, i32)]) -> HashMap<(i32, i32), u64> {
    let registry = registry();
    let dimension = generate(seed, order, &registry);
    order
        .iter()
        .map(|&(x, z)| {
            let hash = dimension
                .get_chunk(x, z)
                .expect("generated")
                .serialize()
                .iter()
                .fold(FNV_OFFSET_BASIS, |hash, &byte| fnv1a_mix(hash, u64::from(byte)));
            ((x, z), hash)
        })
        .collect()
}
