use super::blocks::MineshaftBlocks;
use crate::shared::blocks::{BlockId, Blocks};
use crate::shared::chunk::proto::ProtoChunk;
use crate::shared::structure::{BoundingBox, Probed, StructureBlocks, is_falling_block};
use crate::shared::{CHUNK_MASK, CHUNK_SHIFT};
use chorus_level::dimension_definition::DimensionDefinition;
use rustc_hash::FxHashSet;

pub type Cell = (i32, i32, i32);

const DEEPSLATE_BELOW: i32 = 0;

pub struct Canvas<'a> {
    pub chunk: &'a mut ProtoChunk,
    pub blocks: &'a Blocks,
    pub ids: &'a StructureBlocks,
    pub shaft: &'a MineshaftBlocks,
    pub bounds: BoundingBox,
    pub definition: DimensionDefinition,
    pub fences: FxHashSet<Cell>,
    columns: &'a dyn Fn(i32, i32) -> Probed<(i32, i32)>,
    world: &'a dyn Fn(i32, i32, i32) -> Probed<Option<BlockId>>,
    chunk_x: i32,
    chunk_z: i32,
}

impl<'a> Canvas<'a> {
    pub fn new(
        chunk: &'a mut ProtoChunk,
        blocks: &'a Blocks,
        ids: &'a StructureBlocks,
        columns: &'a dyn Fn(i32, i32) -> Probed<(i32, i32)>,
        world: &'a dyn Fn(i32, i32, i32) -> Probed<Option<BlockId>>,
    ) -> Self {
        let (chunk_x, chunk_z) = (chunk.min_x() >> CHUNK_SHIFT, chunk.min_z() >> CHUNK_SHIFT);
        let definition = chunk.definition();
        Self {
            blocks,
            ids,
            shaft: &ids.mineshaft,
            bounds: BoundingBox::chunk(chunk_x, chunk_z, definition),
            definition,
            fences: FxHashSet::default(),
            columns,
            world,
            chunk_x,
            chunk_z,
            chunk,
        }
    }

    fn inside(&self, x: i32, z: i32) -> bool {
        x >> CHUNK_SHIFT == self.chunk_x && z >> CHUNK_SHIFT == self.chunk_z
    }

    fn in_view(&self, x: i32, z: i32) -> bool {
        ((x >> CHUNK_SHIFT) - self.chunk_x).abs() <= 1 && ((z >> CHUNK_SHIFT) - self.chunk_z).abs() <= 1
    }

    fn ground(&self, x: i32, y: i32, z: i32) -> Probed<BlockId> {
        let (solid, top) = (self.columns)(x, z)?;
        Ok(if y < solid {
            if y < DEEPSLATE_BELOW { self.ids.deepslate } else { self.ids.stone }
        } else if y < top {
            self.ids.water
        } else {
            self.ids.air
        })
    }

    pub fn get(&self, x: i32, y: i32, z: i32) -> Probed<BlockId> {
        if !(self.definition.min_y..self.definition.top()).contains(&y) || !self.in_view(x, z) {
            Ok(self.ids.air)
        } else if self.inside(x, z) {
            Ok(self.chunk.get(x & CHUNK_MASK, y, z & CHUNK_MASK))
        } else {
            match (self.world)(x, y, z)? {
                Some(block) => Ok(block),
                None => self.ground(x, y, z),
            }
        }
    }

    pub fn get_clipped(&self, x: i32, y: i32, z: i32) -> Probed<BlockId> {
        if self.bounds.contains(x, y, z) { self.get(x, y, z) } else { Ok(self.ids.air) }
    }

    pub fn set(&mut self, x: i32, y: i32, z: i32, block: BlockId) {
        if !self.bounds.contains(x, y, z) {
            return;
        }
        let (lx, lz) = (x & CHUNK_MASK, z & CHUNK_MASK);
        let block = if self.chunk.get(lx, y, lz) == self.ids.water { self.blocks.flooded(block) } else { block };
        self.chunk.set(lx, y, lz, block);
    }

    fn height(&self, x: i32, z: i32) -> Probed<i32> {
        if self.inside(x, z) {
            Ok(self.chunk.top(x & CHUNK_MASK, z & CHUNK_MASK))
        } else {
            Ok((self.columns)(x, z)?.1)
        }
    }

    pub fn interior(&self, x: i32, y: i32, z: i32) -> Probed<bool> {
        if y >= self.definition.top() {
            return Ok(false);
        }
        if y < self.definition.min_y {
            return Ok(true);
        }
        Ok(self.in_view(x, z) && self.height(x, z)? > y)
    }

    pub fn is_air(&self, block: BlockId) -> bool {
        self.blocks.is_air(block)
    }

    pub fn is_fluid(&self, block: BlockId) -> bool {
        self.blocks.entry(block).liquid
    }

    pub fn full_solid(&self, block: BlockId) -> bool {
        let entry = self.blocks.entry(block);
        entry.collision_cube && entry.name.as_ref() != "minecraft:mob_spawner"
    }

    pub fn falls(&self, block: BlockId) -> bool {
        is_falling_block(self.blocks.entry(block).name.as_ref())
    }

    pub fn replaceable(&self, block: BlockId) -> bool {
        let entry = self.blocks.entry(block);
        self.is_air(block) || entry.liquid || !entry.solid && entry.name.as_ref() != "minecraft:web" || entry.name.as_ref() == "minecraft:snow_layer"
    }
}
