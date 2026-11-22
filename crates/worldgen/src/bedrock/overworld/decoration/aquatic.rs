use super::ore::Draw;
use super::world::{biome_at, read};
use crate::bedrock::overworld::surface::temperature_at;
use crate::shared::blocks::{BlockId, BlockTable, int, state, text};
use crate::shared::chunk::region::Region;
use crate::shared::tags::Tag;
use chorus_block::block_id::{KELP, MUD, SEAGRASS, WATER};
use chorus_block::state::common::{KELP_AGE, MINECRAFT_VERTICAL_HALF, SEA_GRASS_TYPE, UPSIDE_DOWN_BIT};
use glam::IVec3;
use serde_json::Value;

const KELP_AGE_COUNT: i32 = 26;
const KELP_MIN_DEPTH: i32 = 3;
const COLUMN_MASK: u32 = 7;
const COLD_TEMPERATURE: f32 = 0.15;
const SEAGRASS_BOTTOM: &str = "double_bot";
const SEAGRASS_TOP: &str = "double_top";
const DEFAULT_TRIES: i32 = 1;

pub(super) fn tries(body: &Value) -> i32 {
    body.get("tries").and_then(Value::as_i64).map_or(DEFAULT_TRIES, |tries| tries as i32)
}

pub fn is_water(region: &Region, pos: IVec3) -> bool {
    region.blocks.name_of(read(region, pos)) == WATER
}

pub fn is_solid_blocking(region: &Region, pos: IVec3) -> bool {
    let block = read(region, pos);
    let entry = region.blocks.entry(block);
    entry.solid && !region.blocks.is(block, Tag::Leaves) && (!region.is_partial_template(block) || region.blocks.name_of(block) == MUD)
}

fn is_air_or_water(region: &Region, pos: IVec3) -> bool {
    region.blocks.is_air(read(region, pos)) || is_water(region, pos)
}

fn supports_seagrass(region: &Region, block: BlockId, pos: IVec3) -> bool {
    let below = read(region, pos - IVec3::Y);
    let entry = region.blocks.entry(below);
    entry.top_centre && !region.blocks.denied_on(block, below) && !region.blocks.name_of(below).ends_with("_fence")
}

fn turned(region: &Region, block: BlockId) -> bool {
    region.blocks.flag(block, &UPSIDE_DOWN_BIT) || region.blocks.text(block, &MINECRAFT_VERTICAL_HALF) == Some("top")
}

fn supports_kelp(region: &Region, block: BlockId, pos: IVec3) -> bool {
    let below = read(region, pos - IVec3::Y);
    let entry = region.blocks.entry(below);
    let holds = entry.cube || entry.raised_top || region.blocks.name_of(below) == KELP || turned(region, below);
    holds && !region.blocks.denied_on(block, below)
}

fn climb(region: &Region, mut pos: IVec3) -> IVec3 {
    while pos.y <= region.max_y() && !is_air_or_water(region, pos) {
        pos.y += 1;
    }
    pos
}

fn next_column<D: Draw>(random: &mut D, base: IVec3) -> IVec3 {
    let mut offset = || (random.word() & COLUMN_MASK) as i32 - (random.word() & COLUMN_MASK) as i32;
    let dz = offset();
    let dx = offset();
    IVec3::new(base.x + dx, base.y, base.z + dz)
}

#[derive(Debug)]
pub struct Kelp {
    count: i32,
    ages: Vec<BlockId>,
}

impl Kelp {
    pub fn parse(table: &mut BlockTable, body: &Value) -> Self {
        let ages = (0..KELP_AGE_COUNT).map(|age| table.get_flooded(KELP, &[state(&KELP_AGE, int(age))])).collect();
        Self { count: tries(body), ages }
    }

    fn holds_kelp(&self, region: &Region, pos: IVec3) -> bool {
        region.blocks.name_of(read(region, pos)) == KELP
    }

    pub fn place<D: Draw>(&self, region: &mut Region, random: &mut D, origin: IVec3) -> bool {
        let mut floor = origin;
        while !is_solid_blocking(region, floor - IVec3::Y) && !self.holds_kelp(region, floor) && floor.y > region.min_y() {
            floor.y -= 1;
        }
        if !is_solid_blocking(region, floor - IVec3::Y) {
            return false;
        }
        let mut column = floor;
        for _ in 0..self.count {
            let pos = climb(region, column);
            if region.blocks.is_air(read(region, pos)) || is_water(region, pos - IVec3::Y) {
                continue;
            }
            let depth = (0..KELP_AGE_COUNT).take_while(|&height| is_water(region, pos + IVec3::Y * height)).count() as i32;
            if depth < KELP_MIN_DEPTH {
                continue;
            }
            let length = if depth > KELP_MIN_DEPTH {
                random.below(depth - KELP_MIN_DEPTH) + KELP_MIN_DEPTH
            } else {
                KELP_MIN_DEPTH
            };
            let age = KELP_AGE_COUNT - length - 1;
            if !self.holds_kelp(region, pos - IVec3::Y) && supports_kelp(region, self.ages[0], pos) {
                region.set(pos, self.ages[age as usize]);
                for step in 1..length {
                    let above = pos + IVec3::Y * step;
                    if !is_water(region, above) {
                        break;
                    }
                    region.set(above, self.ages[(age + step) as usize]);
                }
            }
            column = next_column(random, floor);
        }
        true
    }
}

#[derive(Debug)]
pub struct Seagrass {
    count: i32,
    tall_chance: f32,
    single: BlockId,
    bottom: BlockId,
    top: BlockId,
}

impl Seagrass {
    pub fn parse(table: &mut BlockTable, body: &Value) -> Option<Self> {
        let tall_chance = body.get("tall_chance")?.as_f64()? as f32;
        let single = table.get_flooded(SEAGRASS, &[]);
        let bottom = table.get_flooded(SEAGRASS, &[state(&SEA_GRASS_TYPE, text(SEAGRASS_BOTTOM))]);
        let top = table.get_flooded(SEAGRASS, &[state(&SEA_GRASS_TYPE, text(SEAGRASS_TOP))]);
        Some(Self {
            count: tries(body),
            tall_chance,
            single,
            bottom,
            top,
        })
    }

    pub fn place<D: Draw>(&self, region: &mut Region, random: &mut D, origin: IVec3) -> bool {
        let cold = temperature_at(&biome_at(region, origin), origin.x, origin.y, origin.z, region.sea_level()).is_some_and(|temperature| temperature < COLD_TEMPERATURE);
        let blocking = |region: &Region, pos: IVec3| {
            let block = read(region, pos);
            is_solid_blocking(region, pos) && !region.blocks.entry(block).transparent
        };
        let mut floor = origin;
        while !blocking(region, floor) && floor.y > region.min_y() {
            floor.y -= 1;
        }
        if !blocking(region, floor) {
            return false;
        }
        let mut column = floor;
        for _ in 0..self.count {
            let pos = climb(region, column);
            let waters = [0, 1, 2].map(|height| is_water(region, pos + IVec3::Y * height));
            if waters[0] && supports_seagrass(region, self.single, pos) {
                if self.tall_chance > random.float() {
                    if waters[1] && (!cold || waters[2]) {
                        region.set(pos, self.bottom);
                        region.set(pos + IVec3::Y, self.top);
                    }
                } else if !cold || waters[1] {
                    region.set(pos, self.single);
                }
            }
            column = next_column(random, floor);
        }
        true
    }
}
