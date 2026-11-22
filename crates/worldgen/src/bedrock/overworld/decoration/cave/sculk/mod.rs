use super::super::world::{is_air, read};
use super::BlockSet;
use super::multiface::{Multiface, SPREAD_ORDER};
use super::support::holds_face;
use crate::shared::blocks::{self, BlockId, BlockTable, Blocks};
use crate::shared::chunk::region::{Region, RuleKind};
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::tags::Tag;
use chorus_block::block_id::*;
use chorus_block::state::block_state::BlockState;
use chorus_block::state::common::CAN_SUMMON;
use glam::IVec3;
use serde_json::Value;

mod charge;
mod spread;
mod tables;

use tables::{CLEAR_SOLIDS, GLASS_SUFFIX, LIGHT_BLOCK_PREFIX, REPLACEABLE};

const MAX_CURSORS: usize = 32;

const MAX_CHARGE: i32 = 1000;

const MAX_DISTANCE_SQUARED: i32 = 224;

const FACINGS_UNSET: i32 = -1;

const FACINGS_EMPTY: i32 = -2;

const SHRIEKER_SPREAD: i32 = 5;

const SHRIEKER_OFFSET: i32 = 2;

fn opaque_solid(blocks: &Blocks, block: BlockId) -> bool {
    let entry = blocks.entry(block);
    entry.cube
        && !blocks.is(block, Tag::Leaves)
        && !entry.name.ends_with(GLASS_SUFFIX)
        && !entry.name.starts_with(LIGHT_BLOCK_PREFIX)
        && !entry.name.contains("copper_grate")
        && !CLEAR_SOLIDS.iter().any(|name| *name == entry.name.as_ref())
}

#[derive(Debug)]
pub struct Sculk {
    veins: Multiface,
    sculk: BlockId,
    sculk_kind: u16,
    catalyst: BlockId,
    sensor_kinds: [u16; 2],
    sensor: BlockId,
    shrieker: BlockId,
    replaceable: BlockSet,
    can_place_on: BlockSet,
    cursors: i32,
    charge: i32,
    attempts: i32,
    rounds: [i32; 2],
    catalyst_chance: f32,
    extra: (i32, i32),
}

struct Cursor {
    pos: IVec3,
    charge: i32,
    update_delay: i32,
    decay_delay: i32,
    facings: i32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Behaviour {
    Default,
    Sculk,
    Vein,
}

impl Sculk {
    pub fn parse(table: &mut BlockTable, body: &'static Value) -> Option<Self> {
        let number = |key: &str| body.get(key).and_then(Value::as_i64).map(|value| value as i32);
        let extra = body.get("extra_growth_chance");
        let bound = |key: &str| extra.and_then(|range| range.get(key)).and_then(Value::as_i64).unwrap_or(0) as i32;
        let mut replaceable = BlockSet::from_names(table, &REPLACEABLE);
        replaceable.extend(BlockSet::from_names(table, Tag::SulfurSpikeReplaceableBlocks.members()));
        Some(Self {
            veins: Multiface::spreader(table),
            sculk: table.name(SCULK),
            sculk_kind: table.kind(SCULK),
            catalyst: table.name(SCULK_CATALYST),
            sensor_kinds: [table.kind(SCULK_SENSOR), table.kind(SCULK_SHRIEKER)],
            sensor: table.name(SCULK_SENSOR),
            shrieker: table.get(SCULK_SHRIEKER, &[blocks::state(&CAN_SUMMON, BlockState::Bool(true))]),
            replaceable,
            can_place_on: BlockSet::parse(table, body.get("can_place_sculk_patch_on")),
            cursors: number("cursor_count")?,
            charge: number("charge_amount")?,
            attempts: number("spread_attempts")?,
            rounds: [number("growth_rounds")?, number("spread_rounds")?],
            catalyst_chance: body.get("central_block_placement_chance")?.as_f64()? as f32,
            extra: (bound("range_min"), bound("range_max")),
        })
    }

    fn behaviour(&self, region: &Region, block: BlockId) -> Behaviour {
        if region.blocks.is_kind(block, self.sculk_kind) {
            Behaviour::Sculk
        } else if self.veins.is_vein(region, block) {
            Behaviour::Vein
        } else {
            Behaviour::Default
        }
    }

    fn can_start(&self, region: &Region, at: IVec3) -> bool {
        let block = read(region, at);
        if self.behaviour(region, block) != Behaviour::Default {
            return true;
        }
        let blocks = region.blocks;
        if !(blocks.is_air(block) || blocks.name_of(block) == WATER) {
            return false;
        }
        SPREAD_ORDER.iter().any(|&face| {
            let next = read(region, at + face.offset());
            if self.can_place_on.is_empty() {
                opaque_solid(blocks, next)
            } else {
                self.can_place_on.contains(blocks, next)
            }
        })
    }

    pub fn place(&self, region: &mut Region, random: &mut MersenneTwister, at: IVec3) -> bool {
        if !self.can_start(region, at) {
            return false;
        }
        region.set_rule(self.replaceable.kinds(), RuleKind::Replaceable);
        let mut cursors: Vec<Cursor> = Vec::new();
        for round in 0..self.rounds[0] + self.rounds[1] {
            for _ in 0..self.cursors {
                let mut charge = self.charge;
                while charge > 0 {
                    let part = charge.min(MAX_CHARGE);
                    if cursors.len() < MAX_CURSORS {
                        cursors.push(Cursor {
                            pos: at,
                            charge: part,
                            update_delay: 0,
                            decay_delay: 1,
                            facings: FACINGS_UNSET,
                        });
                    }
                    charge -= part;
                }
            }
            for _ in 0..self.attempts {
                self.update_cursors(region, &mut cursors, at, random, round < self.rounds[1]);
            }
            cursors.clear();
        }
        if random.next_float() <= self.catalyst_chance && holds_face(region.blocks, read(region, at - IVec3::Y)) {
            region.set(at, self.catalyst);
        }
        let (low, high) = self.extra;
        let count = if low <= high { low + random.next_int_bounded(high - low + 1) } else { low };
        for _ in 0..count {
            let dx = random.next_int_bounded(SHRIEKER_SPREAD) - SHRIEKER_OFFSET;
            let dz = random.next_int_bounded(SHRIEKER_SPREAD) - SHRIEKER_OFFSET;
            let pos = at + IVec3::new(dx, 0, dz);
            if is_air(region, pos) && holds_face(region.blocks, read(region, pos - IVec3::Y)) {
                region.set(pos, self.shrieker);
            }
        }
        true
    }

    fn update_cursors(&self, region: &mut Region, cursors: &mut Vec<Cursor>, origin: IVec3, random: &mut MersenneTwister, spread_veins: bool) {
        for cursor in cursors.iter_mut() {
            if cursor.charge > 0 {
                self.update(region, cursor, origin, random, spread_veins);
            }
        }
        cursors.retain(|cursor| cursor.charge > 0);
    }

    fn update(&self, region: &mut Region, cursor: &mut Cursor, origin: IVec3, random: &mut MersenneTwister, spread_veins: bool) {
        if cursor.update_delay >= 1 {
            cursor.update_delay -= 1;
            return;
        }
        let mut state = read(region, cursor.pos);
        let mut behaviour = self.behaviour(region, state);
        if spread_veins && self.attempt_spread_vein(region, behaviour, cursor.pos, state, cursor.facings) && behaviour != Behaviour::Sculk {
            state = read(region, cursor.pos);
            behaviour = self.behaviour(region, state);
        }
        cursor.charge = self.use_charge(region, behaviour, cursor, origin, random, spread_veins);
        if cursor.charge < 1 {
            self.on_discharged(region, behaviour, cursor.pos);
            return;
        }
        if let Some(next) = self.movement_target(region, cursor.pos, random) {
            self.on_discharged(region, behaviour, cursor.pos);
            cursor.pos = next;
            let offset = next - origin;
            if offset.length_squared() > MAX_DISTANCE_SQUARED {
                cursor.charge = 0;
                return;
            }
            state = read(region, next);
        }
        if self.behaviour(region, state) != Behaviour::Default {
            behaviour = self.behaviour(region, state);
            cursor.facings = match behaviour {
                Behaviour::Sculk => FACINGS_EMPTY,
                _ => self.veins.face_bits(region, state),
            };
        }
        cursor.update_delay = 1;
        cursor.decay_delay = if behaviour == Behaviour::Default { cursor.decay_delay.max(1) - 1 } else { 1 };
    }
}
