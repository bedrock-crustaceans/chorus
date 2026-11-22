mod common;

use common::seeds::GENERAL_1;
use common::{fingerprints, generate, positions, registry};
use std::collections::HashMap;

const ORE_NAMES: [&str; 18] = [
    "minecraft:coal_ore",
    "minecraft:deepslate_coal_ore",
    "minecraft:iron_ore",
    "minecraft:deepslate_iron_ore",
    "minecraft:copper_ore",
    "minecraft:deepslate_copper_ore",
    "minecraft:gold_ore",
    "minecraft:deepslate_gold_ore",
    "minecraft:redstone_ore",
    "minecraft:deepslate_redstone_ore",
    "minecraft:lapis_ore",
    "minecraft:deepslate_lapis_ore",
    "minecraft:diamond_ore",
    "minecraft:deepslate_diamond_ore",
    "minecraft:emerald_ore",
    "minecraft:tuff",
    "minecraft:gravel",
    "minecraft:granite",
];

fn ore_counts(seed: i64, order: &[(i32, i32)]) -> HashMap<(i32, i32), usize> {
    let registry = registry();
    let dimension = generate(seed, order, &registry);
    let ores: Vec<i32> = ORE_NAMES.iter().filter_map(|name| registry.get_block_id(name)).collect();
    order
        .iter()
        .map(|&(x, z)| {
            let chunk = dimension.get_chunk(x, z).expect("generated");
            let count = (0..16u8)
                .flat_map(|bx| (0..16u8).flat_map(move |bz| (-64..320).map(move |y| (bx, y, bz))))
                .filter(|&(bx, y, bz)| chunk.get_block(bx, y, bz, 0).is_some_and(|block| ores.contains(&block)))
                .count();
            ((x, z), count)
        })
        .collect()
}

#[test]
#[ignore = "run with: cargo test --release -p chorus_worldgen --test determinism -- --ignored"]
fn chunks_do_not_depend_on_request_order() {
    let forward = positions();
    let mut reversed = forward.clone();
    reversed.reverse();
    let first = fingerprints(GENERAL_1, &forward);
    let second = fingerprints(GENERAL_1, &reversed);
    let different: Vec<_> = forward.iter().filter(|cell| first[cell] != second[cell]).collect();
    assert!(
        different.is_empty(),
        "{} of {} chunks depend on request order, for example {:?}",
        different.len(),
        forward.len(),
        &different[..different.len().min(5)]
    );
}

#[test]
#[ignore = "run with: cargo test --release -p chorus_worldgen --test determinism -- --ignored"]
fn decoration_does_not_depend_on_request_order() {
    let forward = positions();
    let mut reversed = forward.clone();
    reversed.reverse();
    let first = ore_counts(GENERAL_1, &forward);
    let again = ore_counts(GENERAL_1, &forward);
    let second = ore_counts(GENERAL_1, &reversed);
    let total: usize = first.values().sum();
    assert!(total > 0, "no ore was decorated");
    let different: Vec<_> = forward.iter().filter(|cell| first[cell] != again[cell] || first[cell] != second[cell]).collect();
    assert!(
        different.is_empty(),
        "{} of {} chunks decorate differently, for example {:?}",
        different.len(),
        forward.len(),
        &different[..different.len().min(5)]
    );
}
