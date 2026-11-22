mod common;

use chorus_worldgen::bedrock::end::EndGenerator;

const HEIGHT: i32 = chorus_level::dimension_definition::DimensionDefinition::END.height;
use chorus_level::dimension_type::DimensionType;
use chorus_level::generator::dimension::Dimension;
use common::end::load;
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
fn end_locate_finds_a_nearby_city_deterministically() {
    for (x, z) in [(0, 0), (1000, 0), (-1000, 0), (0, 1500)] {
        let found = locate(DIMENSIONS_1, (x, z)).expect("a city");
        assert_eq!(locate(DIMENSIONS_1, (x, z)), Some(found));
        assert!((found.0 - (x >> 4)).abs() < 200 && (found.1 - (z >> 4)).abs() < 200, "from {x} {z}: {found:?}");
    }
}

const DIMENSION_SEED: i64 = DIMENSIONS_1;
const DIMENSION_CHUNKS: [(i32, i32); 4] = [(106, 3), (106, 4), (0, 0), (-3, 2)];

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
