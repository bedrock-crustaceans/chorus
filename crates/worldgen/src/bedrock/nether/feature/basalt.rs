use super::super::decorate::{View, opaque_solid, solid};
use crate::bedrock::overworld::ore::Cells;
use crate::shared::blocks::AIR;
use crate::shared::rng::mersenne::MersenneTwister;
use chorus_block::block_id::LAVA;

const PILLAR_SCALE: f32 = f32::from_bits(0x3da3_d708);

const PILLAR_BASE: f32 = f32::from_bits(0x3f5e_b852);

const PILLAR_SIDES: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

const PILLAR_SPREAD: i32 = 3;

const PILLAR_SPREAD_ODDS: i32 = 10;

const PILLAR_DROP_DEPTH: i32 = 3;

const COLUMN_BIG_ODDS: f32 = 0.3;

const COLUMN_BIG_HEIGHT: (i32, i32) = (5, 6);

const COLUMN_SMALL_HEIGHT: (i32, i32) = (1, 4);

const COLUMN_WIDE_ODDS: f32 = 0.9;

const COLUMN_WIDE: (i32, i32, i32) = (50, 11, -5);

const COLUMN_NARROW: (i32, i32, i32) = (15, 17, -8);

const COLUMN_ODD: i32 = 1;

const DELTA_SURFACE_ODDS: f32 = 0.9;

const DELTA_RIM: i32 = 3;

const DELTA_RADIUS: (i32, i32) = (3, 4);

const DELTA_CLOSED: [[i32; 3]; 5] = [[0, -1, 0], [0, 0, -1], [0, 0, 1], [1, 0, 0], [-1, 0, 0]];

pub(super) fn columns(view: &mut View, random: &mut MersenneTwister, [x, mut y, z]: [i32; 3]) -> bool {
    let surface = loop {
        if y <= view.min_y() || y >= (view.max_y() + 1) {
            return false;
        }
        if !solid(view.blocks(), view.get(x, y, z)) && opaque_solid(view.blocks(), view.get(x, y - 1, z)) {
            break y;
        }
        y -= 1;
    };
    let big = random.next_float() >= COLUMN_BIG_ODDS;
    let reach = i32::from(big) + COLUMN_ODD;
    let height = if big {
        random.next_int_bounded(COLUMN_BIG_HEIGHT.1) + COLUMN_BIG_HEIGHT.0
    } else {
        random.next_int_bounded(COLUMN_SMALL_HEIGHT.1) + COLUMN_SMALL_HEIGHT.0
    };
    let (tries, range, offset) = if random.next_float() < COLUMN_WIDE_ODDS { COLUMN_WIDE } else { COLUMN_NARROW };
    for _ in 0..tries {
        let px = x + offset + random.next_int_bounded(range);
        let pz = z + offset + random.next_int_bounded(range);
        let left = height - (px - x).abs() - (pz - z).abs();
        if left <= 0 {
            continue;
        }
        let r = random.next_int_bounded(reach) + reach;
        'columns: for cx in px - r..=px + r {
            for cz in pz - r..=pz + r {
                let steps = (cx - px).abs() + (cz - pz).abs();
                if let Some(base) = column_base(view, [cx, surface, cz], steps) {
                    grow_column(view, [cx, base, cz], left - steps / 2);
                    break 'columns;
                }
            }
        }
    }
    true
}

fn column_base(view: &View, [x, mut y, z]: [i32; 3], mut steps: i32) -> Option<i32> {
    let ids = view.ids();
    let step = if solid(view.blocks(), view.get(x, y, z)) { 1 } else { -1 };
    while y > view.min_y() {
        if y >= (view.max_y() + 1) || steps < 1 {
            return None;
        }
        let here = view.get(x, y, z);
        let below = view.get(x, y - 1, z);
        if !solid(view.blocks(), here) && opaque_solid(view.blocks(), below) {
            return (view.blocks().same_kind(below, ids.basalt) || view.blocks().same_kind(below, ids.blackstone)).then_some(y);
        }
        y += step;
        steps -= 1;
    }
    None
}

fn grow_column(view: &mut View, [x, mut y, z]: [i32; 3], mut count: i32) {
    let (bedrock, basalt) = (view.ids().bedrock, view.ids().basalt);
    while view.get(x, y, z) != bedrock && y < (view.max_y() + 1) && count >= 0 {
        view.set(x, y, z, basalt);
        y += 1;
        count -= 1;
    }
}

pub(super) fn pillar(view: &mut View, random: &mut MersenneTwister, [x, mut y, z]: [i32; 3]) -> bool {
    let ids = view.ids();
    if view.get(x, y, z) != AIR {
        return false;
    }
    let above = view.get(x, y + 1, z);
    if !(above == ids.soul_sand || above == ids.soul_soil || above == ids.netherrack) {
        return false;
    }
    let chances: [f32; 4] = std::array::from_fn(|_| random.next_float() * PILLAR_SCALE + PILLAR_BASE);
    loop {
        random.next_float();
        if view.get(x, y, z) != AIR || view.get(x, y + 1, z) == AIR {
            break;
        }
        view.set(x, y, z, ids.basalt);
        for (&(dx, dz), &chance) in PILLAR_SIDES.iter().zip(&chances) {
            if random.next_float() < chance && view.get(x + dx, y, z + dz) == AIR && view.get(x + dx, y + 1, z + dz) != AIR {
                view.set(x + dx, y, z + dz, ids.basalt);
            }
        }
        y -= 1;
    }
    for (dx, dz) in PILLAR_SIDES {
        if view.get(x + dx, y, z + dz) != AIR {
            view.set(x + dx, y + 1, z + dz, ids.basalt);
        }
    }
    for dx in -PILLAR_SPREAD..=PILLAR_SPREAD {
        for dz in -PILLAR_SPREAD..=PILLAR_SPREAD {
            if random.next_int_bounded(PILLAR_SPREAD_ODDS) < (dx * dz).abs() {
                continue;
            }
            let (qx, qz) = (x + dx, z + dz);
            if let Some(drop) = (0..PILLAR_DROP_DEPTH).find(|drop| view.get(qx, y - 1 - drop, qz) != AIR) {
                view.set(qx, y - drop, qz, ids.basalt);
            }
        }
    }
    true
}

pub(super) fn delta(view: &mut View, random: &mut MersenneTwister, [x, mut y, z]: [i32; 3]) -> bool {
    let ids = view.ids();
    let step = if solid(view.blocks(), view.get(x, y, z)) { 1 } else { -1 };
    loop {
        if !(2..(view.max_y() + 1)).contains(&y) {
            return false;
        }
        let here = view.get(x, y, z);
        let below = view.get(x, y - 1, z);
        if here == AIR && below != ids.bedrock && below != AIR && view.blocks().name_of(below) != LAVA {
            break;
        }
        y += step;
    }
    let rim = random.next_float() < DELTA_SURFACE_ODDS;
    let (ox, oz) = if rim { (random.next_int_bounded(DELTA_RIM), random.next_int_bounded(DELTA_RIM)) } else { (0, 0) };
    let rx = random.next_int_bounded(DELTA_RADIUS.1) + DELTA_RADIUS.0;
    let rz = random.next_int_bounded(DELTA_RADIUS.1) + DELTA_RADIUS.0;
    let reach = rx.max(rz);
    let rim = rim && ox != 0 && oz != 0;
    for dx in -rx..=rx {
        for dz in -rz..=rz {
            if dx.abs() + dz.abs() > reach {
                continue;
            }
            let cell = [x + dx, y, z + dz];
            if !delta_cell(view, cell) {
                continue;
            }
            if rim {
                view.set(cell[0], cell[1], cell[2], ids.magma);
            }
            let molten = [cell[0] + ox, y, cell[2] + oz];
            if delta_cell(view, molten) {
                view.set(molten[0], molten[1], molten[2], ids.lava);
            }
        }
    }
    true
}

fn delta_cell(view: &View, [x, y, z]: [i32; 3]) -> bool {
    if view.blocks().name_of(view.get(x, y, z)) == LAVA || view.get(x, y + 1, z) != AIR {
        return false;
    }
    DELTA_CLOSED.iter().all(|&[dx, dy, dz]| view.get(x + dx, y + dy, z + dz) != AIR)
}
