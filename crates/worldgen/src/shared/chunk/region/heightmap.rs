use super::{Region, SPAN};
use crate::shared::blocks::{BlockId, Blocks};
use crate::shared::tags::Tag;
use chorus_block::block_id::SNOW_LAYER;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Heightmap {
    WorldSurface,
    WorldSurfaceWg,
    OceanFloor,
    OceanFloorWg,
    MotionBlocking,
    MotionBlockingNoLeaves,
    AboveTopSolid,
}

impl Heightmap {
    pub(super) const TRACKED: [Heightmap; 5] = [Self::WorldSurface, Self::OceanFloor, Self::MotionBlocking, Self::MotionBlockingNoLeaves, Self::AboveTopSolid];

    pub(super) fn index(self) -> usize {
        match self {
            Self::WorldSurface | Self::WorldSurfaceWg => 0,
            Self::OceanFloor | Self::OceanFloorWg => 1,
            Self::MotionBlocking => 2,
            Self::MotionBlockingNoLeaves => 3,
            Self::AboveTopSolid => 4,
        }
    }

    pub(super) fn matches(self, blocks: &Blocks, block: BlockId) -> bool {
        let entry = blocks.entry(block);
        match self {
            Self::WorldSurface | Self::WorldSurfaceWg => !blocks.is_air(block),
            Self::OceanFloor | Self::OceanFloorWg => entry.solid,
            Self::MotionBlocking => entry.solid || entry.liquid || entry.flooded,
            Self::MotionBlockingNoLeaves => (entry.solid || entry.liquid || entry.flooded) && !blocks.is(block, Tag::Leaves),
            Self::AboveTopSolid => entry.solid && entry.name.as_ref() != SNOW_LAYER && !blocks.is(block, Tag::Leaves),
        }
    }
}

impl Region<'_> {
    pub(super) fn column_index(&self, x: i32, z: i32) -> usize {
        let (rx, rz) = (x - (self.owner_x << 4) + 16, z - (self.owner_z << 4) + 16);
        (rx.clamp(0, SPAN as i32 - 1) as usize) * SPAN + rz.clamp(0, SPAN as i32 - 1) as usize
    }

    pub fn highest_block(&mut self, x: i32, z: i32) -> Option<i32> {
        let slot = self.slot(x, z)?;
        if let Some(top) = self.highest[slot] {
            return Some(top);
        }
        let (cx, cz) = ((x >> 4) << 4, (z >> 4) << 4);
        let built = (0..16)
            .flat_map(|lx| (0..16).map(move |lz| (lx, lz)))
            .filter_map(|(lx, lz)| self.chunks[slot].worldgen_height(false, lx, lz))
            .max()
            .unwrap_or(self.min_y());
        let mut top = built;
        for lx in 0..16 {
            for lz in 0..16 {
                top = top.max(self.height(Heightmap::WorldSurface, cx + lx, cz + lz) - 1);
            }
        }
        self.highest[slot] = Some(top);
        Some(top)
    }

    pub fn height(&mut self, kind: Heightmap, x: i32, z: i32) -> i32 {
        let Some(slot) = self.slot(x, z) else { return self.min_y() };
        if let (Heightmap::WorldSurfaceWg | Heightmap::OceanFloorWg, Some(height)) = (kind, self.chunks[slot].worldgen_height(kind == Heightmap::OceanFloorWg, x & 15, z & 15)) {
            return height;
        }
        let index = kind.index() * SPAN * SPAN + self.column_index(x, z);
        if let Some(height) = self.heights[index] {
            return height;
        }
        let chunk = &self.chunks[slot];
        let (lx, lz) = (x & 15, z & 15);
        let mut y = chunk.top(lx, lz) - 1;
        let height = loop {
            if y < self.min_y() {
                break self.min_y();
            }
            if kind.matches(self.blocks, chunk.get(lx, y, lz)) {
                break y + 1;
            }
            y -= 1;
        };
        self.heights[index] = Some(height);
        height
    }
}
