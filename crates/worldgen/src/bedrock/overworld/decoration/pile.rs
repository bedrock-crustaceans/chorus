use super::ore::resolve_block;
use super::world::{is_air, read};
use crate::shared::blocks::{BlockId, BlockTable};
use crate::shared::chunk::region::Region;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::tags::Tag;
use chorus_block::block_id::{GRASS_PATH, SNOW_LAYER};
use glam::IVec3;
use serde_json::Value;

const DEFAULT_RADIUS_BASE: i32 = 2;
const DEFAULT_RADIUS_CHOICES: i32 = 2;
const DEFAULT_LAYERS: i32 = 2;
const DEFAULT_NEAR_SCALE: f32 = 10.0;
const DEFAULT_FAR_SCALE: f32 = 6.0;
const DEFAULT_STRAY_CHANCE: f32 = 0.031;
const DEFAULT_MIN_HEIGHT_ABOVE_FLOOR: i32 = 5;

#[derive(Debug)]
enum Chance {
    OneIn(i32),
    AtLeast(f32),
}

#[derive(Debug)]
struct Alternate {
    block: BlockId,
    chance: Chance,
}

#[derive(Debug)]
pub struct BlockPile {
    block: BlockId,
    alternate: Option<Alternate>,
    radius_base: i32,
    radius_choices: i32,
    layers: i32,
    near_scale: f32,
    far_scale: f32,
    stray_chance: f32,
    min_height_above_floor: i32,
    path: u16,
    snow_layer: u16,
}

fn number(body: &Value, key: &str, default: f64) -> f64 {
    body.get(key).and_then(Value::as_f64).unwrap_or(default)
}

impl BlockPile {
    pub fn parse(table: &mut BlockTable, body: &'static Value) -> Option<Self> {
        let block = resolve_block(table, body.get("block")?)?;
        let alternate = match body.get("alternate") {
            Some(alternate) => {
                let chance = match (alternate.get("one_in").and_then(Value::as_i64), alternate.get("at_least").and_then(Value::as_f64)) {
                    (Some(count), None) if count > 0 => Chance::OneIn(count as i32),
                    (None, Some(threshold)) => Chance::AtLeast(threshold as f32),
                    _ => return None,
                };
                Some(Alternate {
                    block: resolve_block(table, alternate.get("block")?)?,
                    chance,
                })
            }
            None => None,
        };
        Some(Self {
            block,
            alternate,
            radius_base: number(body, "radius_base", DEFAULT_RADIUS_BASE as f64) as i32,
            radius_choices: number(body, "radius_choices", DEFAULT_RADIUS_CHOICES as f64) as i32,
            layers: number(body, "layers", DEFAULT_LAYERS as f64) as i32,
            near_scale: number(body, "near_scale", DEFAULT_NEAR_SCALE as f64) as f32,
            far_scale: number(body, "far_scale", DEFAULT_FAR_SCALE as f64) as f32,
            stray_chance: number(body, "stray_chance", DEFAULT_STRAY_CHANCE as f64) as f32,
            min_height_above_floor: number(body, "min_height_above_floor", DEFAULT_MIN_HEIGHT_ABOVE_FLOOR as f64) as i32,
            path: table.kind(GRASS_PATH),
            snow_layer: table.kind(SNOW_LAYER),
        })
    }

    fn supports(&self, region: &Region, pos: IVec3) -> bool {
        let below = read(region, pos);
        let entry = region.blocks.entry(below);
        region.blocks.is_kind(below, self.path) || (entry.cube && !region.blocks.is(below, Tag::Leaves) && !region.blocks.is_kind(below, self.snow_layer))
    }

    fn pick(&self, random: &mut MersenneTwister) -> BlockId {
        let Some(alternate) = &self.alternate else { return self.block };
        let chosen = match alternate.chance {
            Chance::OneIn(count) => random.next_int_bounded(count) == 0,
            Chance::AtLeast(threshold) => random.next_float() >= threshold,
        };
        if chosen { alternate.block } else { self.block }
    }

    pub fn place(&self, region: &mut Region, random: &mut MersenneTwister, origin: IVec3) -> bool {
        if origin.y < region.min_y() + self.min_height_above_floor {
            return false;
        }
        let radius_x = random.next_int_bounded(self.radius_choices) + self.radius_base;
        let radius_z = random.next_int_bounded(self.radius_choices) + self.radius_base;
        for x in origin.x - radius_x..=origin.x + radius_x {
            for y in origin.y..origin.y + self.layers {
                for z in origin.z - radius_z..=origin.z + radius_z {
                    let (dx, dz) = (origin.x - x, origin.z - z);
                    let near = random.next_float() * self.near_scale;
                    let far = random.next_float() * self.far_scale;
                    let kept = (dx * dx + dz * dz) as f32 <= near - far || random.next_float() < self.stray_chance;
                    let pos = IVec3::new(x, y, z);
                    if kept && self.supports(region, pos - IVec3::Y) && is_air(region, pos) {
                        let block = self.pick(random);
                        region.set(pos, block);
                    }
                }
            }
        }
        true
    }
}
