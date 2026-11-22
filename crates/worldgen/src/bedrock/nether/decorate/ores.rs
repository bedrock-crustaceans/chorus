use super::super::NetherBlocks;
use super::view::View;
use super::{CHUNK_WIDTH, FACES};
use crate::bedrock::overworld::ore::{Ore, ReplaceRule};
use crate::shared::blocks::{AIR, BlockId, BlockTable};
use crate::shared::rng::mersenne::MersenneTwister;
use chorus_block::block_id::{BLACKSTONE, NETHER_GOLD_ORE, NETHERRACK, QUARTZ_ORE};
use glam::IVec3;

const GOLD_ORE_SIZE: i32 = 10;

const QUARTZ_ORE_SIZE: i32 = 14;

const MAGMA_SIZE: i32 = 28;

const GRAVEL_SIZE: i32 = 33;

const DEBRIS_SMALL: (i32, i32, i32, i32) = (2, 4, 8, 16);

const DEBRIS_LARGE: (i32, i32, i32, i32) = (3, 3, 8, 112);

const DEBRIS_RANGE: i32 = 6;

const DEBRIS_STEPS: [[i32; 3]; 6] = [[0, -1, 0], [0, 1, 0], [0, 0, -1], [0, 0, 1], [-1, 0, 0], [1, 0, 0]];

const SPREAD_MASK: u32 = 7;

const SPREAD_HEIGHT_MASK: u32 = 3;

pub struct LegacyOres {
    pub(super) gold: Ore,
    pub(super) quartz: Ore,
    pub(super) magma: Ore,
    pub(super) gravel: Ore,
    pub(super) blackstone: Ore,
}

impl LegacyOres {
    pub fn new(table: &mut BlockTable) -> Self {
        let netherrack = table.kind(NETHERRACK);
        let blackstone_kind = table.kind(BLACKSTONE);
        let rule = |places: BlockId, may_replace: Vec<u16>| ReplaceRule { block: places, may_replace };
        Self {
            gold: Ore::custom(GOLD_ORE_SIZE, vec![rule(table.name(NETHER_GOLD_ORE), vec![netherrack])]),
            quartz: Ore::custom(QUARTZ_ORE_SIZE, vec![rule(table.name(QUARTZ_ORE), vec![netherrack])]),
            magma: Ore::custom(MAGMA_SIZE, vec![rule(table.name(chorus_block::block_id::MAGMA), vec![netherrack, blackstone_kind])]),
            gravel: Ore::custom(GRAVEL_SIZE, vec![rule(table.name(chorus_block::block_id::GRAVEL), vec![netherrack])]),
            blackstone: Ore::custom(GRAVEL_SIZE, vec![rule(table.name(BLACKSTONE), vec![netherrack])]),
        }
    }
}

pub(super) fn position(random: &mut MersenneTwister, (x0, z0): (i32, i32), (low, span): (i32, i32)) -> [i32; 3] {
    let x = x0 + random.next_int_bounded(CHUNK_WIDTH);
    let y = random.next_int_bounded(span) + low;
    [x, y, z0 + random.next_int_bounded(CHUNK_WIDTH)]
}

pub(super) fn spread(random: &mut MersenneTwister, mask: u32) -> i32 {
    let first = (random.next_u32() & mask) as i32;
    first - (random.next_u32() & mask) as i32
}

pub fn zyx_offset(random: &mut MersenneTwister, [x, y, z]: [i32; 3]) -> [i32; 3] {
    let dz = spread(random, SPREAD_MASK);
    let dy = spread(random, SPREAD_HEIGHT_MASK);
    let dx = spread(random, SPREAD_MASK);
    [x + dx, y + dy, z + dz]
}

pub(super) fn scatter_ore(view: &mut View, random: &mut MersenneTwister, corner: (i32, i32), ore: &Ore, count: i32, range: (i32, i32)) {
    for _ in 0..count {
        let [x, y, z] = position(random, corner, range);
        ore.place(view, random, IVec3::new(x, y, z));
    }
}

pub fn debris(random: &mut MersenneTwister, source: (i32, i32), view: &mut View, ids: &NetherBlocks) {
    let corner = (source.0 * CHUNK_WIDTH, source.1 * CHUNK_WIDTH);
    for (count, max, low, span) in [DEBRIS_SMALL, DEBRIS_LARGE] {
        for _ in 0..count {
            let at = position(random, corner, (low, span));
            debris_vein(view, random, at, max, ids);
        }
    }
}

fn debris_vein(view: &mut View, random: &mut MersenneTwister, origin: [i32; 3], max: i32, ids: &NetherBlocks) {
    let count = random.next_int_bounded(max);
    for index in 0..count {
        let mut p = origin;
        for _ in 0..2 * index {
            let [sx, sy, sz] = DEBRIS_STEPS[random.next_int_bounded(6) as usize];
            p = [p[0] + sx, p[1] + sy, p[2] + sz];
        }
        let [dx, dy, dz] = [p[0] - origin[0], p[1] - origin[1], p[2] - origin[2]];
        if dx * dx + dy * dy + dz * dz > DEBRIS_RANGE * DEBRIS_RANGE {
            continue;
        }
        if view.get(p[0], p[1], p[2]) == ids.netherrack && !FACES.iter().any(|[fx, fy, fz]| view.get(p[0] + fx, p[1] + fy, p[2] + fz) == AIR) {
            view.set(p[0], p[1], p[2], ids.ancient_debris);
        }
    }
}
