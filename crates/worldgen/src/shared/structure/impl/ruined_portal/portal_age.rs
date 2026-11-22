use crate::shared::blocks::BlockId;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::bounds::BoundingBox;
use crate::shared::structure::place::Library;
use crate::shared::structure::plan::Plan;
use crate::shared::structure::probe::{Probed, StartProbe};
use chorus_structure_data::StatePoolBuilder;

pub(crate) const FULL: &[&str] = &[
    "stone_bricks",
    "chiseled_stone_bricks",
    "cracked_stone_bricks",
    "mossy_stone_bricks",
    "smooth_stone_double_slab",
    "sandstone_double_slab",
    "petrified_oak_double_slab",
    "cobblestone_double_slab",
    "brick_double_slab",
    "stone_brick_double_slab",
    "quartz_double_slab",
    "nether_brick_double_slab",
    "oak_double_slab",
    "stone",
];
const WALL_ALONE: &str = "wall_connection_type_east=none,wall_connection_type_north=none,wall_connection_type_south=none,wall_connection_type_west=none,wall_post_bit=0";
pub(crate) const SLABS: &[&str] = &[
    "smooth_stone_slab",
    "sandstone_slab",
    "petrified_oak_slab",
    "cobblestone_slab",
    "brick_slab",
    "stone_brick_slab",
    "quartz_slab",
    "nether_brick_slab",
    "oak_slab",
];
const BOTTOM_SLAB: &str = "minecraft:vertical_half=bottom";
const HALF: f32 = 0.5;
const MAGMA_CHANCE: f32 = 0.1;
const OBSIDIAN_CHANCE: f32 = 0.2;
const GOLD_CHANCE: f32 = 0.3;
const LAVA_MAGMA_CHANCE: f32 = 0.2;
pub(crate) const SPREAD_RADIUS: i32 = 14;
pub(crate) const SPREAD: [f32; 14] = [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.9, 0.9, 0.8, 0.7, 0.6, 0.4, 0.2];
pub(crate) const LAVA_NEIGHBOURS: [(i32, i32, i32); 5] = [(-1, 0, 0), (1, 0, 0), (0, 0, -1), (0, 0, 1), (0, 1, 0)];

const PLAIN_BLOCKS: [&str; 9] = [
    "magma",
    "netherrack",
    "crying_obsidian",
    "cracked_stone_bricks",
    "mossy_stone_bricks",
    "polished_blackstone",
    "chiseled_polished_blackstone",
    "polished_blackstone_bricks",
    "cracked_polished_blackstone_bricks",
];
const BOTTOM_SLABS: [&str; 4] = ["normal_stone_slab", "stone_brick_slab", "mossy_stone_brick_slab", "polished_blackstone_brick_slab"];
const STAIRS: [&str; 4] = ["stone_brick_stairs", "mossy_stone_brick_stairs", "polished_blackstone_stairs", "polished_blackstone_brick_stairs"];

pub fn declare_states(pool: &mut StatePoolBuilder) {
    for name in PLAIN_BLOCKS {
        pool.intern(name, "", false);
    }
    pool.intern("iron_chain", "pillar_axis=y", false);
    for name in BOTTOM_SLABS {
        pool.intern(name, BOTTOM_SLAB, false);
    }
    for name in ["mossy_stone_brick_wall", "polished_blackstone_brick_wall"] {
        pool.intern(name, WALL_ALONE, false);
    }
    for name in STAIRS {
        for upside_down in 0..2 {
            for weirdo in 0..4 {
                pool.intern(name, &format!("minecraft:corner=none,upside_down_bit={upside_down},weirdo_direction={weirdo}"), false);
            }
        }
    }
}

fn state_value<'a>(states: &'a str, key: &str) -> &'a str {
    states.split(',').filter_map(|pair| pair.split_once('=')).find(|(k, _)| *k == key).map_or("0", |(_, value)| value)
}

fn stairs(library: &Library, name: &str, upside_down: &str, weirdo: &str) -> BlockId {
    library
        .palette
        .named(name, &format!("minecraft:corner=none,upside_down_bit={upside_down},weirdo_direction={weirdo}"), false)
}

fn random_stairs(library: &Library, random: &mut MersenneTwister, name: &str) -> BlockId {
    let weirdo = random.next_int_bounded(4);
    let upside_down = random.next_float() < HALF;
    stairs(library, name, if upside_down { "1" } else { "0" }, &weirdo.to_string())
}

pub struct Weathering {
    pub mossiness: f32,
    pub cold: bool,
    pub sea_floor: bool,
}

pub fn age(plan: &mut Plan, library: &Library, probe: &dyn StartProbe, random: &mut MersenneTwister, b: &BoundingBox, weathering: Weathering) -> Probed<()> {
    let Weathering { mossiness, cold, sea_floor } = weathering;
    let plain = |name: &str| library.palette.named(name, "", false);
    let slab = |name: &str| library.palette.named(name, BOTTOM_SLAB, false);
    let lava = |plan: &Plan, x: i32, y: i32, z: i32| -> Probed<bool> { Ok(matches!(plan.short_name(plan.world(probe, x, y, z)?), "lava" | "flowing_lava")) };
    for x in b.x0..=b.x1 {
        for y in b.y0..=b.y1 {
            for z in b.z0..=b.z1 {
                let block = plan.world(probe, x, y, z)?;
                let name = plan.short_name(block);
                let states = library.palette.states_of(block).unwrap_or("");
                let aged = if FULL.contains(&name) {
                    if random.next_float() < HALF {
                        continue;
                    }
                    let normal = [plain("cracked_stone_bricks"), random_stairs(library, random, "stone_brick_stairs")];
                    let mossy = [plain("mossy_stone_bricks"), random_stairs(library, random, "mossy_stone_brick_stairs")];
                    let f = random.next_float();
                    let i = random.next_int_bounded(2) as usize;
                    if mossiness <= f { normal[i] } else { mossy[i] }
                } else if name == "stone_brick_stairs" {
                    let (weirdo, upside_down) = (state_value(states, "weirdo_direction"), state_value(states, "upside_down_bit"));
                    if random.next_float() < HALF {
                        continue;
                    }
                    let normal = [slab("normal_stone_slab"), slab("stone_brick_slab")];
                    let mossy = [stairs(library, "mossy_stone_brick_stairs", upside_down, weirdo), slab("mossy_stone_brick_slab")];
                    let f = random.next_float();
                    let i = random.next_int_bounded(2) as usize;
                    if mossiness <= f { normal[i] } else { mossy[i] }
                } else if SLABS.contains(&name) {
                    if random.next_float() >= mossiness {
                        continue;
                    }
                    slab("mossy_stone_brick_slab")
                } else if name.ends_with("_wall") && states.contains("wall_post_bit") {
                    if random.next_float() >= mossiness {
                        continue;
                    }
                    library.palette.named("mossy_stone_brick_wall", WALL_ALONE, false)
                } else if name == "netherrack" {
                    if cold || random.next_float() >= MAGMA_CHANCE {
                        continue;
                    }
                    plain("magma")
                } else if name == "obsidian" {
                    if random.next_float() >= OBSIDIAN_CHANCE {
                        continue;
                    }
                    plain("crying_obsidian")
                } else if name == "lava" || name == "flowing_lava" {
                    match (sea_floor, cold) {
                        (false, false) if random.next_float() < LAVA_MAGMA_CHANCE => plain("magma"),
                        (false, false) => continue,
                        (false, true) => plain("netherrack"),
                        (true, _) => plain("magma"),
                    }
                } else if name == "gold_block" {
                    if random.next_float() >= GOLD_CHANCE {
                        continue;
                    }
                    let mut near_lava = false;
                    for (dx, dy, dz) in LAVA_NEIGHBOURS {
                        if lava(plan, x + dx, y + dy, z + dz)? {
                            near_lava = true;
                            break;
                        }
                    }
                    if near_lava { plan.ids.lava } else { plan.ids.air }
                } else {
                    continue;
                };
                plan.set(x, y, z, aged);
            }
        }
    }
    Ok(())
}
