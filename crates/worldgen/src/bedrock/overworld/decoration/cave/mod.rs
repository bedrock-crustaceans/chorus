mod blob;
mod loader;
mod lush;
mod multiface;
mod patch;
mod plant;
mod sculk;
mod snap;
mod speleothem;
pub(super) mod support;
mod weighted;

use super::ore::Draw;
use super::placed::Placed;
use crate::shared::blocks::{BlockId, BlockTable, Blocks};
use crate::shared::chunk::region::Region;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::Probed;
use glam::IVec3;
use serde_json::Value;
use std::sync::Arc;

use blob::Blob;
pub(super) use loader::register;
use lush::Lush;
use multiface::{Multiface, Threshold};
use patch::Patch;
use plant::Plant;
use sculk::Sculk;
use snap::Snap;
use speleothem::Speleothem;

const RANGE_MIN: &str = "range_min";
const RANGE_MAX: &str = "range_max";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub min: i32,
    pub max: i32,
}

impl Span {
    pub fn new(min: i32, max: i32) -> Self {
        Self { min, max }
    }

    pub fn parse(value: Option<&Value>) -> Option<Self> {
        match value? {
            Value::Number(number) => number.as_i64().map(|fixed| Self::new(fixed as i32, fixed as i32)),
            Value::Object(map) => Some(Self::new(map.get(RANGE_MIN)?.as_i64()? as i32, map.get(RANGE_MAX)?.as_i64()? as i32)),
            _ => None,
        }
    }

    pub fn draw<D: Draw>(self, random: &mut D) -> i32 {
        if self.min >= self.max - 1 { self.min } else { self.min + random.below(self.max - self.min) }
    }
}

pub fn pick_weighted<'a, T, D: Draw>(entries: &'a [(T, f32)], random: &mut D) -> Option<&'a T> {
    let total = entries.iter().fold(0i32, |total, (_, weight)| (total as f32 + weight) as i32);
    let mut left = if total == 0 { 0 } else { random.below(total) };
    for (entry, weight) in entries {
        left = (left as f32 - weight) as i32;
        if left < 0 {
            return Some(entry);
        }
    }
    None
}

#[derive(Debug, Default)]
pub struct BlockSet {
    kinds: Vec<u16>,
}

impl BlockSet {
    pub fn parse(table: &mut BlockTable, names: Option<&'static Value>) -> Self {
        let mut kinds = Vec::new();
        for name in names.and_then(Value::as_array).into_iter().flatten().filter_map(Self::name_of) {
            kinds.push(table.kind(name));
            if name == chorus_block::block_id::STONE {
                kinds.extend(STONE_VARIANTS.iter().map(|variant| table.kind(*variant)));
            }
        }
        Self { kinds }
    }

    fn name_of(entry: &'static Value) -> Option<&'static str> {
        let name = match entry {
            Value::String(name) => name.as_str(),
            Value::Object(map) => map.get("name")?.as_str()?,
            _ => return None,
        };
        Some(if name == LEGACY_GRASS { chorus_block::block_id::GRASS_BLOCK } else { name })
    }

    pub fn from_names(table: &mut BlockTable, names: &[&'static str]) -> Self {
        Self {
            kinds: names.iter().map(|name| table.kind(*name)).collect(),
        }
    }

    pub fn contains(&self, blocks: &Blocks, block: BlockId) -> bool {
        self.kinds.iter().any(|&kind| blocks.is_kind(block, kind))
    }

    pub fn kinds(&self) -> &[u16] {
        &self.kinds
    }

    pub fn extend(&mut self, other: Self) {
        self.kinds.extend(other.kinds);
    }

    pub fn is_empty(&self) -> bool {
        self.kinds.is_empty()
    }
}

const LEGACY_GRASS: &str = "minecraft:grass";
const STONE_VARIANTS: [&str; 3] = [chorus_block::block_id::GRANITE, chorus_block::block_id::DIORITE, chorus_block::block_id::ANDESITE];

pub trait CaveDraw: Draw {
    fn gaussian(&mut self) -> f32;
}

impl CaveDraw for MersenneTwister {
    fn gaussian(&mut self) -> f32 {
        self.next_gaussian()
    }
}

#[derive(Debug)]
pub enum Cave {
    Snap(Snap),
    Patch(Patch),
    Plant(Plant),
    Weighted(Vec<(Arc<Placed>, f32)>),
    Lush(Lush),
    Speleothem(Speleothem),
    Multiface(Multiface),
    Threshold(Threshold),
    Blob(Blob),
    Sculk(Sculk),
}

impl Cave {
    pub fn place(&self, region: &mut Region, random: &mut MersenneTwister, pos: IVec3) -> Probed<bool> {
        Ok(match self {
            Self::Snap(snap) => snap.place(region, random, pos)?,
            Self::Patch(patch) => patch.place(region, random, pos)?,
            Self::Plant(plant) => plant.place(region, random, pos),
            Self::Weighted(entries) => weighted::place(entries, region, random, pos)?,
            Self::Lush(lush) => lush.place(region, random, pos)?,
            Self::Speleothem(speleothem) => speleothem.place(region, random, pos),
            Self::Multiface(multiface) => multiface.place(region, random, pos),
            Self::Threshold(threshold) => threshold.place(region, random, pos)?,
            Self::Blob(blob) => blob.place(region, random, pos),
            Self::Sculk(sculk) => sculk.place(region, random, pos),
        })
    }
}
