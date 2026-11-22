mod common;

use chorus_worldgen::bedrock::nether::NetherGenerator;

const HEIGHT: i32 = chorus_level::dimension_definition::DimensionDefinition::NETHER.height;
use chorus_level::dimension_type::DimensionType;
use chorus_level::generator::dimension::Dimension;
use common::nether::{Stages, graph, settled};
use common::registry;
use common::seeds::DIMENSIONS_1;

fn locate(seed: i64, name: &str, block_x: i32, block_z: i32) -> Option<(i32, i32)> {
    bevy_tasks::AsyncComputeTaskPool::get_or_init(bevy_tasks::TaskPool::default);
    let mut dimension = Dimension::new(DimensionType::Nether, NetherGenerator::new(seed, &registry()));
    let job = dimension.start_locate(name, (block_x >> 4, block_z >> 4), 0)?;
    loop {
        dimension.tick();
        if let Some((_, found)) = dimension.take_located().into_iter().find(|(id, _)| *id == job) {
            return found.map(|located| (located.x, located.z));
        }
        std::thread::yield_now();
    }
}

#[test]
fn nether_biomes_land_on_their_regions() {
    let generator = NetherGenerator::new(DIMENSIONS_1, &registry());
    assert_eq!(generator.biome_at(-40 * 16 + 8, -36 * 16 + 8), 181);
    assert_eq!(generator.biome_at(-40 * 16 + 8, -4 * 16 + 8), 179);
    assert_eq!(generator.biome_at(-40 * 16 + 8, 8), 178);
    assert_eq!(generator.biome_at(-12 * 16 + 8, -28 * 16 + 8), 180);
}

#[test]
fn nether_fortress_locate_matches_bds() {
    let answers: [(i32, i32, i32, i32); 6] = [
        (0, 0, 9, -10),
        (-77, 13, -12, 6),
        (123_457, -98_765, 7_755, -6_196),
        (-300_000, 250_000, -18_774, 15_654),
        (1_000_000, 1_000_000, 62_512, 62_478),
        (7_680, 7_680, 485, 480),
    ];
    for (x, z, chunk_x, chunk_z) in answers {
        assert_eq!(locate(DIMENSIONS_1, "fortress", x, z), Some((chunk_x, chunk_z)), "from {x} {z}");
    }
}

#[test]
fn nether_fences_join_their_brick_neighbours() {
    let generator = graph(DIMENSIONS_1);
    let chunk = generator.structured_blocks(10, -8);
    let sides: [(usize, i32, i32); 4] = [(0, 0, -1), (1, 1, 0), (2, 0, 1), (3, -1, 0)];
    let mut fence_cells = 0;
    for x in 0..16 {
        for z in 0..16 {
            for y in 0..HEIGHT {
                let id = chunk.get(x, y, z);
                let Some(code) = generator.ids.nether_brick_fence_sides.iter().position(|&fence| fence == id) else {
                    continue;
                };
                fence_cells += 1;
                for (bit, dx, dz) in sides {
                    let (neighbour_x, neighbour_z) = (x + dx, z + dz);
                    if !(0..16).contains(&neighbour_x) || !(0..16).contains(&neighbour_z) {
                        continue;
                    }
                    let neighbour = generator.blocks().name_of(chunk.get(neighbour_x, y, neighbour_z));
                    if neighbour == chorus_block::block_id::NETHER_BRICK {
                        assert_eq!(code >> bit & 1, 1, "fence at {x} {y} {z} side {bit}");
                    }
                }
            }
        }
    }
    assert!(fence_cells > 50, "the fortress holds {fence_cells} fences");
}

#[test]
fn nether_ruined_portal_places_blackstone_over_the_terrain() {
    let generator = graph(DIMENSIONS_1);
    let changed = |chunk_x: i32, chunk_z: i32| {
        let (carved, structured) = (generator.carved_blocks(chunk_x, chunk_z), generator.structured_blocks(chunk_x, chunk_z));
        let mut count = 0;
        for x in 0..16 {
            for z in 0..16 {
                for y in 0..HEIGHT {
                    if carved.get(x, y, z) != structured.get(x, y, z) {
                        count += 1;
                    }
                }
            }
        }
        count
    };
    let names: Vec<String> = {
        let mut found = Vec::new();
        for (dx, dz) in (-1..=1).flat_map(|dx| (-1..=1).map(move |dz| (dx, dz))) {
            let chunk = generator.structured_blocks(dx, -13 + dz);
            for x in 0..16 {
                for z in 0..16 {
                    for y in 0..HEIGHT {
                        let name = generator.blocks().name_of(chunk.get(x, y, z)).to_string();
                        if name.contains("blackstone") && !found.contains(&name) {
                            found.push(name);
                        }
                    }
                }
            }
        }
        found
    };
    let changed: i32 = (-1..=1).flat_map(|dx| (-1..=1).map(move |dz| (dx, dz))).map(|(dx, dz)| changed(dx, -13 + dz)).sum();
    assert!(changed > 100, "the portal and its neighbours change: {changed}");
    assert!(!names.is_empty(), "the portal turns the stone work to blackstone");
}

type Case = ((i32, i32), &'static [(i32, i32)]);

#[test]
fn nether_portal_and_bastion_do_not_depend_on_request_order() {
    let cases: [Case; 3] = [
        ((9, -10), &[(9, -11), (8, -10), (9, -9), (10, -10)]),
        ((0, -13), &[(-1, -13), (0, -14), (1, -13)]),
        ((30, -8), &[(29, -8), (30, -9), (31, -8)]),
    ];
    for (chunk, neighbours) in cases {
        let first = graph(DIMENSIONS_1);
        let direct = first.structured_blocks(chunk.0, chunk.1);
        let second = graph(DIMENSIONS_1);
        for &neighbour in neighbours {
            second.structured_blocks(neighbour.0, neighbour.1);
        }
        let later = second.structured_blocks(chunk.0, chunk.1);
        for x in 0..16 {
            for z in 0..16 {
                for y in 0..HEIGHT {
                    assert_eq!(direct.get(x, y, z), later.get(x, y, z), "chunk {chunk:?} cell {x} {y} {z}");
                }
            }
        }
    }
}

#[test]
fn nether_decoration_does_not_depend_on_request_order() {
    let registry = registry();
    let first = settled(&registry, &[(-3, -3)]);
    let second = settled(&registry, &[(-4, -4), (-3, -4), (-4, -3), (-3, -3)]);
    let direct = first.get_chunk(-3, -3).expect("generated");
    let later = second.get_chunk(-3, -3).expect("generated");
    for x in 0..16 {
        for z in 0..16 {
            for y in 0..HEIGHT {
                assert_eq!(direct.get_block(x, y, z, 0), later.get_block(x, y, z, 0), "cell {x} {y} {z}");
            }
        }
    }
}

#[test]
fn nether_plants_stand_on_nether_soil() {
    const PLANTS: [&str; 5] = [
        "minecraft:crimson_roots",
        "minecraft:warped_roots",
        "minecraft:nether_sprouts",
        "minecraft:crimson_fungus",
        "minecraft:warped_fungus",
    ];
    const SOILS: [&str; 8] = [
        "minecraft:crimson_nylium",
        "minecraft:warped_nylium",
        "minecraft:soul_soil",
        "minecraft:dirt",
        "minecraft:coarse_dirt",
        "minecraft:grass_block",
        "minecraft:mycelium",
        "minecraft:podzol",
    ];
    let registry = registry();
    let generator = NetherGenerator::new(DIMENSIONS_1, &registry);
    let dimension = settled(&registry, &[(-40, -4), (-40, 0), (-12, -28)]);
    let mut plants = 0;
    for chunk in [(-40, -4), (-40, 0), (-12, -28)] {
        let blocks = dimension.get_chunk(chunk.0, chunk.1).expect("generated");
        for x in 0..16u8 {
            for z in 0..16u8 {
                for y in 1..HEIGHT {
                    let here = blocks.get_block(x, y, z, 0).expect("inside the height range");
                    let Some(name) = generator.blocks().runtime_name(here) else { continue };
                    if !PLANTS.contains(&name) {
                        continue;
                    }
                    let below = blocks.get_block(x, y - 1, z, 0).expect("inside the height range");
                    let below = generator.blocks().runtime_name(below).expect("a known block");
                    assert!(SOILS.contains(&below), "plant at {x} {y} {z} on {below}");
                    plants += 1;
                }
            }
        }
    }
    assert!(plants > 0, "the regions hold plants");
}

const DIMENSION_SEED: i64 = DIMENSIONS_1;
const FORTRESS_CHUNK: (i32, i32) = (9, -10);
const DIMENSION_CHUNKS: [(i32, i32); 4] = [(9, -10), (30, -8), (-4, -4), (0, 0)];

#[test]
fn nether_rules_use_the_fast_lava() {
    let generator = NetherGenerator::new(DIMENSION_SEED, &registry());
    let rules = NetherGenerator::fluid_rules(&generator);
    assert_eq!(rules.motion(true), chorus_level::fluid::LiquidMotion { delay: 10, decay: 1 });
    assert_eq!(rules.motion(false), chorus_level::fluid::LiquidMotion { delay: 5, decay: 1 });
}

#[test]
fn the_nether_dimension_carries_its_biomes_without_block_entities() {
    let registry = registry();
    let reference = NetherGenerator::new(DIMENSION_SEED, &registry);
    let dimension = settled(&registry, &DIMENSION_CHUNKS);
    assert_eq!(dimension.name(), DimensionType::Nether.name());
    assert_eq!(dimension.id(), DimensionType::Nether.id());
    assert!(DIMENSION_CHUNKS.iter().all(|&(x, z)| dimension.get_chunk(x, z).is_some_and(|chunk| chunk.block_entities().is_empty())));
    assert!(
        locate(DIMENSION_SEED, "fortress", FORTRESS_CHUNK.0 * 16, FORTRESS_CHUNK.1 * 16) == Some(FORTRESS_CHUNK),
        "the sampled fortress chunk is a fortress start"
    );
    for &(chunk_x, chunk_z) in &DIMENSION_CHUNKS {
        let generated = dimension.get_chunk(chunk_x, chunk_z).expect("generated");
        for x in 0..16 {
            for z in 0..16 {
                let wanted = reference.biome_at(chunk_x * 16 + x, chunk_z * 16 + z);
                for y in [0, 127] {
                    assert_eq!(generated.get_biome(x as u8, y, z as u8), Some(wanted), "chunk {chunk_x} {chunk_z} biome {x} {y} {z}");
                }
            }
        }
    }
}

#[test]
fn nether_dimension_chunks_do_not_depend_on_request_order() {
    let registry = registry();
    let backward_positions: Vec<(i32, i32)> = DIMENSION_CHUNKS.iter().rev().copied().collect();
    let forward = settled(&registry, &DIMENSION_CHUNKS);
    let backward = settled(&registry, &backward_positions);
    for &(chunk_x, chunk_z) in &DIMENSION_CHUNKS {
        let first = forward.get_chunk(chunk_x, chunk_z).expect("generated");
        let second = backward.get_chunk(chunk_x, chunk_z).expect("generated");
        for x in 0..16u8 {
            for z in 0..16u8 {
                for y in 0..HEIGHT {
                    assert_eq!(first.get_block(x, y, z, 0), second.get_block(x, y, z, 0), "chunk {chunk_x} {chunk_z} cell {x} {y} {z}");
                }
            }
        }
    }
}

#[test]
fn nether_locate_finds_real_starts() {
    let generator = graph(DIMENSIONS_1);
    for start in [
        locate(DIMENSIONS_1, "fortress", 0, 0).expect("a fortress"),
        locate(DIMENSIONS_1, "bastion", 0, 250).expect("a bastion"),
        locate(DIMENSIONS_1, "ruined_portal", 0, 0).expect("a ruined portal"),
    ] {
        let carved = generator.carved_blocks(start.0, start.1);
        let structured = generator.structured_blocks(start.0, start.1);
        let changed = (0..16)
            .flat_map(|x| (0..16).flat_map(move |z| (0..HEIGHT).map(move |y| (x, y, z))))
            .filter(|&(x, y, z)| carved.get(x, y, z) != structured.get(x, y, z))
            .count();
        assert!(changed > 0, "{start:?}");
    }
}
