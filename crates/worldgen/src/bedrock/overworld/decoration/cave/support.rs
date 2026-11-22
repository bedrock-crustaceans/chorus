use super::super::world::read;
use crate::shared::blocks::{BlockId, Blocks};
use crate::shared::chunk::region::Region;
use crate::shared::survival::{Fluid, fluid_at};
use crate::shared::tags::Tag;
use chorus_block::block_id::{LAVA, WATER};
use glam::IVec3;

pub fn is_water_source(blocks: &Blocks, block: BlockId) -> bool {
    blocks.name_of(block) == WATER
}

pub fn is_open(blocks: &Blocks, block: BlockId) -> bool {
    blocks.is_air(block) || is_water_source(blocks, block)
}

pub fn is_wet(blocks: &Blocks, block: BlockId) -> bool {
    fluid_at(blocks, block) == Some(Fluid::Water)
}

pub fn holds_face(blocks: &Blocks, block: BlockId) -> bool {
    blocks.entry(block).full && !blocks.is(block, Tag::Leaves)
}

pub fn holds_face_at(region: &Region, pos: IVec3) -> bool {
    holds_face(region.blocks, read(region, pos))
}

pub fn holds_vine(blocks: &Blocks, block: BlockId) -> bool {
    blocks.entry(block).vine_support
}

pub fn is_lava(blocks: &Blocks, block: BlockId) -> bool {
    blocks.name_of(block) == LAVA
}

pub fn is_open_or_wet(blocks: &Blocks, block: BlockId) -> bool {
    blocks.is_air(block) || is_wet(blocks, block)
}
