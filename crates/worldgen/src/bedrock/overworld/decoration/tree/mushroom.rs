use super::list::BlockList;
use super::site::{block, put};
use crate::bedrock::overworld::decoration::world::biome_at;
use crate::shared::biome::Biome;
use crate::shared::blocks::{BlockId, BlockTable, int, state};
use crate::shared::chunk::region::Region;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::tags::Tag;
use chorus_block::block_id::*;
use chorus_block::state::common::HUGE_MUSHROOM_BITS;
use glam::IVec3;

pub const FEATURE: &str = "minecraft:huge_mushroom_feature";
const SOILS: [&str; 5] = [DIRT, COARSE_DIRT, MYCELIUM, PODZOL, GRASS_BLOCK];
const SMALL_MUSHROOMS: [&str; 2] = [BROWN_MUSHROOM, RED_MUSHROOM];
const CAP_VARIANTS: i32 = 9;
const STEM_BITS: i32 = 10;
const BASE_HEIGHT_BOUND: i32 = 3;
const BASE_HEIGHT_MIN: i32 = 4;
const DOUBLE_HEIGHT_BOUND: i32 = 12;
const WIDE_RADIUS: i32 = 3;
const RED_LAYERS: i32 = 3;
const SWAMP_SINK_LIMIT: i32 = 2;

#[derive(Debug)]
pub struct HugeMushroom {
    brown: [BlockId; 9],
    red: [BlockId; 9],
    stem: BlockId,
    soils: BlockList,
    small: BlockList,
}

impl HugeMushroom {
    pub fn new(table: &mut BlockTable) -> Self {
        let caps = |table: &mut BlockTable, name: &'static str| std::array::from_fn(|index| table.get(name, &[state(&HUGE_MUSHROOM_BITS, int(index as i32 + 1))]));
        Self {
            brown: caps(table, BROWN_MUSHROOM_BLOCK),
            red: caps(table, RED_MUSHROOM_BLOCK),
            stem: table.get(MUSHROOM_STEM, &[state(&HUGE_MUSHROOM_BITS, int(STEM_BITS))]),
            soils: BlockList::from_names(table, &SOILS),
            small: BlockList::from_names(table, &SMALL_MUSHROOMS),
        }
    }

    fn solid_blocking(region: &Region, pos: IVec3) -> bool {
        let found = block(region, pos);
        region.blocks.entry(found).solid && !region.blocks.is(found, Tag::Leaves)
    }

    pub fn place(&self, region: &mut Region, random: &mut MersenneTwister, origin: IVec3) -> bool {
        let red = random.next_int_bounded(2) == 1;
        let base = random.next_int_bounded(BASE_HEIGHT_BOUND) + BASE_HEIGHT_MIN;
        let height = base << i32::from(random.next_int_bounded(DOUBLE_HEIGHT_BOUND) == 0);
        if origin.y <= region.min_y() || origin.y + height + 1 > region.max_y() {
            return false;
        }
        for y in origin.y..origin.y + height + 2 {
            let radius = if y > origin.y + WIDE_RADIUS { WIDE_RADIUS } else { 0 };
            for x in origin.x - radius..=origin.x + radius {
                for z in origin.z - radius..=origin.z + radius {
                    let pos = IVec3::new(x, y, z);
                    let found = block(region, pos);
                    if region.blocks.is_air(found) || region.blocks.is(found, Tag::Leaves) {
                        continue;
                    }
                    if !(pos == origin && self.small.contains(region.blocks, found)) {
                        return false;
                    }
                }
            }
        }
        let top = origin.y + height;
        let swamp = biome_at(region, origin) == Biome::SWAMP;
        let soil = |region: &Region, depth: i32| self.soils.contains(region.blocks, block(region, origin - IVec3::Y * depth));
        let water = |region: &Region, depth: i32| region.blocks.entry(block(region, origin - IVec3::Y * depth)).name.as_ref() == WATER;
        let sink = if soil(region, 1) {
            0
        } else if swamp && water(region, 1) && soil(region, 2) {
            1
        } else if swamp && water(region, 1) && water(region, 2) && soil(region, SWAMP_SINK_LIMIT + 1) {
            SWAMP_SINK_LIMIT
        } else {
            return false;
        };
        let base_y = origin.y - sink;
        let caps = if red { &self.red } else { &self.brown };
        for y in (if red { top - RED_LAYERS } else { top })..=top {
            let radius = if !red {
                WIDE_RADIUS
            } else if y < top {
                2
            } else {
                1
            };
            let restricted = !red || y < top;
            let (x0, x1, z0, z1) = (origin.x - radius, origin.x + radius, origin.z - radius, origin.z + radius);
            for x in x0..=x1 {
                for z in z0..=z1 {
                    let mut bits = 5 + i32::from(x == x1) - i32::from(x == x0) + 3 * (i32::from(z == z1) - i32::from(z == z0));
                    if restricted {
                        if (x == x0 || x == x1) && (z == z0 || z == z1) {
                            continue;
                        }
                        for (bx, bz, value) in [
                            (x0 + 1, z0, 1),
                            (x0, z0 + 1, 1),
                            (x1 - 1, z0, 3),
                            (x1, z0 + 1, 3),
                            (x0 + 1, z1, 7),
                            (x0, z1 - 1, 7),
                            (x1 - 1, z1, 9),
                            (x1, z1 - 1, 9),
                        ] {
                            if x == bx && z == bz {
                                bits = value;
                            }
                        }
                    }
                    if bits == 5 && y < top {
                        continue;
                    }
                    let pos = IVec3::new(x, y, z);
                    if !Self::solid_blocking(region, pos) {
                        put(region, pos, caps[(bits - 1).clamp(0, CAP_VARIANTS - 1) as usize]);
                    }
                }
            }
        }
        for y in base_y..top {
            let pos = IVec3::new(origin.x, y, origin.z);
            if !Self::solid_blocking(region, pos) {
                put(region, pos, self.stem);
            }
        }
        true
    }
}
