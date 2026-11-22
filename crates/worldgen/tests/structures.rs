mod common;

use chorus_worldgen::bedrock::overworld::OverworldGenerator;
use common::seeds::GENERAL_1;

#[test]
fn monument_is_built_at_its_start() {
    let start = (14, 73);
    let order: Vec<(i32, i32)> = (start.0 - 2..=start.0 + 2).flat_map(|x| (start.1 - 2..=start.1 + 2).map(move |z| (x, z))).collect();
    let registry = common::registry();
    let dimension = common::generate(GENERAL_1, &order, &registry);
    let count = |identifier: &str| {
        order
            .iter()
            .map(|&(x, z)| {
                let chunk = dimension.get_chunk(x, z).expect("generated");
                (0..16u8)
                    .flat_map(|bx| (0..16u8).map(move |bz| (bx, bz)))
                    .flat_map(|(bx, bz)| (30..70).map(move |y| (bx, y, bz)))
                    .filter(|&(bx, y, bz)| {
                        chunk
                            .get_block(bx, y, bz, 0)
                            .and_then(|id| registry.get_permutation(id))
                            .is_some_and(|permutation| permutation.get_identifier() == identifier)
                    })
                    .count()
            })
            .sum::<usize>()
    };
    assert!(count("minecraft:prismarine_bricks") > 1000);
    assert!(count("minecraft:sea_lantern") > 20);
    assert!(count("minecraft:gold_block") == 8);
}

#[test]
fn mansion_is_built_at_its_start() {
    let start = (-39, -448);
    let order: Vec<(i32, i32)> = (start.0 - 2..=start.0 + 2).flat_map(|x| (start.1 - 2..=start.1 + 2).map(move |z| (x, z))).collect();
    let registry = common::registry();
    let dimension = common::generate(GENERAL_1, &order, &registry);
    let count = |identifier: &str| {
        order
            .iter()
            .map(|&(x, z)| {
                let chunk = dimension.get_chunk(x, z).expect("generated");
                (0..16u8)
                    .flat_map(|bx| (0..16u8).map(move |bz| (bx, bz)))
                    .flat_map(|(bx, bz)| (60..130).map(move |y| (bx, y, bz)))
                    .filter(|&(bx, y, bz)| {
                        chunk
                            .get_block(bx, y, bz, 0)
                            .and_then(|id| registry.get_permutation(id))
                            .is_some_and(|permutation| permutation.get_identifier() == identifier)
                    })
                    .count()
            })
            .sum::<usize>()
    };
    assert!(count("minecraft:dark_oak_planks") > 1500);
    assert!(count("minecraft:cobblestone") > 300);
    assert!(count("minecraft:chest") > 0);
}

#[test]
fn outpost_tower_is_built_at_its_start() {
    let start = (27, 37);
    let order: Vec<(i32, i32)> = (start.0 - 1..=start.0 + 3).flat_map(|x| (start.1 - 1..=start.1 + 3).map(move |z| (x, z))).collect();
    let registry = common::registry();
    let dimension = common::generate(GENERAL_1, &order, &registry);
    let count = |identifier: &str| {
        order
            .iter()
            .map(|&(x, z)| {
                let chunk = dimension.get_chunk(x, z).expect("generated");
                (0..16u8)
                    .flat_map(|bx| (0..16u8).map(move |bz| (bx, bz)))
                    .flat_map(|(bx, bz)| (-64..200).map(move |y| (bx, y, bz)))
                    .filter(|&(bx, y, bz)| {
                        chunk
                            .get_block(bx, y, bz, 0)
                            .and_then(|id| registry.get_permutation(id))
                            .is_some_and(|permutation| permutation.get_identifier() == identifier)
                    })
                    .count()
            })
            .sum::<usize>()
    };
    assert!(count("minecraft:dark_oak_planks") > 100);
    assert!(count("minecraft:birch_planks") > 100);
}

fn count_blocks(seed: i64, chunks: ((i32, i32), (i32, i32)), identifiers: &[&str]) -> Vec<usize> {
    let ((x0, z0), (x1, z1)) = chunks;
    let order: Vec<(i32, i32)> = (x0..=x1).flat_map(|x| (z0..=z1).map(move |z| (x, z))).collect();
    let registry = common::registry();
    let dimension = common::generate(seed, &order, &registry);
    let mut counts = vec![0; identifiers.len()];
    for &(x, z) in &order {
        let chunk = dimension.get_chunk(x, z).expect("generated");
        for (bx, bz, y) in (0..16u8).flat_map(|bx| (0..16u8).flat_map(move |bz| (-64..200).map(move |y| (bx, bz, y)))) {
            let Some(permutation) = chunk.get_block(bx, y, bz, 0).and_then(|id| registry.get_permutation(id)) else {
                continue;
            };
            if let Some(position) = identifiers.iter().position(|identifier| permutation.get_identifier() == *identifier) {
                counts[position] += 1;
            }
        }
    }
    counts
}

#[test]
fn trail_ruins_are_built_at_their_start() {
    let counts = count_blocks(GENERAL_1, ((-82, -13), (-76, -7)), &["minecraft:mud_bricks", "minecraft:suspicious_gravel"]);
    assert!(counts[0] > 100);
    assert!(counts[1] > 0);
}

#[test]
fn abandoned_camp_is_built_with_its_tree() {
    let counts = count_blocks(GENERAL_1, ((-127, -60), (-124, -57)), &["minecraft:white_wool_stairs", "minecraft:straw_bed"]);
    assert!(counts[0] >= 20);
    assert!(counts[1] >= 2);
}

#[test]
fn starts_never_cycle_through_terrain() {
    use chorus_worldgen::bedrock::overworld::phases::StartsPhase;
    let supply = common::supply::GraphSupply::new(0);
    for z in 40..80 {
        for x in 150..200 {
            supply.require::<StartsPhase>((x, z));
        }
    }
}

#[test]
fn chunks_beside_a_start_generate_through_a_dimension() {
    use chorus_level::dimension_type::DimensionType;
    use chorus_level::generator::dimension::Dimension;
    use std::time::{Duration, Instant};
    bevy_tasks::AsyncComputeTaskPool::get_or_init(bevy_tasks::TaskPool::default);
    let mut dimension = Dimension::new(DimensionType::Overworld, OverworldGenerator::new(0, &common::registry()).expect("the generator builds"));
    let wanted = [(164, 63), (175, 58), (160, 50)];
    dimension.request_chunks(&wanted);
    let deadline = Instant::now() + Duration::from_secs(900);
    while wanted.iter().any(|&(x, z)| dimension.get_chunk(x, z).is_none()) {
        assert!(Instant::now() < deadline, "a chunk did not generate");
        dimension.tick();
        std::thread::yield_now();
    }
}
