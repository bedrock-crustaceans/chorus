use super::list::BlockList;
use super::site::{block, ground, put};
use crate::shared::CHUNK_WIDTH;
use crate::shared::blocks::{BlockId, BlockTable};
use crate::shared::chunk::region::Region;
use crate::shared::rng::mersenne::MersenneTwister;
use chorus_block::block_id::{COARSE_DIRT, DIRT, GRASS_BLOCK, MOSSY_COBBLESTONE, STONE};
use glam::IVec3;

pub const FEATURE: &str = "minecraft:legacy:forest_rock_feature";
const GROUND: [&str; 4] = [GRASS_BLOCK, STONE, DIRT, COARSE_DIRT];
const MAX_ROCKS: i32 = 3;
const BLOB_COUNT: i32 = 3;
const BLOB_AXIS_BOUND: i32 = 2;
const RADIUS_STEP: f32 = 0.333;
const RADIUS_BASE: f32 = 0.5;
const FLOOR_MARGIN: i32 = 3;

#[derive(Debug)]
pub struct ForestRock {
    ground: BlockList,
    moss: BlockId,
}

impl ForestRock {
    pub fn new(table: &mut BlockTable) -> Self {
        Self {
            ground: BlockList::from_names(table, &GROUND),
            moss: table.name(MOSSY_COBBLESTONE),
        }
    }

    pub fn place(&self, region: &mut Region, random: &mut MersenneTwister, at: IVec3) -> bool {
        for _ in 0..random.next_int_bounded(MAX_ROCKS) {
            let dz = random.next_int_bounded(CHUNK_WIDTH);
            let dx = random.next_int_bounded(CHUNK_WIDTH);
            let (x, z) = (at.x + dx, at.z + dz);
            let y = ground(region, x, z);
            self.blob(region, random, IVec3::new(x, y, z));
        }
        true
    }

    fn blob(&self, region: &mut Region, random: &mut MersenneTwister, mut centre: IVec3) {
        let floor = region.min_y() + FLOOR_MARGIN;
        while centre.y > floor {
            let below = block(region, centre - IVec3::Y);
            if !region.blocks.is_air(below) && self.ground.contains(region.blocks, below) {
                break;
            }
            centre.y -= 1;
        }
        if centre.y <= floor {
            return;
        }
        for _ in 0..BLOB_COUNT {
            let a = random.next_int_bounded(BLOB_AXIS_BOUND);
            let b = random.next_int_bounded(BLOB_AXIS_BOUND);
            let c = random.next_int_bounded(BLOB_AXIS_BOUND);
            let radius = (a + b + c) as f32 * RADIUS_STEP + RADIUS_BASE;
            for x in centre.x - a..=centre.x + a {
                for y in centre.y - b..=centre.y + b {
                    for z in centre.z - c..=centre.z + c {
                        let offset = IVec3::new(x, y, z) - centre;
                        if offset.length_squared() as f32 <= radius * radius {
                            put(region, IVec3::new(x, y, z), self.moss);
                        }
                    }
                }
            }
            centre.x += random.next_int_bounded(BLOB_AXIS_BOUND) - 1;
            centre.y -= random.next_int_bounded(BLOB_AXIS_BOUND);
            centre.z += random.next_int_bounded(BLOB_AXIS_BOUND) - 1;
        }
    }
}
