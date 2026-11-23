use super::super::{NetherBlocks, NetherChunk};
use super::CHUNK_WIDTH;
use crate::bedrock::overworld::expression::Host;
use crate::bedrock::overworld::ore::Cells;
use crate::shared::CHUNK_MASK;
use crate::shared::blocks::{BlockId, Blocks};
use crate::shared::chunk::proto::RoutedTick;
use crate::shared::decoration_window::{SLOTS, SourceChanges, Window};
use chorus_block::block_id::{BROWN_MUSHROOM, FIRE, LAVA, RED_MUSHROOM, SOUL_FIRE};
use glam::IVec3;
use std::sync::Arc;

pub struct View<'a> {
    window: Window<'a>,
    heights: &'a [Arc<[i32; 256]>; SLOTS],
    blocks: &'a Blocks,
    ids: &'a NetherBlocks,
}

fn named(blocks: &Blocks, id: BlockId) -> &str {
    blocks.name_of(id)
}

fn lava_block(blocks: &Blocks, id: BlockId) -> bool {
    named(blocks, id) == LAVA
}

pub fn thin(blocks: &Blocks, id: BlockId) -> bool {
    matches!(
        named(blocks, id),
        chorus_block::block_id::AIR
            | chorus_block::block_id::CRIMSON_ROOTS
            | chorus_block::block_id::WARPED_ROOTS
            | chorus_block::block_id::NETHER_SPROUTS
            | chorus_block::block_id::CRIMSON_FUNGUS
            | chorus_block::block_id::WARPED_FUNGUS
            | chorus_block::block_id::WEEPING_VINES
            | chorus_block::block_id::TWISTING_VINES
            | BROWN_MUSHROOM
            | RED_MUSHROOM
            | FIRE
            | SOUL_FIRE
    )
}

pub fn solid(blocks: &Blocks, id: BlockId) -> bool {
    !blocks.is_air(id) && !lava_block(blocks, id) && !blocks.entry(id).liquid && !thin(blocks, id)
}

pub fn opaque_solid(blocks: &Blocks, id: BlockId) -> bool {
    solid(blocks, id) && blocks.entry(id).cube
}

pub(super) fn opaque_to_light(blocks: &Blocks, id: BlockId) -> bool {
    !blocks.is_air(id) && !blocks.entry(id).liquid && !thin(blocks, id)
}

pub fn light_heights(blocks: &Blocks, chunk: &NetherChunk) -> [i32; 256] {
    let mut heights = [0i32; 256];
    for x in 0..CHUNK_WIDTH {
        for z in 0..CHUNK_WIDTH {
            heights[(z * CHUNK_WIDTH + x) as usize] = (chunk.min_y..=chunk.max_y())
                .rev()
                .find(|&y| opaque_to_light(blocks, chunk.get(x, y, z)))
                .map_or(chunk.min_y, |y| y + 1);
        }
    }
    heights
}

impl<'a> View<'a> {
    pub fn new(chunks: &'a mut [NetherChunk; SLOTS], heights: &'a [Arc<[i32; 256]>; SLOTS], blocks: &'a Blocks, ids: &'a NetherBlocks, source: (i32, i32)) -> Self {
        Self {
            window: Window::new(chunks, source),
            heights,
            blocks,
            ids,
        }
    }

    pub fn get(&self, x: i32, y: i32, z: i32) -> BlockId {
        self.window.get(x, y, z)
    }

    pub fn set(&mut self, x: i32, y: i32, z: i32, block: BlockId) -> bool {
        self.window.set_changed(x, y, z, block)
    }

    pub fn take_changes(&mut self) -> SourceChanges {
        self.window.take_changes()
    }

    pub fn schedule(&mut self, pos: [i32; 3], lava: bool, delay: u64) {
        self.window.route_tick(RoutedTick {
            pos: IVec3::from_array(pos),
            lava,
            delay,
        });
    }

    pub fn heightmap(&self, x: i32, z: i32) -> i32 {
        let Some(slot) = self.window.slot(x, z) else { return 0 };
        self.heights[slot][((z & CHUNK_MASK) * CHUNK_WIDTH + (x & CHUNK_MASK)) as usize]
    }

    pub fn blocks(&self) -> &'a Blocks {
        self.blocks
    }

    pub fn ids(&self) -> &'a NetherBlocks {
        self.ids
    }
}

impl Host for View<'_> {
    fn heightmap(&mut self, x: i32, z: i32) -> i32 {
        View::heightmap(self, x, z)
    }

    fn above_top_solid(&mut self, _x: i32, _z: i32) -> i32 {
        0
    }
}

impl Cells for View<'_> {
    fn block(&self, pos: IVec3) -> BlockId {
        self.get(pos.x, pos.y, pos.z)
    }

    fn place(&mut self, pos: IVec3, block: BlockId) {
        self.set(pos.x, pos.y, pos.z, block);
    }

    fn kind(&self, block: BlockId) -> u16 {
        self.blocks.entry(block).kind
    }

    fn is_air(&self, block: BlockId) -> bool {
        self.blocks.is_air(block)
    }

    fn min_y(&self) -> i32 {
        self.window.min_y()
    }

    fn max_y(&self) -> i32 {
        self.window.max_y()
    }
}
