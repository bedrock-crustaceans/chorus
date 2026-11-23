use std::cell::RefCell;

use chorus_level::section_store::{SectionStore, SectionStoreBuilder};

use crate::shared::blocks::BlockId;

pub const SECTION_CELLS: usize = 4096;
const SPARE_DENSE_CAP: usize = 64;

thread_local! {
    static SCRATCH: RefCell<Vec<BlockId>> = const { RefCell::new(Vec::new()) };
    static SPARE_DENSE: RefCell<Vec<Box<[BlockId; SECTION_CELLS]>>> = const { RefCell::new(Vec::new()) };
}

fn take_dense() -> Box<[BlockId; SECTION_CELLS]> {
    SPARE_DENSE.try_with(|spare| spare.borrow_mut().pop()).ok().flatten().unwrap_or_else(|| Box::new([0; SECTION_CELLS]))
}

pub fn take_scratch(len: usize, fill: BlockId) -> Vec<BlockId> {
    let mut scratch = SCRATCH.try_with(|scratch| std::mem::take(&mut *scratch.borrow_mut())).unwrap_or_default();
    scratch.clear();
    scratch.resize(len, fill);
    scratch
}

pub fn give_scratch(scratch: Vec<BlockId>) {
    let _ = SCRATCH.try_with(|slot| *slot.borrow_mut() = scratch);
}

fn give_dense(blocks: Box<[BlockId; SECTION_CELLS]>) {
    let _ = SPARE_DENSE.try_with(|spare| {
        let mut spare = spare.borrow_mut();
        if spare.len() < SPARE_DENSE_CAP {
            spare.push(blocks);
        }
    });
}

#[derive(Clone)]
pub struct BlockStore {
    min_y: i32,
    height: i32,
    air: BlockId,
    sections: SectionStore,
    dense: Vec<Option<Box<[BlockId; SECTION_CELLS]>>>,
}

impl Drop for BlockStore {
    fn drop(&mut self) {
        self.discard_dense();
    }
}

impl BlockStore {
    pub fn new(min_y: i32, height: i32, air: BlockId) -> Self {
        Self {
            min_y,
            height,
            air,
            sections: SectionStore::uniform(air as i32, (height / 16) as usize),
            dense: Vec::new(),
        }
    }

    fn locate(&self, x: i32, y: i32, z: i32) -> Option<(usize, usize)> {
        let ry = y - self.min_y;
        (0..self.height).contains(&ry).then_some(((ry >> 4) as usize, (((ry & 15) << 8) | (z << 4) | x) as usize))
    }

    pub fn get(&self, x: i32, y: i32, z: i32) -> BlockId {
        self.locate(x, y, z).map_or(self.air, |(section, local)| match self.dense.get(section) {
            Some(Some(blocks)) => blocks[local],
            _ => self.sections.get(section, local) as BlockId,
        })
    }

    pub fn columns_vec(&self) -> Vec<BlockId> {
        let height = self.height as usize;
        let mut cells = vec![self.air; 256 * height];
        for section in 0..height / 16 {
            let dense = matches!(self.dense.get(section), Some(Some(_)));
            let uniform = if dense { None } else { self.sections.uniform_value(section) };
            if uniform == Some(self.air as i32) {
                continue;
            }
            let array = self.section_array(section);
            for column in 0..256 {
                let (x, z) = (column >> 4, column & 15);
                let target = &mut cells[column * height + section * 16..][..16];
                for (y, slot) in target.iter_mut().enumerate() {
                    *slot = array[(y << 8) | (z << 4) | x] as BlockId;
                }
            }
        }
        cells
    }

    pub fn set(&mut self, x: i32, y: i32, z: i32, block: BlockId) {
        let Some((section, local)) = self.locate(x, y, z) else { return };
        if !matches!(self.dense.get(section), Some(Some(_))) {
            if self.sections.get(section, local) == block as i32 {
                return;
            }
            if self.dense.is_empty() {
                self.dense = vec![None; self.sections.len()];
            }
            let mut blocks = take_dense();
            self.sections.decode_into(section, &mut blocks, |value| value as BlockId);
            self.dense[section] = Some(blocks);
        }
        if let Some(Some(blocks)) = self.dense.get_mut(section) {
            blocks[local] = block;
        }
    }

    pub fn fill_empty(&mut self, blocks: &[BlockId]) {
        debug_assert_eq!(blocks.len(), self.sections.len() * SECTION_CELLS);
        self.discard_dense();
        let mut builder = SectionStoreBuilder::default();
        for (section, cells) in blocks.as_chunks::<SECTION_CELLS>().0.iter().enumerate() {
            if cells.iter().all(|&block| block == self.air) {
                builder.push_section(&self.sections, section);
            } else {
                let mut array = [0i32; SECTION_CELLS];
                for (target, &block) in array.iter_mut().zip(cells) {
                    *target = block as i32;
                }
                builder.push_blocks(&array);
            }
        }
        self.sections = builder.build();
    }

    pub fn read_cells(&self, out: &mut [BlockId]) {
        debug_assert_eq!(out.len(), self.sections.len() * SECTION_CELLS);
        for (section, target) in out.as_chunks_mut::<SECTION_CELLS>().0.iter_mut().enumerate() {
            match self.dense.get(section) {
                Some(Some(blocks)) => target.copy_from_slice(&blocks[..]),
                _ => self.sections.decode_into(section, target, |value| value as BlockId),
            }
        }
    }

    pub fn write_cells(&mut self, blocks: &[BlockId]) {
        debug_assert_eq!(blocks.len(), self.sections.len() * SECTION_CELLS);
        self.discard_dense();
        let mut builder = SectionStoreBuilder::default();
        for cells in blocks.as_chunks::<SECTION_CELLS>().0.iter() {
            let mut array = [0i32; SECTION_CELLS];
            for (target, &block) in array.iter_mut().zip(cells) {
                *target = block as i32;
            }
            builder.push_blocks(&array);
        }
        self.sections = builder.build();
    }

    pub fn section_array(&self, section: usize) -> [i32; SECTION_CELLS] {
        match self.dense.get(section) {
            Some(Some(blocks)) => blocks.map(|block| block as i32),
            _ => self.sections.to_array(section),
        }
    }

    pub fn section_palette(&self, section: usize) -> Option<&[i32]> {
        match self.dense.get(section) {
            Some(Some(_)) => None,
            _ => Some(self.sections.palette(section)),
        }
    }

    pub fn column_tops_below(&self, limit: i32) -> [i32; 256] {
        let mut tops = [0i32; 256];
        let mut found = [false; 256];
        let mut pending = 256;
        let limit = limit.min(self.min_y + self.height);
        let highest = ((limit - self.min_y + 15) >> 4).clamp(0, self.sections.len() as i32) as usize;
        for section in (0..highest).rev() {
            let is_dense = matches!(self.dense.get(section), Some(Some(_)));
            if !is_dense && self.sections.uniform_value(section) == Some(self.air as i32) {
                continue;
            }
            let array = self.section_array(section);
            let section_y = self.min_y + ((section as i32) << 4);
            let rows = (limit - section_y).min(16);
            for row in (0..rows).rev() {
                for column in 0..256 {
                    if !found[column] && array[((row as usize) << 8) | column] != self.air as i32 {
                        tops[column] = section_y + row + 1;
                        found[column] = true;
                        pending -= 1;
                    }
                }
                if pending == 0 {
                    return tops;
                }
            }
        }
        tops
    }

    pub fn top_block_section(&self) -> usize {
        (0..self.sections.len())
            .rev()
            .find(|&section| {
                let is_dense = matches!(self.dense.get(section), Some(Some(_)));
                if !is_dense && self.sections.uniform_value(section) == Some(self.air as i32) {
                    return false;
                }
                self.section_array(section).iter().any(|&block| block != self.air as i32)
            })
            .unwrap_or(0)
    }

    pub fn compact(&mut self) {
        if self.dense.iter().all(Option::is_none) {
            self.dense = Vec::new();
            return;
        }
        let mut builder = SectionStoreBuilder::default();
        for (section, dense) in self.dense.drain(..).enumerate() {
            match dense {
                Some(blocks) => {
                    builder.push_blocks(&blocks.map(|block| block as i32));
                    give_dense(blocks);
                }
                None => builder.push_section(&self.sections, section),
            }
        }
        self.sections = builder.build();
    }

    pub fn discard_dense(&mut self) {
        for blocks in self.dense.drain(..).flatten() {
            give_dense(blocks);
        }
    }
}
