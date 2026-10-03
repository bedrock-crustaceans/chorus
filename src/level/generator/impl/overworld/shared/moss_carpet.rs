use glam::IVec3;

use super::blocks::{AIR, BlockId, BlockTable, flag, state, text};
use super::random::RandomSource;
use super::region::{BlockView, Region};
use super::survival::{Direction, can_survive};
use super::tags::Tag;
use crate::block::block_id::PALE_MOSS_CARPET;
use crate::block::state::block_state::BlockStateDefinition;
use crate::block::state::common::{PALE_MOSS_CARPET_SIDE_EAST, PALE_MOSS_CARPET_SIDE_NORTH, PALE_MOSS_CARPET_SIDE_SOUTH, PALE_MOSS_CARPET_SIDE_WEST, UPPER_BLOCK_BIT};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    None,
    Low,
    Tall,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Carpet {
    base: bool,
    sides: [Side; 4],
}

const SIDE_NAMES: [&str; 3] = ["none", "short", "tall"];
const SIDE_STATES: [BlockStateDefinition; 4] = [PALE_MOSS_CARPET_SIDE_NORTH, PALE_MOSS_CARPET_SIDE_EAST, PALE_MOSS_CARPET_SIDE_SOUTH, PALE_MOSS_CARPET_SIDE_WEST];

impl Carpet {
    fn index(self) -> usize {
        self.sides.iter().fold(self.base as usize, |index, side| index * 3 + *side as usize)
    }

    fn from_index(mut index: usize) -> Self {
        let mut sides = [Side::None; 4];
        for side in sides.iter_mut().rev() {
            *side = [Side::None, Side::Low, Side::Tall][index % 3];
            index /= 3;
        }
        Self { base: index == 1, sides }
    }

    fn has_faces(self) -> bool {
        self.base || self.sides.iter().any(|&side| side != Side::None)
    }
}

pub struct MossCarpet {
    pub kind: u16,
    states: Vec<BlockId>,
}

impl MossCarpet {
    pub fn register(table: &mut BlockTable) -> Self {
        let states = (0..162)
            .map(|index| {
                let carpet = Carpet::from_index(index);
                let mut states = vec![state(UPPER_BLOCK_BIT, flag(!carpet.base))];
                for (definition, side) in SIDE_STATES.into_iter().zip(carpet.sides) {
                    states.push(state(definition, text(SIDE_NAMES[side as usize])));
                }
                table.get(PALE_MOSS_CARPET, &states)
            })
            .collect();
        Self {
            kind: table.kind(PALE_MOSS_CARPET),
            states,
        }
    }

    fn decode(&self, state: BlockId) -> Option<Carpet> {
        self.states.iter().position(|&id| id == state).map(Carpet::from_index)
    }

    fn state(&self, carpet: Carpet) -> BlockId {
        self.states[carpet.index()]
    }

    fn updated(&self, region: &(impl BlockView + ?Sized), mut carpet: Carpet, pos: IVec3, create_sides: bool) -> Carpet {
        let create_sides = create_sides || carpet.base;
        for (index, direction) in Direction::HORIZONTAL.into_iter().enumerate() {
            let supported = region.blocks().entry(region.get(pos + direction.offset())).full;
            let mut side = if supported { if create_sides { Side::Low } else { carpet.sides[index] } } else { Side::None };
            if side == Side::Low {
                if let Some(above) = self.decode(region.get(pos + IVec3::Y))
                    && above.sides[index] != Side::None
                    && !above.base
                {
                    side = Side::Tall;
                }
                if !carpet.base
                    && let Some(below) = self.decode(region.get(pos - IVec3::Y))
                    && below.sides[index] == Side::None
                {
                    side = Side::None;
                }
            }
            carpet.sides[index] = side;
        }
        carpet
    }

    fn topper(&self, region: &Region, random: &mut dyn RandomSource, pos: IVec3) -> Option<Carpet> {
        let above = pos + IVec3::Y;
        let previous = region.get(above);
        let previous_carpet = self.decode(previous);
        let replaceable = region.blocks.is_air(previous) || region.blocks.is(previous, Tag::Replaceable);
        if previous_carpet.is_some_and(|carpet| carpet.base) || (previous_carpet.is_none() && !replaceable) {
            return None;
        }
        let mut carpet = self.updated(region, Carpet { base: false, sides: [Side::None; 4] }, above, true);
        for side in carpet.sides.iter_mut() {
            if *side != Side::None && !random.next_boolean() {
                *side = Side::None;
            }
        }
        (carpet.has_faces() && previous_carpet != Some(carpet)).then_some(carpet)
    }

    pub fn update_shape(&self, view: &(impl BlockView + ?Sized), state: BlockId, pos: IVec3) -> BlockId {
        if !can_survive(view, state, pos) {
            return AIR;
        }
        let Some(carpet) = self.decode(state) else { return state };
        let updated = self.updated(view, carpet, pos, false);
        if updated.has_faces() { self.state(updated) } else { AIR }
    }

    pub fn place(&self, region: &mut Region, random: &mut dyn RandomSource, pos: IVec3) {
        let adjusted = self.updated(region, Carpet { base: true, sides: [Side::None; 4] }, pos, true);
        region.set(pos, self.state(adjusted));
        if let Some(topper) = self.topper(region, random, pos) {
            region.set(pos + IVec3::Y, self.state(topper));
            let bottom = self.updated(region, adjusted, pos, true);
            region.set(pos, self.state(bottom));
        }
    }
}
