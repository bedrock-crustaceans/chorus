mod common;

use chorus_worldgen::bedrock::overworld::OverworldGenerator;
use common::seeds::GENERAL_1;

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
