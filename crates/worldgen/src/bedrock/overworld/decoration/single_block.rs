use super::ore::{Draw, block_state, resolve_block};
use super::world::read;
use crate::shared::blocks::{BlockId, BlockTable, State, int, state};
use crate::shared::chunk::region::{Region, RuleKind};
use crate::shared::survival::can_survive;
use crate::shared::tags::Tag;
use chorus_block::block_id::{FERN, SHORT_GRASS, SNOW_LAYER};
use chorus_block::state::common::DIRECTION;
use glam::IVec3;
use serde_json::Value;

const LEGACY_TALL_GRASS: &str = "minecraft:tallgrass";
const TALL_GRASS_TYPE: &str = "tall_grass_type";
const FERN_TYPE: &str = "fern";
const SIDES: [IVec3; 4] = [IVec3::NEG_Z, IVec3::X, IVec3::Z, IVec3::NEG_X];
const DIAGONALS: [IVec3; 4] = [IVec3::new(1, 0, 1), IVec3::new(-1, 0, 1), IVec3::new(1, 0, -1), IVec3::new(-1, 0, -1)];
const SIDE_KEYS: [&str; 4] = crate::shared::blocks::COMPASS;
const SIDE_DIRECTIONS: [i32; 4] = [2, 3, 0, 1];
const DIRECTION_KEY: &str = "direction";
const DEFAULT_MIN_SIDES: i32 = 4;

pub fn may_place(region: &Region, pos: IVec3, block: BlockId) -> bool {
    let there = region.blocks.entry(read(region, pos));
    let open = region.blocks.is_air(read(region, pos)) || there.liquid || there.flooded || there.name.as_ref() == SNOW_LAYER || region.blocks.is(read(region, pos), Tag::Replaceable);
    let is_lower_half = region.blocks.entry(block).upper == Some(false);
    open && can_survive(region, block, pos) && (!is_lower_half || region.blocks.is_air(read(region, pos + IVec3::Y)))
}

fn resolve(table: &mut BlockTable, descriptor: &'static Value) -> Option<BlockId> {
    if descriptor.get("name").and_then(Value::as_str) == Some(LEGACY_TALL_GRASS) {
        let fern = descriptor["states"].get(TALL_GRASS_TYPE).and_then(Value::as_str) == Some(FERN_TYPE);
        return Some(table.name(if fern { FERN } else { SHORT_GRASS }));
    }
    resolve_block(table, descriptor)
}

fn kinds(table: &mut BlockTable, value: Option<&'static Value>) -> Option<Vec<u16>> {
    match value {
        None => Some(Vec::new()),
        Some(Value::Array(items)) => items.iter().map(|item| block_kind(table, item)).collect(),
        Some(single) => Some(vec![block_kind(table, single)?]),
    }
}

fn block_kind(table: &mut BlockTable, value: &'static Value) -> Option<u16> {
    match value {
        Value::String(name) => Some(table.kind(name)),
        Value::Object(map) => Some(table.kind(map.get("name")?.as_str()?)),
        _ => None,
    }
}

fn matches_any(region: &Region, pos: IVec3, lists: &[&[u16]]) -> bool {
    let kind = region.blocks.entry(read(region, pos)).kind;
    lists.iter().all(|list| list.is_empty()) || lists.iter().any(|list| list.contains(&kind))
}

#[derive(Debug, Default)]
struct Attach {
    top: Vec<u16>,
    bottom: Vec<u16>,
    diagonal: Vec<u16>,
    sides: [Vec<u16>; 4],
    any_side: Vec<u16>,
    all: Vec<u16>,
    min_sides: i32,
    auto_rotate: bool,
}

impl Attach {
    fn parse(table: &mut BlockTable, body: &'static Value) -> Option<Self> {
        let mut sides: [Vec<u16>; 4] = Default::default();
        for (slot, key) in sides.iter_mut().zip(SIDE_KEYS) {
            *slot = kinds(table, body.get(key))?;
        }
        Some(Self {
            top: kinds(table, body.get("top"))?,
            bottom: kinds(table, body.get("bottom"))?,
            diagonal: kinds(table, body.get("diagonal"))?,
            sides,
            any_side: kinds(table, body.get("sides"))?,
            all: kinds(table, body.get("all"))?,
            min_sides: body.get("min_sides_must_attach").and_then(Value::as_i64).unwrap_or(DEFAULT_MIN_SIDES as i64) as i32,
            auto_rotate: body.get("auto_rotate").and_then(Value::as_bool).unwrap_or(true),
        })
    }

    fn allows(&self, region: &Region, pos: IVec3) -> Option<Option<usize>> {
        if !matches_any(region, pos + IVec3::Y, &[&self.top, &self.all]) || !matches_any(region, pos - IVec3::Y, &[&self.bottom, &self.all]) {
            return None;
        }
        if DIAGONALS.iter().any(|&diagonal| !matches_any(region, pos + diagonal, &[&self.diagonal, &self.all])) {
            return None;
        }
        let mut matched = 0;
        let mut last = None;
        for (index, (&side, listed)) in SIDES.iter().zip(&self.sides).enumerate() {
            if matches_any(region, pos + side, &[listed, &self.any_side, &self.all]) {
                matched += 1;
                last = Some(index);
            }
        }
        (matched >= self.min_sides).then_some(last)
    }
}

fn rotations(table: &mut BlockTable, descriptor: &'static Value) -> Option<[BlockId; 4]> {
    let name = descriptor.get("name")?.as_str()?;
    let kept: Vec<State> = descriptor["states"]
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(key, _)| key.as_str() != DIRECTION_KEY)
        .filter_map(|(key, value)| block_state(key, value))
        .collect();
    let mut ids = [0; 4];
    for (slot, direction) in ids.iter_mut().zip(SIDE_DIRECTIONS) {
        let mut states = kept.clone();
        states.push(state(&DIRECTION, int(direction)));
        *slot = table.get(name, &states);
    }
    Some(ids)
}

#[derive(Debug)]
pub struct SingleBlock {
    blocks: Vec<(BlockId, i32)>,
    turned: Vec<[BlockId; 4]>,
    may_replace: Vec<u16>,
    placement_rules: bool,
    survival_rules: bool,
    not_on: Vec<u16>,
    attach: Option<Attach>,
}

impl SingleBlock {
    pub fn parse(table: &mut BlockTable, body: &'static Value) -> Option<Self> {
        let blocks = match &body["places_block"] {
            Value::Array(entries) => entries
                .iter()
                .map(|entry| Some((resolve(table, entry.get("block")?)?, entry.get("weight").and_then(Value::as_i64).unwrap_or(1) as i32)))
                .collect::<Option<Vec<_>>>()?,
            single => vec![(resolve(table, single)?, 1)],
        };
        let attach = match body.get("may_attach_to") {
            Some(attach) => Some(Attach::parse(table, attach)?),
            None => None,
        };
        let turned = match (&attach, &body["places_block"]) {
            (Some(attach), descriptor @ Value::Object(_)) if attach.auto_rotate => vec![rotations(table, descriptor)?],
            (Some(attach), _) if attach.auto_rotate => return None,
            _ => Vec::new(),
        };
        Some(Self {
            turned,
            blocks,
            may_replace: kinds(table, body.get("may_replace"))?,
            placement_rules: body["enforce_placement_rules"].as_bool().unwrap_or(false),
            survival_rules: body["enforce_survivability_rules"].as_bool().unwrap_or(false),
            not_on: kinds(table, body["may_not_attach_to"].get("bottom"))?,
            attach,
        })
    }

    pub fn place<D: Draw>(&self, region: &mut Region, random: &mut D, pos: IVec3) -> bool {
        if !self.may_replace.is_empty() {
            region.set_rule(&self.may_replace, RuleKind::Vegetation);
        }
        let below = region.blocks.entry(read(region, pos - IVec3::Y)).kind;
        if self.not_on.contains(&below) {
            return false;
        }
        let total: i32 = self.blocks.iter().map(|&(_, weight)| weight).sum();
        let mut left = if total == 0 { 0 } else { random.below(total) };
        let Some(&(block, _)) = self.blocks.iter().find(|&&(_, weight)| {
            left -= weight;
            left < 0
        }) else {
            return false;
        };
        let mut block = block;
        if let Some(attach) = &self.attach {
            let Some(last) = attach.allows(region, pos) else { return false };
            if let (Some(side), Some(rotated)) = (last, self.turned.first()) {
                block = rotated[side];
            }
        }
        if self.placement_rules && !may_place(region, pos, block) {
            return false;
        }
        if self.survival_rules && !can_survive(region, block, pos) {
            return false;
        }
        if !self.may_replace.is_empty() && !self.may_replace.contains(&region.blocks.entry(read(region, pos)).kind) {
            return false;
        }
        region.set(pos, block)
    }
}
