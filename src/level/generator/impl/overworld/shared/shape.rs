use glam::IVec3;

use super::blocks::{self, AIR, BlockId, Blocks, int};
use super::moss_carpet::MossCarpet;
use super::region::BlockView;
use super::survival::{Direction, can_survive, from_legacy_direction, multiface_bit, vine_bit};
use super::tags::Tag;
use crate::block::block_id::*;
use crate::block::state::common::{DIRECTION, MULTI_FACE_DIRECTION_BITS, VINE_DIRECTION_BITS};

fn broken(blocks: &Blocks, state: BlockId, water: BlockId) -> BlockId {
    if blocks.entry(state).flooded { water } else { AIR }
}

fn vine_faces(blocks: &Blocks, vine: BlockId) -> Vec<Direction> {
    let bits = blocks.int(vine, VINE_DIRECTION_BITS).unwrap_or(0);
    if bits == 0 {
        return vec![Direction::Up];
    }
    Direction::HORIZONTAL.into_iter().filter(|&face| bits & vine_bit(face) != 0).collect()
}

fn half(blocks: &Blocks, state: BlockId) -> Option<bool> {
    blocks.entry(state).upper
}

pub struct Shapes<'a> {
    pub carpet: &'a MossCarpet,
    pub water: BlockId,
}

impl Shapes<'_> {
    pub fn update(&self, view: &(impl BlockView + ?Sized), pos: IVec3, state: BlockId, direction: Direction) -> BlockId {
        let blocks = view.blocks();
        if blocks.is_air(state) || blocks.entry(state).liquid {
            return state;
        }
        let neighbour = view.get(pos + direction.offset());
        if blocks.is_kind(state, self.carpet.kind) {
            return self.carpet.update_shape(view, state, pos);
        }
        if let Some(upper) = half(blocks, state) {
            let vertical = matches!(direction, Direction::Up | Direction::Down);
            if vertical && upper != (direction == Direction::Up) {
                let matching = blocks.same_kind(neighbour, state) && half(blocks, neighbour) != Some(upper);
                return if matching { state } else { broken(blocks, state, self.water) };
            }
            return if can_survive(view, state, pos) { state } else { broken(blocks, state, self.water) };
        }
        match blocks.name_of(state) {
            VINE => {
                if direction == Direction::Down {
                    return state;
                }
                let above = view.get(pos + IVec3::Y);
                let supported = vine_faces(blocks, state).into_iter().all(|face| {
                    let attached = blocks.entry(view.get(pos + face.offset())).full;
                    attached || (face != Direction::Up && blocks.same_kind(above, state) && vine_faces(blocks, above).contains(&face))
                });
                if supported { state } else { AIR }
            }
            GLOW_LICHEN | SCULK_VEIN => {
                let bits = blocks.int(state, MULTI_FACE_DIRECTION_BITS).unwrap_or(0);
                let bit = multiface_bit(direction);
                if bits & bit == 0 || blocks.entry(neighbour).full {
                    return state;
                }
                let remaining = bits & !bit;
                if remaining == 0 {
                    AIR
                } else {
                    blocks.with(state, blocks::state(MULTI_FACE_DIRECTION_BITS, int(remaining)))
                }
            }
            COCOA => {
                let facing = from_legacy_direction(blocks.int(state, DIRECTION).unwrap_or(0));
                if facing == direction && !blocks.is(neighbour, Tag::Logs) { AIR } else { state }
            }
            _ => {
                if can_survive(view, state, pos) {
                    state
                } else {
                    broken(blocks, state, self.water)
                }
            }
        }
    }

    pub fn update_from_neighbours(&self, view: &(impl BlockView + ?Sized), pos: IVec3) -> BlockId {
        let mut state = view.get(pos);
        for direction in [Direction::West, Direction::East, Direction::North, Direction::South, Direction::Down, Direction::Up] {
            state = self.update(view, pos, state, direction);
        }
        state
    }
}
