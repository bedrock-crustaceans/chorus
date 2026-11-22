use super::ore::Draw;
use super::world::read;
use crate::shared::blocks::{BlockId, BlockTable, state, text};
use crate::shared::chunk::region::Region;
use crate::shared::structure::Probed;
use crate::shared::survival::Direction;
use chorus_block::block_id::*;
use chorus_block::state::common::MINECRAFT_BLOCK_FACE;
use glam::IVec3;
use serde_json::Value;

mod noise;
mod scan;

pub use noise::GeodeNoise;
use noise::threshold;
use scan::{Scan, place_bud, sample, set_changed};

const CRACK_BASE_SIZE: f32 = 2.0;

const CRACK_HEIGHTS: [i32; 3] = [7, 5, 1];

const INNER_OFFSET: f32 = 2.2;

const MIDDLE_OFFSET: f32 = 3.2;

const OUTER_OFFSET: f32 = 4.2;

const CRACK_CHANCE: f32 = 0.95;

const CRACK_SIZE_SCALE: f32 = 0.5;

const CRACK_DIRECTIONS: i32 = 4;

const CRACK_MIN_POINTS: i32 = 4;

const REJECTED_BLOCKS: [&str; 5] = [BEDROCK, PACKED_ICE, BLUE_ICE, ICE, FROSTED_ICE];

#[derive(Debug)]
pub struct Geode {
    filler: BlockId,
    inner: BlockId,
    alternate: BlockId,
    middle: BlockId,
    outer: BlockId,
    placements: Vec<[BlockId; 6]>,
    min_wall: i32,
    max_wall: i32,
    min_points: i32,
    max_points: i32,
    min_offset: i32,
    max_offset: i32,
    max_radius: i32,
    crack_point_offset: f32,
    noise_multiplier: f32,
    placement_chance: f32,
    alternate_chance: f32,
    require_alternate: bool,
    invalid_threshold: i32,
    rejected: Vec<u16>,
}

fn block(table: &mut BlockTable, body: &'static Value, key: &str) -> Option<BlockId> {
    Some(table.name(body.get(key)?.as_str()?))
}

fn int(body: &Value, key: &str) -> Option<i32> {
    Some(body.get(key)?.as_i64()? as i32)
}

fn float(body: &Value, key: &str) -> Option<f32> {
    Some(body.get(key)?.as_f64()? as f32)
}

impl Geode {
    pub fn parse(table: &mut BlockTable, body: &'static Value) -> Option<Self> {
        let placements = body.get("inner_placements")?.as_array()?;
        let placements = placements
            .iter()
            .map(|entry| {
                let name = entry.get("name")?.as_str()?;
                Some(Direction::ALL.map(|face| table.get(name, &[state(&MINECRAFT_BLOCK_FACE, text(face.name()))])))
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Self {
            filler: block(table, body, "filler")?,
            inner: block(table, body, "inner_layer")?,
            alternate: block(table, body, "alternate_inner_layer")?,
            middle: block(table, body, "middle_layer")?,
            outer: block(table, body, "outer_layer")?,
            placements,
            min_wall: int(body, "min_outer_wall_distance")?,
            max_wall: int(body, "max_outer_wall_distance")?,
            min_points: int(body, "min_distribution_points")?,
            max_points: int(body, "max_distribution_points")?,
            min_offset: int(body, "min_point_offset")?,
            max_offset: int(body, "max_point_offset")?,
            max_radius: int(body, "max_radius")?,
            crack_point_offset: float(body, "crack_point_offset").unwrap_or(0.0),
            noise_multiplier: float(body, "noise_multiplier")?,
            placement_chance: float(body, "use_potential_placements_chance")?,
            alternate_chance: float(body, "use_alternate_layer0_chance")?,
            require_alternate: body.get("placements_require_layer0_alternate")?.as_bool()?,
            invalid_threshold: int(body, "invalid_blocks_threshold")?,
            rejected: REJECTED_BLOCKS.iter().map(|name| table.kind(*name)).collect(),
        })
    }

    fn is_rejected(&self, region: &Region, pos: IVec3) -> bool {
        let id = read(region, pos);
        let entry = region.blocks.entry(id);
        region.blocks.is_air(id) || entry.liquid || entry.flooded || self.rejected.contains(&entry.kind)
    }

    pub fn place<D: Draw>(&self, region: &mut Region, random: &mut D, origin: IVec3) -> Probed<bool> {
        region.begin_feature();
        region.set_any_rule();
        if region.in_structure(origin)? {
            return Ok(false);
        }
        let count = sample(random, self.min_points, self.max_points);
        let noise = GeodeNoise::new(random);
        let ratio = count as f32 / self.max_wall as f32;
        let layers = [INNER_OFFSET, MIDDLE_OFFSET, OUTER_OFFSET].map(|size| threshold(ratio, size));
        let crack_size = (if count >= CRACK_MIN_POINTS { ratio } else { 0.0 }) + (random.float() * CRACK_SIZE_SCALE + CRACK_BASE_SIZE);
        let crack_threshold = 1.0 / crack_size.sqrt();
        let crack = random.float() < CRACK_CHANCE;

        let mut points: Vec<(IVec3, i32)> = Vec::with_capacity(count.max(0) as usize);
        let mut invalid = 0;
        for _ in 0..count {
            let offset = IVec3::new(
                sample(random, self.min_wall, self.max_wall),
                sample(random, self.min_wall, self.max_wall),
                sample(random, self.min_wall, self.max_wall),
            );
            let anchor = origin + offset;
            if self.is_rejected(region, anchor) {
                if invalid >= self.invalid_threshold {
                    return Ok(false);
                }
                invalid += 1;
            }
            points.push((anchor, sample(random, self.min_offset, self.max_offset)));
        }
        let mut cracks = Vec::new();
        if crack {
            let span = count * 2 + 1;
            let (dx, dz) = match random.below(CRACK_DIRECTIONS) {
                0 => (span, 0),
                1 => (0, span),
                2 => (span, span),
                _ => (0, 0),
            };
            cracks.extend(CRACK_HEIGHTS.map(|dy| origin + IVec3::new(dx, dy, dz)));
        }
        let mut scan = Scan {
            geode: self,
            region,
            random,
            noise: &noise,
            origin,
            points,
            cracks,
            crack,
            crack_threshold,
            layers,
            buffered: Vec::new(),
            potential: Vec::new(),
            placed: false,
        };
        scan.run();
        let Scan {
            buffered,
            potential,
            mut placed,
            region,
            random,
            ..
        } = scan;
        for (pos, layer) in buffered {
            placed |= set_changed(region, pos, layer);
        }
        if !self.placements.is_empty() {
            for pos in potential {
                let faces = &self.placements[random.below(self.placements.len() as i32) as usize];
                placed |= place_bud(region, pos, faces);
            }
        }
        Ok(placed)
    }
}
