use super::list::BlockList;
use crate::shared::blocks::{AIR, BlockId, BlockTable, Blocks, int, state, text};
use crate::shared::chunk::region::{Heightmap, Region};
use crate::shared::survival::{Fluid, fluid_at};
use crate::shared::tags::Tag;
use chorus_block::block_id::*;
use chorus_block::state::common::{PILLAR_AXIS, VINE_DIRECTION_BITS};
use glam::IVec3;

pub const SIDES: [IVec3; 4] = [IVec3::new(-1, 0, 0), IVec3::new(1, 0, 0), IVec3::new(0, 0, -1), IVec3::new(0, 0, 1)];

const REPLACEABLE_PLANTS: [&str; 11] = [SHORT_GRASS, FERN, TALL_GRASS, LARGE_FERN, DEADBUSH, VINE, SHORT_DRY_GRASS, TALL_DRY_GRASS, BUSH, SEAGRASS, LEAF_LITTER];
const VINE_BITS_BY_SIDE: [i32; 4] = [8, 2, 1, 4];

#[derive(Debug)]
pub struct Foliage {
    plants: BlockList,
    vines: [BlockId; 4],
}

impl Foliage {
    pub fn new(table: &mut BlockTable) -> Self {
        for waterlogging in [MANGROVE_LEAVES, MANGROVE_ROOTS] {
            let dry = table.name(waterlogging);
            table.flooded(dry);
        }
        Self {
            plants: BlockList::from_names(table, &REPLACEABLE_PLANTS),
            vines: VINE_BITS_BY_SIDE.map(|bits| table.get(VINE, &[state(&VINE_DIRECTION_BITS, int(bits))])),
        }
    }

    pub fn vine_on(&self, side: usize) -> BlockId {
        self.vines[side]
    }

    pub fn is_plant(&self, blocks: &Blocks, block: BlockId) -> bool {
        self.plants.contains(blocks, block)
    }

    pub fn leaf_replaceable(&self, blocks: &Blocks, block: BlockId) -> bool {
        blocks.is_air(block) || blocks.is(block, Tag::Leaves) || self.is_plant(blocks, block)
    }
}

pub fn block(region: &Region, pos: IVec3) -> BlockId {
    region.get_contained(pos).unwrap_or(AIR)
}

pub fn ground(region: &mut Region, x: i32, z: i32) -> i32 {
    region.height(Heightmap::WorldSurfaceWg, x, z)
}

pub fn is_air(region: &Region, pos: IVec3) -> bool {
    region.blocks.is_air(block(region, pos))
}

pub fn put(region: &mut Region, pos: IVec3, value: BlockId) -> bool {
    let Some(seen) = region.get_contained(pos) else { return false };
    let value = if fluid_at(region.blocks, seen) == Some(Fluid::Water) {
        region.blocks.flooded(value)
    } else {
        value
    };
    if seen == value {
        return false;
    }
    region.set(pos, value)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Log {
    pub upright: BlockId,
    pub along_x: BlockId,
    pub along_z: BlockId,
}

impl Log {
    pub fn new(table: &mut BlockTable, name: &'static str) -> Self {
        let axis = |table: &mut BlockTable, value: &'static str| table.get(name, &[state(&PILLAR_AXIS, text(value))]);
        Self {
            upright: axis(table, "y"),
            along_x: axis(table, "x"),
            along_z: axis(table, "z"),
        }
    }

    pub fn along(&self, dx: i32, dz: i32) -> BlockId {
        match (dx.abs(), dz.abs()) {
            (0, 0) => self.upright,
            (ax, az) if ax >= az => self.along_x,
            _ => self.along_z,
        }
    }
}
