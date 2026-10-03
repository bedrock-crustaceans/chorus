use glam::IVec3;

use super::biome::Biome;
use super::blocks::{AIR, BlockId, Blocks};
use super::proto::{BlockEntity, PlacedEntity, ProtoChunk};
use super::tags::Tag;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Heightmap {
    WorldSurface,
    WorldSurfaceWg,
    OceanFloor,
    OceanFloorWg,
    MotionBlocking,
    MotionBlockingNoLeaves,
}

impl Heightmap {
    const TRACKED: [Heightmap; 4] = [Self::WorldSurface, Self::OceanFloor, Self::MotionBlocking, Self::MotionBlockingNoLeaves];

    fn index(self) -> usize {
        match self {
            Self::WorldSurface | Self::WorldSurfaceWg => 0,
            Self::OceanFloor | Self::OceanFloorWg => 1,
            Self::MotionBlocking => 2,
            Self::MotionBlockingNoLeaves => 3,
        }
    }

    fn matches(self, blocks: &Blocks, block: BlockId) -> bool {
        let entry = blocks.entry(block);
        match self {
            Self::WorldSurface | Self::WorldSurfaceWg => !blocks.is_air(block),
            Self::OceanFloor | Self::OceanFloorWg => entry.solid,
            Self::MotionBlocking => entry.solid || entry.liquid || entry.flooded,
            Self::MotionBlockingNoLeaves => (entry.solid || entry.liquid || entry.flooded) && !blocks.is(block, Tag::Leaves),
        }
    }
}

const SPAN: usize = 48;
const CENTER: usize = 4;

pub fn slot(dx: i32, dz: i32) -> Option<usize> {
    ((-1..=1).contains(&dx) && (-1..=1).contains(&dz)).then_some(((dx + 1) * 3 + dz + 1) as usize)
}

pub struct Region<'a> {
    pub owner_x: i32,
    pub owner_z: i32,
    pub blocks: &'a Blocks,
    chunks: [ProtoChunk; 9],
    dirty: [Vec<(u8, i16, u8)>; 9],
    entities: [Vec<PlacedEntity>; 9],
    marks: [Vec<(u8, i16, u8)>; 9],
    record: Option<Vec<IVec3>>,
    heights: Vec<Option<i32>>,
}

pub struct Changes {
    owner_x: i32,
    owner_z: i32,
    changes: [Vec<(u8, i16, u8, BlockId)>; 9],
    entities: [Vec<PlacedEntity>; 9],
    marks: [Vec<(u8, i16, u8)>; 9],
}

pub trait BlockView {
    fn blocks(&self) -> &Blocks;
    fn get(&self, pos: IVec3) -> BlockId;
}

impl BlockView for Region<'_> {
    fn blocks(&self) -> &Blocks {
        self.blocks
    }

    fn get(&self, pos: IVec3) -> BlockId {
        Region::get(self, pos)
    }
}

impl<'a> Region<'a> {
    pub fn new(owner_x: i32, owner_z: i32, blocks: &'a Blocks, chunks: [ProtoChunk; 9]) -> Self {
        Self {
            owner_x,
            owner_z,
            blocks,
            chunks,
            dirty: Default::default(),
            entities: Default::default(),
            marks: Default::default(),
            record: None,
            heights: vec![None; 4 * SPAN * SPAN],
        }
    }

    pub fn min_y(&self) -> i32 {
        self.chunks[CENTER].min_y
    }

    pub fn max_y(&self) -> i32 {
        self.chunks[CENTER].max_y()
    }

    pub fn is_outside_build_height(&self, y: i32) -> bool {
        y < self.min_y() || y > self.max_y()
    }

    fn slot(&self, x: i32, z: i32) -> Option<usize> {
        slot((x >> 4) - self.owner_x, (z >> 4) - self.owner_z)
    }

    pub fn contains(&self, pos: IVec3) -> bool {
        self.slot(pos.x, pos.z).is_some() && !self.is_outside_build_height(pos.y)
    }

    pub fn get_contained(&self, pos: IVec3) -> Option<BlockId> {
        let slot = self.slot(pos.x, pos.z)?;
        (!self.is_outside_build_height(pos.y)).then(|| self.chunks[slot].get(pos.x & 15, pos.y, pos.z & 15))
    }

    pub fn get(&self, pos: IVec3) -> BlockId {
        match self.slot(pos.x, pos.z) {
            Some(slot) => self.chunks[slot].get(pos.x & 15, pos.y, pos.z & 15),
            None => AIR,
        }
    }

    pub fn set(&mut self, pos: IVec3, block: BlockId) -> bool {
        let Some(slot) = self.slot(pos.x, pos.z) else { return false };
        if self.is_outside_build_height(pos.y) {
            return false;
        }
        let (lx, lz) = (pos.x & 15, pos.z & 15);
        self.chunks[slot].set(lx, pos.y, lz, block);
        self.dirty[slot].push((lx as u8, pos.y as i16, lz as u8));
        if let Some(record) = &mut self.record {
            record.push(pos);
        }
        let column = self.column_index(pos.x, pos.z);
        for kind in Heightmap::TRACKED {
            let index = kind.index() * SPAN * SPAN + column;
            let Some(height) = self.heights[index] else { continue };
            if pos.y < height - 1 {
                continue;
            }
            if kind.matches(self.blocks, block) {
                if pos.y >= height {
                    self.heights[index] = Some(pos.y + 1);
                }
            } else if pos.y == height - 1 {
                self.heights[index] = None;
            }
        }
        true
    }

    pub fn water(&self) -> BlockId {
        self.chunks[CENTER].water
    }

    pub fn start_recording(&mut self) {
        self.record = Some(Vec::new());
    }

    pub fn resume_recording(&mut self, recorded: Vec<IVec3>) {
        self.record = Some(recorded);
    }

    pub fn take_recording(&mut self) -> Vec<IVec3> {
        self.record.take().unwrap_or_default()
    }

    pub fn mark_post_process(&mut self, pos: IVec3) {
        let Some(slot) = self.slot(pos.x, pos.z) else { return };
        if self.is_outside_build_height(pos.y) {
            return;
        }
        let local = ((pos.x & 15) as u8, pos.y as i16, (pos.z & 15) as u8);
        self.chunks[slot].mark_post_process(local.0, local.1, local.2);
        self.marks[slot].push(local);
    }

    pub fn mark_above(&mut self, pos: IVec3) {
        for i in 1..=2 {
            let above = pos + IVec3::Y * i;
            if self.blocks.is_air(self.get(above)) {
                return;
            }
            self.mark_post_process(above);
        }
    }

    pub fn set_block_entity(&mut self, pos: IVec3, entity: BlockEntity) {
        let Some(slot) = self.slot(pos.x, pos.z) else { return };
        if self.is_outside_build_height(pos.y) {
            return;
        }
        let placed = PlacedEntity {
            x: (pos.x & 15) as u8,
            y: pos.y as i16,
            z: (pos.z & 15) as u8,
            block: self.get(pos),
            entity,
        };
        self.chunks[slot].add_entity(placed.clone());
        self.entities[slot].push(placed);
    }

    pub fn biome(&self, pos: IVec3) -> Biome {
        match self.slot(pos.x, pos.z) {
            Some(slot) => self.chunks[slot].biome_at(pos.x & 15, pos.y, pos.z & 15),
            None => self.chunks[CENTER].biome_at(0, pos.y, 0),
        }
    }

    pub fn biomes(&self) -> Vec<Biome> {
        let mut out: Vec<Biome> = Vec::new();
        for chunk in &self.chunks {
            for biome in chunk.biomes() {
                if !out.contains(&biome) {
                    out.push(biome);
                }
            }
        }
        out
    }

    fn column_index(&self, x: i32, z: i32) -> usize {
        let (rx, rz) = (x - (self.owner_x << 4) + 16, z - (self.owner_z << 4) + 16);
        (rx.clamp(0, SPAN as i32 - 1) as usize) * SPAN + rz.clamp(0, SPAN as i32 - 1) as usize
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

    pub fn into_changes(self) -> Changes {
        let changes = std::array::from_fn(|slot| {
            let chunk = &self.chunks[slot];
            self.dirty[slot].iter().map(|&(x, y, z)| (x, y, z, chunk.get(x as i32, y as i32, z as i32))).collect()
        });
        Changes {
            owner_x: self.owner_x,
            owner_z: self.owner_z,
            changes,
            entities: self.entities,
            marks: self.marks,
        }
    }
}

impl Changes {
    pub fn placed(&self, chunk_x: i32, chunk_z: i32) -> &[(u8, i16, u8, BlockId)] {
        slot(chunk_x - self.owner_x, chunk_z - self.owner_z).map_or(&[], |index| &self.changes[index])
    }

    pub fn overlay_onto(&self, chunk_x: i32, chunk_z: i32, baseline: &ProtoChunk, target: &mut ProtoChunk) {
        let Some(index) = slot(chunk_x - self.owner_x, chunk_z - self.owner_z) else { return };
        for &(x, y, z, block) in &self.changes[index] {
            let (x, y, z) = (x as i32, y as i32, z as i32);
            if block != baseline.get(x, y, z) {
                target.set(x, y, z, block);
            }
        }
        for entity in &self.entities[index] {
            target.add_entity(entity.clone());
        }
        for &(x, y, z) in &self.marks[index] {
            target.mark_post_process(x, y, z);
        }
    }
}
