mod common;

use chorus_worldgen::OverworldGenerator;

const BDS_SPAWNS: [(i64, (i32, i32)); 12] = [
    (-123456789012, (48, 113)),
    (-1, (498, 387)),
    (0, (-31, 6)),
    (1, (163, 170)),
    (2, (-31, 6)),
    (3, (0, 0)),
    (42, (-31, 6)),
    (100, (0, 0)),
    (777, (0, 0)),
    (1234567, (25, -31)),
    (9876543210, (0, 0)),
    (4802433367836389877, (-75, 127)),
];

#[test]
fn the_spawn_column_matches_the_bedrock_dedicated_server() {
    let registry = common::registry();
    let wrong: Vec<_> = BDS_SPAWNS
        .iter()
        .filter_map(|&(seed, expected)| {
            let found = OverworldGenerator::new(seed, &registry).expect("the generator builds").spawn_column();
            (found != expected).then_some((seed, expected, found))
        })
        .collect();
    assert!(wrong.is_empty(), "seed, bds, ours: {wrong:?}");
}
