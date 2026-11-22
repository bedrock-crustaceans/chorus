use super::aquatic::{is_water, tries};
use super::disk::kinds;
use super::ore::{Draw, resolve_block};
use super::plants::spread;
use super::single_block::may_place;
use super::world::{is_air, is_solid, read};
use crate::shared::blocks::{BlockId, BlockTable};
use crate::shared::chunk::region::{Heightmap, Region};
use crate::shared::survival::can_survive;
use crate::shared::tags::Tag;
use chorus_block::block_id::*;
use glam::IVec3;
use serde_json::Value;

const COLUMN_MASK: u32 = 7;
const PILE_Y_MASK: u32 = 3;
const SHADE_DIAMOND: [(i32, i32); 13] = [(0, 0), (0, -1), (1, 0), (0, 1), (-1, 0), (1, -1), (-1, -1), (1, 1), (-1, 1), (0, -2), (2, 0), (0, 2), (-2, 0)];
const SHADE_REACH: i32 = 10;
const FIREFLY_TRIES: usize = 20;
const FIREFLY_SPREAD: i32 = 9;
const FIREFLY_SPREAD_CENTRE: i32 = 4;
const FIREFLY_EXTRA_BOUND: i32 = 3;
const HORIZONTAL: [IVec3; 4] = [IVec3::NEG_X, IVec3::X, IVec3::NEG_Z, IVec3::Z];

fn offset<D: Draw>(random: &mut D, mask: u32) -> i32 {
    let high = (random.word() & mask) as i32;
    high - (random.word() & mask) as i32
}

fn open_for_plant(region: &Region, pos: IVec3) -> bool {
    let block = read(region, pos);
    region.blocks.is_air(block) || region.blocks.is(block, Tag::Replaceable)
}

#[derive(Debug)]
enum Start {
    Ground,
    Origin,
}

#[derive(Debug)]
enum Open {
    Air,
    Replaceable,
}

#[derive(Debug)]
pub struct SpreadPlant {
    block: BlockId,
    tries: i32,
    start: Start,
    open: Open,
    below: Option<Vec<u16>>,
    survive: bool,
}

impl SpreadPlant {
    pub fn parse(table: &mut BlockTable, body: &'static Value) -> Option<Self> {
        let block = table.name(body.get("block")?.as_str()?);
        let start = match body.get("start")?.as_str()? {
            "ground" => Start::Ground,
            "origin" => Start::Origin,
            _ => return None,
        };
        let open = match body.get("open")?.as_str()? {
            "air" => Open::Air,
            "replaceable" => Open::Replaceable,
            _ => return None,
        };
        let below = match body.get("below") {
            Some(_) => Some(kinds(table, body, "below")?),
            None => None,
        };
        Some(Self {
            block,
            tries: tries(body),
            start,
            open,
            below,
            survive: body.get("survive").and_then(Value::as_bool).unwrap_or(false),
        })
    }

    fn is_open(&self, region: &Region, pos: IVec3) -> bool {
        match self.open {
            Open::Air => is_air(region, pos),
            Open::Replaceable => open_for_plant(region, pos),
        }
    }

    pub fn place<D: Draw>(&self, region: &mut Region, random: &mut D, origin: IVec3) -> bool {
        let mut base = origin;
        if matches!(self.start, Start::Ground) {
            let mut seen_solid = false;
            while !seen_solid && base.y > region.min_y() {
                seen_solid = !is_air(region, base);
                base.y -= 1;
            }
        }
        for _ in 0..self.tries {
            let pos = base + spread(random);
            if !self.is_open(region, pos) {
                continue;
            }
            if let Some(below) = &self.below {
                let under = read(region, pos - IVec3::Y);
                if !below.iter().any(|&kind| region.blocks.is_kind(under, kind)) {
                    continue;
                }
            }
            if self.survive && !can_survive(region, self.block, pos) {
                continue;
            }
            region.set(pos, self.block);
        }
        true
    }
}

#[derive(Debug)]
pub struct WaterEdgeColumn {
    block: BlockId,
    tries: i32,
    min_height: i32,
    bound: i32,
}

impl WaterEdgeColumn {
    pub fn parse(table: &mut BlockTable, body: &'static Value) -> Option<Self> {
        Some(Self {
            block: table.name(body.get("block")?.as_str()?),
            tries: tries(body),
            min_height: body.get("min_height")?.as_i64()? as i32,
            bound: body.get("bound")?.as_i64()? as i32,
        })
    }

    pub fn place<D: Draw>(&self, region: &mut Region, random: &mut D, origin: IVec3) -> bool {
        for _ in 0..self.tries {
            let dx = offset(random, COLUMN_MASK);
            let dz = offset(random, COLUMN_MASK);
            let pos = origin + IVec3::new(dx, 0, dz);
            if !is_air(region, pos) {
                continue;
            }
            if !HORIZONTAL.iter().any(|side| is_water(region, pos - IVec3::Y + *side)) {
                continue;
            }
            let bound = random.below(self.bound) + 1;
            let height = random.below(bound) + self.min_height;
            for step in 0..height {
                let cell = pos + IVec3::Y * step;
                if !is_air(region, cell) || !can_survive(region, self.block, cell) {
                    break;
                }
                region.set(cell, self.block);
            }
        }
        true
    }
}

#[derive(Debug)]
pub struct Waterlily {
    block: BlockId,
}

impl Waterlily {
    pub fn new(table: &mut BlockTable) -> Self {
        Self { block: table.name(WATERLILY) }
    }

    pub fn place(&self, region: &mut Region, origin: IVec3) -> bool {
        let open = is_air(region, origin) && is_water(region, origin - IVec3::Y) && region.height(Heightmap::WorldSurfaceWg, origin.x, origin.z) <= origin.y;
        open && region.set(origin, self.block)
    }
}

#[derive(Debug)]
pub struct RandomWalkPlant {
    tries: i32,
    variants: Vec<BlockId>,
    below: Vec<u16>,
}

impl RandomWalkPlant {
    pub fn parse(table: &mut BlockTable, body: &'static Value) -> Option<Self> {
        let variants: Vec<BlockId> = body.get("variants")?.as_array()?.iter().map(|variant| resolve_block(table, variant)).collect::<Option<_>>()?;
        if variants.is_empty() {
            return None;
        }
        Some(Self {
            tries: tries(body),
            variants,
            below: kinds(table, body, "below")?,
        })
    }

    pub fn place<D: Draw>(&self, region: &mut Region, random: &mut D, origin: IVec3) -> bool {
        let mut pos = origin;
        for _ in 0..self.tries {
            if is_air(region, pos) && self.below.iter().any(|&kind| region.blocks.is_kind(read(region, pos - IVec3::Y), kind)) {
                let chosen = self.variants[random.below(self.variants.len() as i32) as usize];
                region.set(pos, chosen);
            }
            let dx = offset(random, COLUMN_MASK);
            let dz = offset(random, COLUMN_MASK);
            let dy = offset(random, PILE_Y_MASK);
            pos = origin + IVec3::new(dx, dy, dz);
        }
        true
    }
}

#[derive(Debug)]
pub struct Mushroom {
    block: BlockId,
    ground: Vec<u16>,
    tries: i32,
}

impl Mushroom {
    pub fn parse(table: &mut BlockTable, body: &'static Value) -> Option<Self> {
        Some(Self {
            block: table.name(body.get("block")?.as_str()?),
            ground: kinds(table, body, "ground")?,
            tries: tries(body),
        })
    }

    fn shaded(region: &mut Region, pos: IVec3) -> bool {
        region.height(Heightmap::WorldSurfaceWg, pos.x, pos.z) > pos.y || (pos.y + 1..=pos.y + SHADE_REACH).any(|above| is_solid(region, IVec3::new(pos.x, above, pos.z)))
    }

    pub fn place<D: Draw>(&self, region: &mut Region, random: &mut D, origin: IVec3) -> bool {
        let mut pos = origin;
        for _ in 0..self.tries {
            let below = read(region, pos - IVec3::Y);
            if is_air(region, pos) && !region.blocks.is_air(below) {
                let grows_anywhere = self.ground.iter().any(|&kind| region.blocks.is_kind(below, kind)) || region.blocks.is(below, Tag::Logs);
                if grows_anywhere || (SHADE_DIAMOND.iter().all(|&(dx, dz)| Self::shaded(region, pos + IVec3::new(dx, 0, dz))) && can_survive(region, self.block, pos)) {
                    region.set(pos, self.block);
                }
            }
            pos = origin + spread(random);
        }
        true
    }
}

#[derive(Debug)]
pub struct Firefly {
    block: BlockId,
    needs_water: bool,
}

impl Firefly {
    pub fn new(table: &mut BlockTable, needs_water: bool) -> Self {
        Self {
            block: table.name(FIREFLY_BUSH),
            needs_water,
        }
    }

    fn near<D: Draw>(&self, region: &mut Region, random: &mut D, around: IVec3) -> Option<IVec3> {
        let x = around.x + random.below(FIREFLY_SPREAD) - FIREFLY_SPREAD_CENTRE;
        let z = around.z + random.below(FIREFLY_SPREAD) - FIREFLY_SPREAD_CENTRE;
        let pos = IVec3::new(x, region.height(Heightmap::WorldSurfaceWg, x, z), z);
        may_place(region, pos, self.block).then_some(pos)
    }

    fn beside_water(region: &Region, pos: IVec3) -> bool {
        HORIZONTAL.iter().any(|side| is_water(region, pos - IVec3::Y + *side))
    }

    pub fn place<D: Draw>(&self, region: &mut Region, random: &mut D, origin: IVec3) -> bool {
        for _ in 0..FIREFLY_TRIES {
            let Some(pos) = self.near(region, random, origin) else { continue };
            if self.needs_water && !Self::beside_water(region, pos) {
                continue;
            }
            region.set(pos, self.block);
            let mut remaining = random.below(FIREFLY_EXTRA_BOUND) + 1;
            for _ in 0..FIREFLY_TRIES {
                if let Some(extra) = self.near(region, random, pos) {
                    region.set(extra, self.block);
                    remaining -= 1;
                }
                if remaining == 0 {
                    break;
                }
            }
            return true;
        }
        false
    }
}
