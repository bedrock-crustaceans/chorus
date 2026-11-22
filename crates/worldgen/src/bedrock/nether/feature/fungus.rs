use super::super::decorate::{View, opaque_solid};
use super::vines::{vine_length, weeping_column};
use super::{buildable, nether_soil};
use crate::bedrock::overworld::ore::Cells;
use crate::shared::blocks::{AIR, BlockId};
use crate::shared::rng::mersenne::MersenneTwister;
use chorus_block::block_id::LAVA;

const FUNGUS_BASE_DEPTH: i32 = 100;

const FUNGUS_HEIGHT_LOW: i32 = 4;

const FUNGUS_HEIGHT_SPAN: i32 = 10;

const FUNGUS_DOUBLE_DIVISOR: i32 = 12;

const FUNGUS_HUGE_BELOW: f64 = 0.06;

const FUNGUS_CORNER_ODDS: f32 = 0.1;

const FUNGUS_HAT_BASE: i32 = 5;

const FUNGUS_HAT_SMALL: i32 = 9;

const FUNGUS_HAT_SMALL_RIM: i32 = 4;

const HAT_DROP_BELOW: i32 = 3;

const HAT_DROP_CHANCE: f32 = 0.15;

const HAT_DROP_VINES: i32 = 11;

const HAT_SHROOMLIGHT: f32 = 0.1;

const HAT_SHROOMLIGHT_ODDS: f32 = 0.2;

const HAT_VINES_CHANCE: i32 = 10;

const HAT_CORNER_BELOW: f32 = 0.7;

const HAT_CORNER_VINES: i32 = 12;

const HAT_EDGE_BELOW: f32 = 0.98;

const HAT_EDGE_VINES: i32 = 14;

const HAT_RARE_SHROOMLIGHT: f32 = 0.02;

const VINE_TOP_AGES: (i32, i32) = (23, 25);

const VINE_LENGTH_BOUND: i32 = 5;

const VINE_LENGTH_ZERO: i32 = 0;

pub(super) fn fungus(view: &mut View, random: &mut MersenneTwister, [x, y, z]: [i32; 3], crimson: bool) -> bool {
    let (stem, hat) = if crimson {
        (view.ids().crimson_stem, view.ids().nether_wart)
    } else {
        (view.ids().warped_stem, view.ids().warped_wart)
    };
    let height = (random.next_int_bounded(FUNGUS_HEIGHT_SPAN) + FUNGUS_HEIGHT_LOW) << u32::from(random.next_int_bounded(FUNGUS_DOUBLE_DIVISOR) == 0);
    if y <= view.min_y() || y + height + 1 > view.max_y() {
        return false;
    }
    let Some(y) = fungus_base(view, [x, y, z]) else { return false };
    let huge = (random.next_float() as f64) < FUNGUS_HUGE_BELOW;
    fungus_stem(view, random, [x, y, z], stem, height, huge);
    fungus_hat(view, random, [x, y, z], crimson, hat, height, huge);
    true
}

fn fungus_base(view: &View, [x, mut y, z]: [i32; 3]) -> Option<i32> {
    for _ in 0..=FUNGUS_BASE_DEPTH {
        let here = view.get(x, y, z);
        let below = view.get(x, y - 1, z);
        if view.blocks().name_of(here) == LAVA || view.blocks().name_of(below) == LAVA {
            return None;
        }
        if nether_soil(view, below) {
            return Some(y);
        }
        y -= 1;
        if y <= view.min_y() {
            return None;
        }
    }
    None
}

fn fungus_stem(view: &mut View, random: &mut MersenneTwister, [x, y, z]: [i32; 3], stem: BlockId, height: i32, huge: bool) {
    let radius = i32::from(huge);
    for dx in -radius..=radius {
        for dz in -radius..=radius {
            let corner = huge && dx.abs() == 1 && dz.abs() == 1;
            for dy in 0..height.max(1) {
                let [cx, cy, cz] = [x + dx, y + dy, z + dz];
                let there = view.get(cx, cy, cz);
                if !(buildable(view, there) || there == view.ids().crimson_fungus || there == view.ids().warped_fungus) {
                    continue;
                }
                if !corner || random.next_float() < FUNGUS_CORNER_ODDS {
                    view.set(cx, cy, cz, stem);
                }
            }
        }
    }
}

fn fungus_hat(view: &mut View, random: &mut MersenneTwister, [x, y, z]: [i32; 3], crimson: bool, hat: BlockId, height: i32, huge: bool) {
    let size = (random.next_int_bounded(height / 3 + 1) + FUNGUS_HAT_BASE).min(height);
    let low = height - size;
    for level in low..=height {
        let roll = random.next_int_bounded(3);
        let radius = if size >= FUNGUS_HAT_SMALL && level < low + FUNGUS_HAT_SMALL_RIM {
            3
        } else if level < height - roll {
            2
        } else {
            1
        };
        let edge = radius + i32::from(huge);
        for dx in -edge..=edge {
            for dz in -edge..=edge {
                let at = [x + dx, y + level, z + dz];
                let there = view.get(at[0], at[1], at[2]);
                if opaque_solid(view.blocks(), there) || !buildable(view, there) {
                    continue;
                }
                let (on_x, on_z, top) = (dx.abs() == edge, dz.abs() == edge, level == height);
                if level < low + HAT_DROP_BELOW {
                    if on_x || on_z || top {
                        hat_drop(view, random, crimson, hat, at);
                    }
                } else {
                    hat_upper(view, random, crimson, hat, at, (on_x || on_z || top, on_x && on_z));
                }
            }
        }
    }
}

fn hat_drop(view: &mut View, random: &mut MersenneTwister, crimson: bool, hat: BlockId, [x, y, z]: [i32; 3]) {
    if view.get(x, y - 1, z) == hat {
        view.set(x, y, z, hat);
    } else if random.next_float() < HAT_DROP_CHANCE {
        hat_vines(view, random, crimson, hat, [x, y, z], HAT_DROP_VINES);
    }
}

fn hat_upper(view: &mut View, random: &mut MersenneTwister, crimson: bool, hat: BlockId, [x, y, z]: [i32; 3], (rim, corner): (bool, bool)) {
    let roll = random.next_float();
    if !rim {
        if roll < HAT_SHROOMLIGHT {
            view.set(x, y, z, view.ids().shroomlight);
        } else if random.next_float() < HAT_SHROOMLIGHT_ODDS {
            hat_vines(view, random, crimson, hat, [x, y, z], HAT_VINES_CHANCE);
        }
        return;
    }
    let (below, chance) = if corner { (HAT_CORNER_BELOW, HAT_CORNER_VINES) } else { (HAT_EDGE_BELOW, HAT_EDGE_VINES) };
    if roll < below {
        hat_vines(view, random, crimson, hat, [x, y, z], chance);
    } else if random.next_float() < HAT_RARE_SHROOMLIGHT {
        view.set(x, y, z, view.ids().shroomlight);
    }
}

fn hat_vines(view: &mut View, random: &mut MersenneTwister, crimson: bool, hat: BlockId, [x, y, z]: [i32; 3], chance: i32) {
    view.set(x, y, z, hat);
    if crimson && random.next_int_bounded(chance) == 0 && view.get(x, y - 1, z) == AIR {
        let length = vine_length(random, VINE_LENGTH_BOUND, VINE_LENGTH_ZERO);
        weeping_column(view, random, [x, y - 1, z], length, VINE_TOP_AGES);
    }
}
