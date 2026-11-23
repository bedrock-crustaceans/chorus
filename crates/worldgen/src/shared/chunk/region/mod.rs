mod changes;
mod heightmap;
mod merge;
mod radix;
mod rules;
mod ticks;

pub use changes::{Changes, Write};
pub use heightmap::Heightmap;
pub use merge::{merge, replay_ordered};
pub use rules::RuleKind;
pub use ticks::FluidSchedule;

use super::proto::{BlockEntity, PlacedEntity, ProtoChunk, RoutedTick};
use super::scratch::Scratch;
use crate::shared::SEA_LEVEL;
use crate::shared::biome::Biome;
use crate::shared::blocks::{AIR, BlockId, Blocks};
use crate::shared::decoration_window::slot_at;
use crate::shared::structure::Probed;
use glam::IVec3;
use rules::NO_RULE;

const SPAN: usize = 48;
const CENTER: usize = 4;

thread_local! {
    static SPARE_DIRTY: std::cell::RefCell<Vec<Vec<(u8, i16, u8)>>> = const { std::cell::RefCell::new(Vec::new()) };
    static SPARE_HEIGHTS: std::cell::RefCell<Vec<Option<i32>>> = const { std::cell::RefCell::new(Vec::new()) };
}

pub struct Region<'a> {
    pub owner_x: i32,
    pub owner_z: i32,
    pub blocks: &'a Blocks,
    chunks: [ProtoChunk; 9],
    dirty: [Vec<(u8, i16, u8)>; 9],
    entities: [Vec<PlacedEntity>; 9],
    marks: [Vec<(u8, i16, u8)>; 9],
    fluid_schedule: Vec<FluidSchedule>,
    routed: [Vec<RoutedTick>; 9],
    heights: Scratch<Option<i32>>,
    highest: [Option<i32>; 9],
    top_block: Option<i32>,
    preliminary: Option<&'a (dyn Fn(i32, i32) -> i32 + Sync)>,
    structure_check: Option<&'a (dyn Fn(IVec3) -> Probed<bool> + Sync)>,
    partial_template: Option<&'a (dyn Fn(BlockId) -> bool + Sync)>,
    writes: [Vec<Write>; 9],
    rules: Vec<(Box<[u16]>, RuleKind)>,
    rule: u16,
    replay: bool,
    sea_level: i32,
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
            dirty: std::array::from_fn(|_| SPARE_DIRTY.try_with(|spare| spare.borrow_mut().pop()).ok().flatten().unwrap_or_default()),
            entities: Default::default(),
            marks: Default::default(),
            fluid_schedule: Vec::new(),
            routed: Default::default(),
            heights: Scratch::take(&SPARE_HEIGHTS, Heightmap::TRACKED.len() * SPAN * SPAN, None),
            highest: [None; 9],
            top_block: None,
            preliminary: None,
            structure_check: None,
            partial_template: None,
            writes: std::array::from_fn(|_| Vec::new()),
            rules: Vec::new(),
            rule: NO_RULE,
            replay: false,
            sea_level: SEA_LEVEL,
        }
    }

    pub fn top_block_y(&mut self) -> i32 {
        if let Some(top) = self.top_block {
            return top;
        }
        let top = self
            .chunks
            .iter()
            .map(|chunk| chunk.highest_block_y())
            .try_fold(i32::MIN, |top, found| found.map(|found| top.max(found)))
            .unwrap_or(i32::MAX);
        self.top_block = Some(top);
        top
    }

    pub fn with_sea_level(mut self, sea_level: i32) -> Self {
        self.sea_level = sea_level;
        self
    }

    pub fn sea_level(&self) -> i32 {
        self.sea_level
    }

    pub fn with_replay(mut self) -> Self {
        self.replay = true;
        self
    }

    pub fn with_partial_templates(mut self, partial: &'a (dyn Fn(BlockId) -> bool + Sync)) -> Self {
        self.partial_template = Some(partial);
        self
    }

    pub fn is_partial_template(&self, block: BlockId) -> bool {
        self.partial_template.is_some_and(|partial| partial(block))
    }

    pub fn with_structure_check(mut self, check: &'a (dyn Fn(IVec3) -> Probed<bool> + Sync)) -> Self {
        self.structure_check = Some(check);
        self
    }

    pub fn in_structure(&self, pos: IVec3) -> Probed<bool> {
        match self.structure_check {
            Some(check) => check(pos),
            None => Ok(false),
        }
    }

    pub fn with_preliminary_surface(mut self, surface: &'a (dyn Fn(i32, i32) -> i32 + Sync)) -> Self {
        self.preliminary = Some(surface);
        self
    }

    pub fn preliminary_surface(&self, quart_x: i32, quart_z: i32) -> Option<i32> {
        self.preliminary.map(|surface| surface(quart_x, quart_z))
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
        slot_at((x >> 4) - self.owner_x, (z >> 4) - self.owner_z)
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
        self.set_with_rule(pos, block, self.rule)
    }

    fn set_with_rule(&mut self, pos: IVec3, block: BlockId, rule: u16) -> bool {
        let Some(slot) = self.slot(pos.x, pos.z) else { return false };
        if self.is_outside_build_height(pos.y) {
            return false;
        }
        let (lx, lz) = (pos.x & 15, pos.z & 15);
        if self.replay {
            let seen = self.chunks[slot].get(lx, pos.y, lz);
            if seen != block {
                self.writes[slot].push(Write {
                    x: lx as u8,
                    y: pos.y as i16,
                    z: lz as u8,
                    block,
                    seen,
                    rule,
                });
                self.chunks[slot].set(lx, pos.y, lz, block);
            }
        } else {
            self.chunks[slot].set(lx, pos.y, lz, block);
        }
        self.dirty[slot].push((lx as u8, pos.y as i16, lz as u8));
        if !self.blocks.is_air(block) {
            if let Some(top) = &mut self.highest[slot] {
                *top = (*top).max(pos.y);
            }
            if let Some(top) = &mut self.top_block {
                *top = (*top).max(pos.y);
            }
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

    pub fn has_column(&self, x: i32, z: i32) -> bool {
        self.slot(x, z).is_some()
    }

    pub fn settle_structure_spills(&mut self) {
        for (x, y, z, block) in self.chunks[CENTER].structure_spills().to_vec() {
            let pos = IVec3::new(x, y, z);
            if self.get_contained(pos) == Some(AIR) {
                self.set(pos, block);
            }
        }
    }

    pub fn water(&self) -> BlockId {
        self.chunks[CENTER].water
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

    pub fn biome_id(&self, pos: IVec3) -> i32 {
        match self.slot(pos.x, pos.z) {
            Some(slot) => self.chunks[slot].biome_id_at(pos.x & 15, pos.y, pos.z & 15),
            None => self.chunks[CENTER].biome_id_at(0, pos.y, 0),
        }
    }

    pub fn uniform_biome_at(&self, y: i32) -> Option<Biome> {
        self.chunks[CENTER].uniform_biome_at(y)
    }

    pub fn uniform_biome_id_at(&self, y: i32) -> Option<i32> {
        self.chunks[CENTER].uniform_biome_id_at(y)
    }

    pub fn owner_biome_section(&self, y: i32) -> (usize, [i32; 4096]) {
        (self.chunks[CENTER].biome_palette_len_at(y), self.chunks[CENTER].biome_section_ids_at(y))
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
}
