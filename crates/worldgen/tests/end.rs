mod common;

use chorus_worldgen::bedrock::end::EndGenerator;

const HEIGHT: i32 = chorus_level::dimension_definition::DimensionDefinition::END.height;
use chorus_level::dimension_type::DimensionType;
use chorus_level::generator::dimension::Dimension;
use chorus_worldgen::shared::biome::pack_ids::THE_END;
use common::end::{Stages, graph, load};
use common::registry;
use common::seeds::DIMENSIONS_1;

fn locate(seed: i64, from: (i32, i32)) -> Option<(i32, i32)> {
    bevy_tasks::AsyncComputeTaskPool::get_or_init(bevy_tasks::TaskPool::default);
    let mut dimension = Dimension::new(DimensionType::End, EndGenerator::new(seed, &registry()));
    let job = dimension.start_locate("end_city", (from.0 >> 4, from.1 >> 4), 0)?;
    loop {
        dimension.tick();
        if let Some((_, found)) = dimension.take_located().into_iter().find(|(id, _)| *id == job) {
            return found.map(|located| (located.x, located.z));
        }
        std::thread::yield_now();
    }
}

#[test]
fn located_cities_match_the_server_answers() {
    let answers = [
        ((0, 0), (-872, -600)),
        ((1000, 0), (1704, 56)),
        ((-1000, 0), (-1176, -552)),
        ((0, 1000), (344, 1304)),
        ((0, -1000), (-232, -1224)),
        ((1500, 1500), (1384, 1064)),
    ];
    for ((x, z), wanted) in answers {
        let found = locate(DIMENSIONS_1, (x, z)).map(|(chunk_x, chunk_z)| (chunk_x * 16 + 8, chunk_z * 16 + 8));
        assert_eq!(found, Some(wanted), "from {x} {z}");
    }
}

#[test]
fn end_cities_start_beyond_the_main_island() {
    let generator = graph(DIMENSIONS_1);
    assert!(generator.city_starts(106, 3) > 0, "the known city start");
    for empty in [(0, 0), (7, -9), (20, 4)] {
        assert_eq!(generator.city_starts(empty.0, empty.1), 0, "{empty:?}");
    }
}

#[test]
fn islands_noise_puts_the_spikes_on_the_expected_circle() {
    let generator = EndGenerator::new(DIMENSIONS_1, &registry());
    let centres: Vec<(i32, i32)> = generator.spikes().iter().map(|spike| (spike.x, spike.z)).collect();
    assert_eq!(centres, [(42, 0), (33, 24), (12, 39), (-12, 39), (-33, 24), (-42, 0), (-33, -24), (-12, -39), (12, -39), (33, -24)]);
}

const DIMENSION_SEED: i64 = DIMENSIONS_1;
const CITY_CHUNK: (i32, i32) = (106, 3);
const DIMENSION_CHUNKS: [(i32, i32); 4] = [(106, 3), (106, 4), (0, 0), (-3, 2)];

fn purpur_cells(dimension: &Dimension, reference: &EndGenerator, chunk_x: i32, chunk_z: i32) -> usize {
    let generated = dimension.get_chunk(chunk_x, chunk_z).expect("generated");
    let mut found = 0;
    for x in 0..16 {
        for z in 0..16 {
            for y in 0..HEIGHT {
                let block = generated.get_block(x as u8, y, z as u8, 0).expect("inside the height range");
                if reference.blocks().runtime_name(block) == Some("minecraft:purpur_block") {
                    found += 1;
                }
            }
            for y in [0, 127, 255] {
                assert_eq!(generated.get_biome(x as u8, y, z as u8), Some(THE_END), "chunk {chunk_x} {chunk_z} biome {x} {y} {z}");
            }
        }
    }
    found
}

#[test]
fn the_end_dimension_builds_its_cities() {
    bevy_tasks::AsyncComputeTaskPool::get_or_init(bevy_tasks::TaskPool::default);
    let registry = registry();
    let reference = graph(DIMENSION_SEED);
    let mut dimension = Dimension::new(DimensionType::End, EndGenerator::new(DIMENSION_SEED, &registry));
    load(&mut dimension, &DIMENSION_CHUNKS);
    assert_eq!(dimension.name(), DimensionType::End.name());
    assert_eq!(dimension.id(), DimensionType::End.id());
    assert!(DIMENSION_CHUNKS.iter().all(|&(x, z)| dimension.get_chunk(x, z).is_some_and(|chunk| chunk.block_entities().is_empty())));
    assert!(reference.city_starts(CITY_CHUNK.0, CITY_CHUNK.1) > 0, "the sampled city chunk is a city start");
    for &(chunk_x, chunk_z) in &DIMENSION_CHUNKS {
        let found = purpur_cells(&dimension, &reference, chunk_x, chunk_z);
        if (chunk_x, chunk_z) == CITY_CHUNK {
            assert!(found > 0, "the city chunk holds purpur");
        } else {
            assert_eq!(found, 0, "chunk {chunk_x} {chunk_z}");
        }
    }
}

#[test]
fn end_dimension_chunks_do_not_depend_on_request_order() {
    bevy_tasks::AsyncComputeTaskPool::get_or_init(bevy_tasks::TaskPool::default);
    let registry = registry();
    let forward_positions = DIMENSION_CHUNKS;
    let backward_positions: Vec<(i32, i32)> = DIMENSION_CHUNKS.iter().rev().copied().collect();
    let mut forward = Dimension::new(DimensionType::End, EndGenerator::new(DIMENSION_SEED, &registry));
    load(&mut forward, &forward_positions);
    let mut backward = Dimension::new(DimensionType::End, EndGenerator::new(DIMENSION_SEED, &registry));
    load(&mut backward, &backward_positions);
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
