mod common;

mod overworld {
    use crate::common::{fingerprints, positions};
    use chorus_worldgen::shared::hash::{FNV_OFFSET_BASIS, fnv1a_mix};
    use std::time::Instant;

    #[test]
    #[ignore = "run with: cargo test --release -p chorus_worldgen --test checksums -- --ignored output_checksum --nocapture"]
    fn output_checksum() {
        let order = positions();
        let start = Instant::now();
        let hashes = fingerprints(0, &order);
        let elapsed = start.elapsed();
        let total = order.iter().fold(FNV_OFFSET_BASIS, |total, cell| fnv1a_mix(total, hashes[cell]));
        println!("bedrock checksum {total:016x} over {} chunks in {elapsed:?}", order.len());
        assert_eq!(total, 0x1ae4a0d54dc07430);
    }
}

mod end {
    use crate::common::end::load;
    use crate::common::registry;
    use crate::common::seeds::DIMENSIONS_1;
    use chorus_level::dimension_type::DimensionType;
    use chorus_level::generator::dimension::Dimension;
    use chorus_worldgen::bedrock::end::EndGenerator;
    const HEIGHT: i32 = chorus_level::dimension_definition::DimensionDefinition::END.height;
    use std::time::Instant;

    use chorus_worldgen::shared::hash::{FNV_OFFSET_BASIS, fnv1a_mix};

    #[test]
    #[ignore = "run with: cargo test --release -p chorus_worldgen --test checksums -- --ignored output_checksum --nocapture"]
    fn output_checksum() {
        bevy_tasks::AsyncComputeTaskPool::get_or_init(bevy_tasks::TaskPool::default);
        let registry = registry();
        let order = [(106, 3), (106, 4), (0, 0), (-3, 2), (105, 3), (107, 4), (200, 100), (-300, 40)];
        let start = Instant::now();
        let dimension = {
            let mut dimension = Dimension::new(DimensionType::End, EndGenerator::new(DIMENSIONS_1, &registry));
            load(&mut dimension, &order);
            dimension
        };
        let mut total = FNV_OFFSET_BASIS;
        for &(chunk_x, chunk_z) in &order {
            let chunk = dimension.get_chunk(chunk_x, chunk_z).expect("generated");
            for x in 0..16u8 {
                for z in 0..16u8 {
                    for y in 0..HEIGHT {
                        let block = chunk.get_block(x, y, z, 0).expect("inside the height range");
                        total = fnv1a_mix(total, block as u32 as u64);
                    }
                }
            }
        }
        println!("end checksum {total:016x} over {} chunks in {:?}", order.len(), start.elapsed());
        assert_eq!(total, 0x8e9f39b592c8b0bd);
    }
}

mod nether {
    use crate::common::nether::settled;
    use crate::common::registry;
    const HEIGHT: i32 = chorus_level::dimension_definition::DimensionDefinition::NETHER.height;
    use std::time::Instant;

    use chorus_worldgen::shared::hash::{FNV_OFFSET_BASIS, fnv1a_mix};

    #[test]
    #[ignore = "run with: cargo test --release -p chorus_worldgen --test checksums -- --ignored output_checksum --nocapture"]
    fn output_checksum() {
        let registry = registry();
        let order = [(9, -10), (30, -8), (0, -13), (10, -8), (-4, -4), (0, 0), (-40, -4), (-12, -28), (12, 12), (-25, 18)];
        let start = Instant::now();
        let dimension = settled(&registry, &order);
        let mut total = FNV_OFFSET_BASIS;
        for &(chunk_x, chunk_z) in &order {
            let chunk = dimension.get_chunk(chunk_x, chunk_z).expect("generated");
            for x in 0..16u8 {
                for z in 0..16u8 {
                    for y in 0..HEIGHT {
                        let block = chunk.get_block(x, y, z, 0).expect("inside the height range");
                        total = fnv1a_mix(total, block as u32 as u64);
                    }
                }
            }
        }
        println!("nether checksum {total:016x} over {} chunks in {:?}", order.len(), start.elapsed());
        assert_eq!(total, 0xed4e9c724bd435b3);
    }
}
