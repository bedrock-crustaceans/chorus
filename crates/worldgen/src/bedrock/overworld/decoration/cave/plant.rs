use super::super::ore::resolve_block;
use super::super::world::{is_air, read};
use super::support::is_water_source;
use super::{Span, pick_weighted};
use crate::shared::blocks::{BlockId, BlockTable, int, state};
use crate::shared::chunk::region::Region;
use crate::shared::rng::mersenne::MersenneTwister;
use chorus_block::state::common::GROWING_PLANT_AGE;
use glam::IVec3;
use serde_json::Value;

const FIRST_AGE: i32 = 17;
const LAST_AGE: i32 = 25;
const DIRECTION_UP: &str = "UP";

#[derive(Debug)]
pub struct Head {
    block: BlockId,
    by_age: Vec<BlockId>,
}

#[derive(Debug)]
pub struct Plant {
    heights: Vec<(Span, f32)>,
    up: bool,
    age: Span,
    body: Vec<(BlockId, f32)>,
    head: Vec<(Head, f32)>,
    allow_water: bool,
}

fn weighted_blocks(table: &mut BlockTable, entries: Option<&'static Value>, flooded: bool) -> Option<Vec<(BlockId, f32)>> {
    entries?
        .as_array()?
        .iter()
        .map(|entry| {
            let pair = entry.as_array()?;
            let block = resolve_block(table, pair.first()?)?;
            if flooded {
                table.flooded(block);
            }
            Some((block, pair.get(1)?.as_f64()? as f32))
        })
        .collect()
}

impl Plant {
    pub fn parse(table: &mut BlockTable, body: &'static Value) -> Option<Self> {
        let heights = body
            .get("height_distribution")?
            .as_array()?
            .iter()
            .map(|entry| {
                let pair = entry.as_array()?;
                Some((Span::parse(pair.first())?, pair.get(1)?.as_f64()? as f32))
            })
            .collect::<Option<Vec<_>>>()?;
        let age = Span::parse(body.get("age")).unwrap_or(Span::new(0, 0));
        let allow_water = body.get("allow_water").and_then(Value::as_bool).unwrap_or(false);
        let heads = body.get("head_blocks")?.as_array()?;
        let head = heads
            .iter()
            .map(|entry| {
                let pair = entry.as_array()?;
                let descriptor = pair.first()?;
                let block = resolve_block(table, descriptor)?;
                if allow_water {
                    table.flooded(block);
                }
                let by_age = match descriptor.as_str() {
                    Some(name) if age.max != 0 => (FIRST_AGE..=LAST_AGE).map(|value| table.get(name, &[state(&GROWING_PLANT_AGE, int(value))])).collect(),
                    _ => Vec::new(),
                };
                Some((Head { block, by_age }, pair.get(1)?.as_f64()? as f32))
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Self {
            heights,
            up: body.get("growth_direction").and_then(Value::as_str) == Some(DIRECTION_UP),
            age,
            body: weighted_blocks(table, body.get("body_blocks"), allow_water)?,
            head,
            allow_water,
        })
    }

    pub fn place(&self, region: &mut Region, random: &mut MersenneTwister, pos: IVec3) -> bool {
        let span = pick_weighted(&self.heights, random).copied().unwrap_or(Span::new(0, 0));
        let height = span.draw(random);
        let age = self.age.draw(random);
        if height < 1 {
            return false;
        }
        let step = if self.up { IVec3::Y } else { IVec3::NEG_Y };
        let mut head_at = None;
        for index in 0..height {
            let cell = pos + step * index;
            let block = read(region, cell);
            if !(is_air(region, cell) || self.allow_water && is_water_source(region.blocks, block)) {
                continue;
            }
            if index == height - 1 || !is_air(region, cell + step) {
                head_at = Some(cell);
                break;
            }
            if let Some(&body) = pick_weighted(&self.body, random) {
                put(region, cell, body);
            }
        }
        let Some(head_at) = head_at else { return false };
        if let Some(head) = pick_weighted(&self.head, random) {
            let block = match head.by_age.get((age.clamp(FIRST_AGE, LAST_AGE) - FIRST_AGE) as usize) {
                Some(&aged) if self.age.max != 0 => aged,
                _ => head.block,
            };
            put(region, head_at, block);
        }
        true
    }
}

fn put(region: &mut Region, pos: IVec3, block: BlockId) {
    let in_water = is_water_source(region.blocks, read(region, pos));
    let block = if in_water { region.blocks.flooded(block) } else { block };
    region.set(pos, block);
}
