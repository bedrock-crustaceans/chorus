use super::site::{is_air, put};
use crate::bedrock::overworld::decoration::single_block::may_place;
use crate::shared::blocks::{BlockId, BlockTable, flag, state};
use crate::shared::chunk::region::{Heightmap, Region};
use crate::shared::rng::mersenne::MersenneTwister;
use chorus_block::block_id::{LILAC, PEONY, ROSE_BUSH};
use chorus_block::state::common::UPPER_BLOCK_BIT;
use glam::IVec3;

const PLANTS: [&str; 3] = [LILAC, ROSE_BUSH, PEONY];
const PATCH_ROLL: i32 = 5;
const FLOWER_FOREST_PATCH_INSET: i32 = 1;
const PATCH_INSET: i32 = 3;
const PLANT_BOUND: i32 = 3;
const COLUMN_SPAN: i32 = 16;
const TRIES_PER_PATCH: i32 = 5;
const HEIGHT_LIFT: i32 = 32;
const GATE_REACH: i32 = 7;
const GATE_DROP: i32 = 4;
const ATTEMPTS: i32 = 64;
const SPREAD_MASKS: [u32; 3] = [7, 3, 7];

#[derive(Debug)]
struct DoublePlant {
    lower: BlockId,
    upper: BlockId,
}

impl DoublePlant {
    fn new(table: &mut BlockTable, name: &'static str) -> Self {
        Self {
            lower: table.get(name, &[state(&UPPER_BLOCK_BIT, flag(false))]),
            upper: table.get(name, &[state(&UPPER_BLOCK_BIT, flag(true))]),
        }
    }
}

#[derive(Debug)]
pub struct ForestFoliage {
    flower_forest: bool,
    plants: [DoublePlant; 3],
}

fn spread(random: &mut MersenneTwister, mask: u32) -> i32 {
    let first = (random.next_u32() & mask) as i32;
    first - (random.next_u32() & mask) as i32
}

impl ForestFoliage {
    pub fn new(table: &mut BlockTable, flower_forest: bool) -> Self {
        Self {
            flower_forest,
            plants: PLANTS.map(|name| DoublePlant::new(table, name)),
        }
    }

    pub fn place(&self, region: &mut Region, random: &mut MersenneTwister, at: IVec3) -> bool {
        let inset = if self.flower_forest { FLOWER_FOREST_PATCH_INSET } else { PATCH_INSET };
        let patches = random.next_int_bounded(PATCH_ROLL) - inset;
        for _ in 0..patches {
            let plant = &self.plants[random.next_int_bounded(PLANT_BOUND) as usize];
            for _ in 0..TRIES_PER_PATCH {
                let dx = random.next_int_bounded(COLUMN_SPAN);
                let dz = random.next_int_bounded(COLUMN_SPAN);
                let (x, z) = (at.x + dx, at.z + dz);
                let height = region.height(Heightmap::WorldSurfaceWg, x, z);
                if height + HEIGHT_LIFT <= 0 {
                    continue;
                }
                let y = random.next_int_bounded(height + HEIGHT_LIFT);
                self.double_plant(region, random, IVec3::new(x, y, z), plant);
            }
        }
        true
    }

    fn double_plant(&self, region: &mut Region, random: &mut MersenneTwister, at: IVec3, plant: &DoublePlant) {
        let highest = [(-GATE_REACH, -GATE_REACH), (GATE_REACH, -GATE_REACH), (-GATE_REACH, GATE_REACH), (GATE_REACH, GATE_REACH)]
            .into_iter()
            .filter_map(|(dx, dz)| region.highest_block(at.x + dx, at.z + dz))
            .max();
        if highest.is_some_and(|top| top < at.y - GATE_DROP) {
            return;
        }
        for _ in 0..ATTEMPTS {
            let dz = spread(random, SPREAD_MASKS[0]);
            let dy = spread(random, SPREAD_MASKS[1]);
            let dx = spread(random, SPREAD_MASKS[2]);
            let pos = at + IVec3::new(dx, dy, dz);
            if is_air(region, pos) && may_place(region, pos, plant.lower) {
                put(region, pos, plant.lower);
                put(region, pos + IVec3::Y, plant.upper);
            }
        }
    }
}
