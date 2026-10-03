use std::collections::HashMap;

use glam::IVec3;

use super::super::blocks::{BlockId, BlockTable, Blocks, int, state};
use super::super::random::RandomSource;
use super::super::tags::Tag;
use super::Context;
use super::predicate::{Direction, Fluid, fluid_at, multiface_bit};
use super::region::Region;
use super::tree::shuffle;
use chorus_block::block_id::{FIRE, SOUL_FIRE, WATER};
use chorus_block::state::common::MULTI_FACE_DIRECTION_BITS;

const WATERLOGGED_BIT: u8 = 1 << 6;

fn face_bit(direction: Direction) -> u8 {
    1 << Direction::ALL.iter().position(|&d| d == direction).unwrap_or(0)
}

pub struct MultifaceStates {
    pub kind: u16,
    states: Vec<BlockId>,
    masks: HashMap<BlockId, u8>,
}

impl MultifaceStates {
    pub fn register(table: &mut BlockTable, name: &'static str) -> Self {
        let mut states = Vec::with_capacity(128);
        let mut masks = HashMap::new();
        for mask in 0u8..128 {
            let bits = Direction::ALL.iter().filter(|&&d| mask & face_bit(d) != 0).map(|&d| multiface_bit(d)).sum();
            let dry = table.get(name, &[state(MULTI_FACE_DIRECTION_BITS, int(bits))]);
            let id = if mask & WATERLOGGED_BIT != 0 { table.flooded(dry) } else { dry };
            states.push(id);
            masks.insert(id, mask);
        }
        Self {
            kind: table.kind(name),
            states,
            masks,
        }
    }

    pub fn mask(&self, state: BlockId) -> Option<u8> {
        self.masks.get(&state).copied()
    }

    pub fn state(&self, mask: u8) -> BlockId {
        self.states[mask as usize]
    }

    pub fn is(&self, blocks: &Blocks, state: BlockId) -> bool {
        blocks.entry(state).kind == self.kind
    }

    pub fn has_face(&self, state: BlockId, face: Direction) -> bool {
        self.mask(state).is_some_and(|mask| mask & face_bit(face) != 0)
    }

    pub fn faces(&self, state: BlockId) -> Vec<Direction> {
        Direction::ALL.into_iter().filter(|&d| self.has_face(state, d)).collect()
    }

    pub fn with_face(&self, state: BlockId, face: Direction, set: bool) -> BlockId {
        let mask = self.mask(state).unwrap_or(0);
        self.state(if set { mask | face_bit(face) } else { mask & !face_bit(face) })
    }

    pub fn has_any_face(&self, state: BlockId) -> bool {
        self.mask(state).is_some_and(|mask| mask & !WATERLOGGED_BIT != 0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SpreadType {
    SamePosition,
    SamePlane,
    WrapAround,
}

pub const DEFAULT_ORDER: &[SpreadType] = &[SpreadType::SamePosition, SpreadType::SamePlane, SpreadType::WrapAround];
pub const SAME_POSITION: &[SpreadType] = &[SpreadType::SamePosition];

impl SpreadType {
    fn spread_pos(self, pos: IVec3, spread: Direction, from: Direction) -> (IVec3, Direction) {
        match self {
            Self::SamePosition => (pos, spread),
            Self::SamePlane => (pos + spread.offset(), from),
            Self::WrapAround => (pos + spread.offset() + from.offset(), spread.opposite()),
        }
    }
}

pub struct SculkBlocks {
    pub sculk: u16,
    pub catalyst: u16,
    pub moving_block: u16,
}

pub struct Spreader<'s> {
    pub states: &'s MultifaceStates,
    pub types: &'static [SpreadType],
    pub sculk: Option<&'s SculkBlocks>,
    pub post_process: bool,
}

fn full(region: &Region, pos: IVec3) -> bool {
    region.blocks.entry(region.get(pos)).full
}

fn is_water_source(blocks: &Blocks, state: BlockId) -> bool {
    fluid_at(blocks, state) == Some(Fluid::Water)
}

fn axis(direction: Direction) -> u8 {
    match direction {
        Direction::Down | Direction::Up => 0,
        Direction::North | Direction::South => 1,
        Direction::West | Direction::East => 2,
    }
}

impl Spreader<'_> {
    fn other_block_valid_as_source(&self, region: &Region, state: BlockId) -> bool {
        self.sculk.is_some() && !self.states.is(region.blocks, state)
    }

    fn can_spread_from(&self, region: &Region, state: BlockId, face: Direction) -> bool {
        self.other_block_valid_as_source(region, state) || self.states.has_face(state, face)
    }

    pub fn valid_for_placement(&self, region: &Region, old: BlockId, pos: IVec3, face: Direction) -> bool {
        (!self.states.is(region.blocks, old) || !self.states.has_face(old, face)) && full(region, pos + face.offset())
    }

    pub fn state_for_placement(&self, region: &Region, old: BlockId, pos: IVec3, face: Direction) -> Option<BlockId> {
        if !self.valid_for_placement(region, old, pos, face) {
            return None;
        }
        let base = if self.states.is(region.blocks, old) {
            old
        } else if is_water_source(region.blocks, old) {
            self.states.state(WATERLOGGED_BIT)
        } else {
            self.states.state(0)
        };
        Some(self.states.with_face(base, face, true))
    }

    fn state_can_be_replaced(&self, region: &Region, source: IVec3, pos: IVec3, face: Direction, existing: BlockId) -> bool {
        let blocks = region.blocks;
        let default = blocks.is_air(existing) || self.states.is(blocks, existing) || blocks.name_of(existing) == WATER;
        let Some(sculk) = self.sculk else { return default };
        let against = blocks.entry(region.get(pos + face.offset())).kind;
        if against == sculk.sculk || against == sculk.catalyst || against == sculk.moving_block {
            return false;
        }
        if (source - pos).abs().element_sum() == 2 && full(region, source - face.offset()) {
            return false;
        }
        if fluid_at(blocks, existing) == Some(Fluid::Lava) {
            return false;
        }
        if matches!(blocks.name_of(existing), FIRE | SOUL_FIRE) {
            return false;
        }
        blocks.is(existing, Tag::Replaceable) || default
    }

    fn can_spread_into(&self, region: &Region, source: IVec3, (pos, face): (IVec3, Direction)) -> bool {
        let existing = region.get(pos);
        self.state_can_be_replaced(region, source, pos, face, existing) && self.valid_for_placement(region, existing, pos, face)
    }

    fn spread_target(&self, region: &Region, state: BlockId, pos: IVec3, from: Direction, spread: Direction) -> Option<(IVec3, Direction)> {
        if axis(spread) == axis(from) {
            return None;
        }
        if !self.other_block_valid_as_source(region, state) && (!self.states.has_face(state, from) || self.states.has_face(state, spread)) {
            return None;
        }
        self.types
            .iter()
            .map(|kind| kind.spread_pos(pos, spread, from))
            .find(|&target| self.can_spread_into(region, pos, target))
    }

    fn spread_to_face(&self, region: &mut Region, (pos, face): (IVec3, Direction)) -> bool {
        let old = region.get(pos);
        match self.state_for_placement(region, old, pos, face) {
            Some(state) => {
                if self.post_process {
                    region.mark_post_process(pos);
                }
                region.set(pos, state)
            }
            None => false,
        }
    }

    fn spread_toward(&self, region: &mut Region, state: BlockId, pos: IVec3, from: Direction, spread: Direction) -> bool {
        match self.spread_target(region, state, pos, from, spread) {
            Some(target) => self.spread_to_face(region, target),
            None => false,
        }
    }

    pub fn spread_toward_random_direction(&self, region: &mut Region, state: BlockId, pos: IVec3, from: Direction, random: &mut dyn RandomSource) -> bool {
        let mut directions = Direction::ALL;
        shuffle(&mut directions, random);
        directions.into_iter().any(|spread| self.spread_toward(region, state, pos, from, spread))
    }

    pub fn spread_all(&self, region: &mut Region, state: BlockId, pos: IVec3) -> i64 {
        let mut count = 0;
        for face in Direction::ALL {
            if !self.can_spread_from(region, state, face) {
                continue;
            }
            for spread in Direction::ALL {
                if self.spread_toward(region, state, pos, face, spread) {
                    count += 1;
                }
            }
        }
        count
    }
}

pub struct MultifaceGrowth {
    pub states: MultifaceStates,
    pub sculk: Option<SculkBlocks>,
    pub search_range: i32,
    pub floor: bool,
    pub ceiling: bool,
    pub wall: bool,
    pub chance_of_spreading: f32,
    pub can_be_placed_on: Vec<u16>,
}

impl MultifaceGrowth {
    fn spreader(&self) -> Spreader<'_> {
        Spreader {
            states: &self.states,
            types: DEFAULT_ORDER,
            sculk: self.sculk.as_ref(),
            post_process: true,
        }
    }

    fn valid_directions(&self) -> Vec<Direction> {
        let mut directions = Vec::with_capacity(6);
        if self.ceiling {
            directions.push(Direction::Up);
        }
        if self.floor {
            directions.push(Direction::Down);
        }
        if self.wall {
            directions.extend(Direction::HORIZONTAL);
        }
        directions
    }

    fn air_or_water(region: &Region, state: BlockId) -> bool {
        region.blocks.is_air(state) || region.blocks.name_of(state) == WATER
    }

    pub fn place(&self, ctx: &mut Context, random: &mut dyn RandomSource, origin: IVec3) -> bool {
        let region = &mut *ctx.region;
        if !Self::air_or_water(region, region.get(origin)) {
            return false;
        }
        let mut directions = self.valid_directions();
        shuffle(&mut directions, random);
        let state = region.get(origin);
        if self.place_growth(region, origin, state, random, &directions) {
            return true;
        }
        for &search in &directions {
            let mut placement: Vec<Direction> = self.valid_directions().into_iter().filter(|&d| d != search.opposite()).collect();
            shuffle(&mut placement, random);
            let pos = origin + search.offset();
            for _ in 0..self.search_range {
                let state = region.get(pos);
                if !Self::air_or_water(region, state) && !self.states.is(region.blocks, state) {
                    break;
                }
                if self.place_growth(region, pos, state, random, &placement) {
                    return true;
                }
            }
        }
        false
    }

    fn place_growth(&self, region: &mut Region, pos: IVec3, old: BlockId, random: &mut dyn RandomSource, directions: &[Direction]) -> bool {
        let spreader = self.spreader();
        for &direction in directions {
            let neighbour = region.get(pos + direction.offset());
            if !self.can_be_placed_on.contains(&region.blocks.entry(neighbour).kind) {
                continue;
            }
            let Some(state) = spreader.state_for_placement(region, old, pos, direction) else { return false };
            region.set(pos, state);
            region.mark_post_process(pos);
            if random.next_float() < self.chance_of_spreading {
                spreader.spread_toward_random_direction(region, state, pos, direction, random);
            }
            return true;
        }
        false
    }
}
