mod common;

use chorus_block::block_registry::BlockRegistry;
use chorus_level::dimension_definition::DimensionDefinition;
use chorus_level::dimension_type::DimensionType;
use chorus_level::generator::dimension::Dimension;
use chorus_worldgen::shared::biome::Biome;
use chorus_worldgen::{BiomeSource, Checkerboard, OverworldGenerator, OverworldSettings, Preset};
use common::seeds::GENERAL_1;
use std::ops::Range;

const SEED: i64 = 1234567;
const REGION: [(i32, i32); 4] = [(0, 0), (1, 0), (0, 1), (1, 1)];
const MONUMENT: (i32, i32) = (14, 73);

fn square(centre: (i32, i32), radius: i32) -> Vec<(i32, i32)> {
    (centre.0 - radius..=centre.0 + radius)
        .flat_map(|x| (centre.1 - radius..=centre.1 + radius).map(move |z| (x, z)))
        .collect()
}

fn generate(seed: i64, region: &[(i32, i32)], settings: &OverworldSettings, registry: &BlockRegistry) -> Dimension {
    generate_defined(seed, region, settings, DimensionDefinition::OVERWORLD, registry)
}

fn generate_defined(seed: i64, region: &[(i32, i32)], settings: &OverworldSettings, definition: DimensionDefinition, registry: &BlockRegistry) -> Dimension {
    bevy_tasks::AsyncComputeTaskPool::get_or_init(bevy_tasks::TaskPool::default);
    let generator = OverworldGenerator::with_definition(seed, registry, settings, definition).unwrap();
    let mut dimension = Dimension::with_definition(DimensionType::Overworld, definition, generator);
    let order = common::with_ring(region);
    dimension.request_chunks(&order);
    let mut done = 0;
    while done < order.len() {
        done += dimension.tick().len();
        std::thread::yield_now();
    }
    dimension
}

fn count(dimension: &Dimension, region: &[(i32, i32)], registry: &BlockRegistry, identifier: &str, heights: Range<i32>) -> usize {
    let wanted = registry.get_block_id(identifier).unwrap();
    region
        .iter()
        .map(|&(chunk_x, chunk_z)| {
            let chunk = dimension.get_chunk(chunk_x, chunk_z).expect("generated");
            (0..16u8)
                .flat_map(|x| (0..16u8).map(move |z| (x, z)))
                .flat_map(|(x, z)| heights.clone().map(move |y| (x, y, z)))
                .filter(|&(x, y, z)| chunk.get_block(x, y, z, 0) == Some(wanted))
                .count()
        })
        .sum()
}

fn highest_solid(dimension: &Dimension, region: &[(i32, i32)], air: i32) -> i32 {
    let mut highest = i32::MIN;
    for &(chunk_x, chunk_z) in region {
        let chunk = dimension.get_chunk(chunk_x, chunk_z).expect("generated");
        for x in 0..16u8 {
            for z in 0..16u8 {
                let range = dimension.definition().min_y..dimension.definition().top();
                if let Some(top) = range.rev().find(|&y| chunk.get_block(x, y, z, 0).is_some_and(|block| block != air)) {
                    highest = highest.max(top);
                }
            }
        }
    }
    highest
}

fn fingerprint(dimension: &Dimension, region: &[(i32, i32)]) -> Vec<u8> {
    region.iter().flat_map(|&(x, z)| dimension.get_chunk(x, z).expect("generated").serialize()).collect()
}

#[test]
fn every_preset_generates_and_changes_the_world() {
    let registry = common::registry();
    let air = registry.get_block_id("minecraft:air").unwrap();
    let vanilla_world = generate(SEED, &REGION, &Preset::Default.settings(), &registry);
    let vanilla = fingerprint(&vanilla_world, &REGION);
    let vanilla_top = highest_solid(&vanilla_world, &REGION, air);
    for preset in [Preset::LargeBiomes, Preset::Amplified, Preset::AmplifiedLargeBiomes] {
        let dimension = generate(SEED, &REGION, &preset.settings(), &registry);
        assert_ne!(fingerprint(&dimension, &REGION), vanilla, "{preset:?}");
        let top = highest_solid(&dimension, &REGION, air);
        assert!(top > -64, "{preset:?}");
        if preset == Preset::Amplified {
            assert!(top > vanilla_top, "{top} against {vanilla_top}");
        }
    }
}

#[test]
fn single_biome_and_checkerboard_sources_place_their_biomes() {
    let registry = common::registry();
    let single = generate(SEED, &REGION, &OverworldSettings::single_biome(Biome::DESERT), &registry);
    let desert = Biome::DESERT.bedrock_id();
    for &(chunk_x, chunk_z) in &REGION {
        let chunk = single.get_chunk(chunk_x, chunk_z).expect("generated");
        for (x, y, z) in [(0u8, 64, 0u8), (7, 100, 9), (15, -30, 15), (3, 250, 12)] {
            assert_eq!(chunk.get_biome(x, y, z), Some(desert));
        }
    }
    let settings = OverworldSettings {
        biome_source: BiomeSource::Checkerboard(Checkerboard {
            biomes: vec![Biome::PLAINS, Biome::DESERT],
            scale: 2,
        }),
        ..OverworldSettings::default()
    };
    let region = [(2, 2), (6, 2)];
    let dimension = generate(SEED, &region, &settings, &registry);
    let biome_in = |chunk: (i32, i32)| dimension.get_chunk(chunk.0, chunk.1).expect("generated").get_biome(8, 64, 8);
    assert_eq!(biome_in((2, 2)), Some(Biome::PLAINS.bedrock_id()));
    assert_eq!(biome_in((6, 2)), Some(Biome::DESERT.bedrock_id()));
}

#[test]
fn structures_aquifers_and_ore_veins_can_be_switched_off() {
    let registry = common::registry();
    let monument = square(MONUMENT, 2);
    let bricks = |dimension: &Dimension| count(dimension, &monument, &registry, "minecraft:prismarine_bricks", 30..70);
    let without_structures = OverworldSettings {
        structures: false,
        ..OverworldSettings::default()
    };
    assert!(bricks(&generate(GENERAL_1, &monument, &OverworldSettings::default(), &registry)) > 1000);
    assert_eq!(bricks(&generate(GENERAL_1, &monument, &without_structures, &registry)), 0);

    let region = square((0, 0), 5);
    let lava = |dimension: &Dimension| count(dimension, &region, &registry, "minecraft:lava", -54..0);
    let raw = |dimension: &Dimension| count(dimension, &region, &registry, "minecraft:raw_copper_block", -64..60) + count(dimension, &region, &registry, "minecraft:raw_iron_block", -64..60);
    let with = generate(SEED, &region, &OverworldSettings::default(), &registry);
    assert!(lava(&with) > 0);
    assert!(raw(&with) > 0);
    let mut settings = without_structures;
    settings.terrain.aquifers = false;
    settings.terrain.ore_veins = false;
    let without = generate(SEED, &region, &settings, &registry);
    assert_eq!(lava(&without), 0);
    assert_eq!(raw(&without), 0);
}

#[test]
fn the_overworld_follows_a_custom_height() {
    let registry = common::registry();
    let air = registry.get_block_id("minecraft:air").unwrap();
    let bedrock = registry.get_block_id("minecraft:bedrock").unwrap();
    let region = square((0, 0), 1);
    for (min_y, height) in [(0, 256), (-128, 512)] {
        let definition = DimensionDefinition { min_y, height };
        let dimension = generate_defined(SEED, &region, &OverworldSettings::default(), definition, &registry);
        let chunk = dimension.get_chunk(0, 0).expect("generated");
        assert_eq!(chunk.sub_chunk_count(), definition.sub_chunk_count(), "{min_y} {height}");
        assert_eq!(chunk.min_sub_chunk_y(), definition.min_sub_chunk_y(), "{min_y} {height}");
        assert_eq!(chunk.get_block(0, min_y, 0, 0), Some(bedrock), "{min_y} {height}");
        assert_eq!(chunk.get_block(0, definition.max_y(), 0, 0), Some(air), "{min_y} {height}");
        let solid = highest_solid(&dimension, &region, air);
        assert!(solid > min_y + 8 && solid < definition.max_y(), "{min_y} {height}: {solid}");
    }
}

fn settle<G: chorus_level::generator::dimension::Generator>(dimension_type: DimensionType, definition: DimensionDefinition, generator: G, region: &[(i32, i32)]) -> Dimension {
    bevy_tasks::AsyncComputeTaskPool::get_or_init(bevy_tasks::TaskPool::default);
    let mut dimension = Dimension::with_definition(dimension_type, definition, generator);
    let order = common::with_ring(region);
    dimension.request_chunks(&order);
    let mut done = 0;
    while done < order.len() {
        done += dimension.tick().len();
        std::thread::yield_now();
    }
    dimension
}

#[test]
fn the_nether_follows_a_custom_height() {
    let registry = common::registry();
    let bedrock = registry.get_block_id("minecraft:bedrock").unwrap();
    let air = registry.get_block_id("minecraft:air").unwrap();
    let region = square((0, 0), 1);
    for (min_y, height) in [(0, 256), (-32, 192)] {
        let definition = DimensionDefinition { min_y, height };
        let generator = chorus_worldgen::NetherGenerator::with_definition(SEED, &registry, definition).unwrap();
        let dimension = settle(DimensionType::Nether, definition, generator, &region);
        let chunk = dimension.get_chunk(0, 0).expect("generated");
        assert_eq!(
            (chunk.sub_chunk_count(), chunk.min_sub_chunk_y()),
            (definition.sub_chunk_count(), definition.min_sub_chunk_y()),
            "{min_y} {height}"
        );
        assert_eq!(chunk.get_block(0, 0, 0, 0), Some(bedrock), "{min_y} {height}");
        assert_eq!(chunk.get_block(0, 127, 0, 0), Some(bedrock), "{min_y} {height}");
        assert_eq!(chunk.get_block(0, definition.max_y(), 0, 0), Some(air), "{min_y} {height}");
        let solid = highest_solid(&dimension, &region, air);
        assert!(solid > 8 && solid <= 127, "{min_y} {height}: {solid}");
    }
}

#[test]
fn the_end_follows_a_custom_height() {
    let registry = common::registry();
    let air = registry.get_block_id("minecraft:air").unwrap();
    let end_stone = registry.get_block_id("minecraft:end_stone").unwrap();
    let region = square((0, 0), 1);
    for (min_y, height) in [(0, 384), (-64, 320)] {
        let definition = DimensionDefinition { min_y, height };
        let generator = chorus_worldgen::EndGenerator::with_definition(SEED, &registry, definition).unwrap();
        let dimension = settle(DimensionType::End, definition, generator, &region);
        let chunk = dimension.get_chunk(0, 0).expect("generated");
        assert_eq!(
            (chunk.sub_chunk_count(), chunk.min_sub_chunk_y()),
            (definition.sub_chunk_count(), definition.min_sub_chunk_y()),
            "{min_y} {height}"
        );
        let stone = (0..16u8)
            .flat_map(|x| (0..16u8).map(move |z| (x, z)))
            .filter(|&(x, z)| (min_y..definition.top()).any(|y| chunk.get_block(x, y, z, 0) == Some(end_stone)))
            .count();
        assert!(stone > 0, "{min_y} {height}");
        assert!(highest_solid(&dimension, &region, air) < definition.top(), "{min_y} {height}");
    }
}
