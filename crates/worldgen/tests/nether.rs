mod common;

use chorus_worldgen::bedrock::nether::NetherGenerator;

const HEIGHT: i32 = chorus_level::dimension_definition::DimensionDefinition::NETHER.height;
use chorus_level::dimension_type::DimensionType;
use chorus_level::generator::dimension::Dimension;
use common::nether::settled;
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
fn nether_locate_finds_a_nearby_fortress_deterministically() {
    for (x, z) in [(0, 0), (-77, 13), (123_457, -98_765)] {
        let found = locate(DIMENSIONS_1, "fortress", x, z).expect("a fortress");
        assert_eq!(locate(DIMENSIONS_1, "fortress", x, z), Some(found));
        assert!((found.0 - (x >> 4)).abs() < 100 && (found.1 - (z >> 4)).abs() < 100, "from {x} {z}: {found:?}");
    }
}

const DIMENSION_SEED: i64 = DIMENSIONS_1;
const FORTRESS_CHUNK: (i32, i32) = (9, -10);
const DIMENSION_CHUNKS: [(i32, i32); 4] = [(9, -10), (30, -8), (-4, -4), (0, 0)];

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
