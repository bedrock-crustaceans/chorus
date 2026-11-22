use super::super::liquid::TickQueue;
use super::super::{NetherBlocks, NetherChunk};
use super::{CHUNK_WIDTH, RoutedTick};
use crate::bedrock::overworld::expression::Host;
use crate::bedrock::overworld::ore::Cells;
use crate::shared::blocks::{BlockId, Blocks};
use crate::shared::decoration_window::{SLOTS, Window, Writes};
use crate::shared::{CHUNK_MASK, CHUNK_SHIFT};
use chorus_block::block_id::{BROWN_MUSHROOM, FIRE, LAVA, RED_MUSHROOM, SOUL_FIRE};
use glam::IVec3;
use std::sync::Arc;

const TICK_REACH: i32 = 8;

const ROUTED_TICK_NOW: u64 = 0;

pub struct View<'a> {
    window: Window<'a>,
    heights: &'a [Arc<[i32; 256]>; SLOTS],
    blocks: &'a Blocks,
    ids: &'a NetherBlocks,
    pub ticks: TickQueue,
    pub routed: Vec<RoutedTick>,
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
            heights[(z * CHUNK_WIDTH + x) as usize] = (chunk.min_y()..=chunk.max_y())
                .rev()
                .find(|&y| opaque_to_light(blocks, chunk.get(x, y, z)))
                .map_or(chunk.min_y(), |y| y + 1);
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
            ticks: TickQueue::chunk(),
            routed: Vec::new(),
        }
    }

    pub fn get(&self, x: i32, y: i32, z: i32) -> BlockId {
        self.window.get(x, y, z)
    }

    pub fn set(&mut self, x: i32, y: i32, z: i32, block: BlockId) -> bool {
        self.window.set_changed(x, y, z, block)
    }

    pub fn has_block(&self, x: i32, y: i32, z: i32) -> bool {
        self.window.has_block(x, y, z)
    }

    pub fn take_writes(&mut self) -> Writes {
        self.window.take_writes()
    }

    pub fn can_tick(&self, x: i32, z: i32) -> bool {
        [-1, 1].iter().all(|dx| [-1, 1].iter().all(|dz| self.window.slot(x + dx, z + dz).is_some()))
    }

    pub fn queue_reaches(&self, x: i32, z: i32) -> bool {
        [-TICK_REACH, TICK_REACH]
            .iter()
            .all(|dx| [-TICK_REACH, TICK_REACH].iter().all(|dz| self.window.slot(x + dx, z + dz).is_some()))
    }

    pub fn schedule(&mut self, pos: [i32; 3], lava: bool, delay: u64) {
        if self.queue_reaches(pos[0], pos[2]) {
            self.ticks.add(pos, lava, delay);
        } else {
            self.routed.push(RoutedTick {
                target: (pos[0] >> CHUNK_SHIFT, pos[2] >> CHUNK_SHIFT),
                pos,
                lava,
                delay: ROUTED_TICK_NOW,
            });
        }
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

    pub fn lava_level(&self, depth: u8) -> BlockId {
        self.ids.lava_levels[depth as usize]
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
