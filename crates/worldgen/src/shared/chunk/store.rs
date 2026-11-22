use std::cell::RefCell;

use chorus_level::section_store::{SectionStore, SectionStoreBuilder};

use crate::shared::blocks::BlockId;

pub const SECTION_CELLS: usize = 4096;

thread_local! {
    static SPARE_DENSE: RefCell<Vec<Box<[BlockId; SECTION_CELLS]>>> = const { RefCell::new(Vec::new()) };
}

fn take_dense() -> Box<[BlockId; SECTION_CELLS]> {
    SPARE_DENSE.try_with(|spare| spare.borrow_mut().pop()).ok().flatten().unwrap_or_else(|| Box::new([0; SECTION_CELLS]))
}

fn give_dense(blocks: Box<[BlockId; SECTION_CELLS]>) {
    let _ = SPARE_DENSE.try_with(|spare| spare.borrow_mut().push(blocks));
}

#[derive(Clone)]
pub struct BlockStore {
    min_y: i32,
    height: i32,
    air: BlockId,
    sections: SectionStore,
    dense: Vec<Option<Box<[BlockId; SECTION_CELLS]>>>,
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
            for (dense, value) in blocks.iter_mut().zip(self.sections.to_array(section)) {
                *dense = value as BlockId;
            }
            self.dense[section] = Some(blocks);
        }
        if let Some(Some(blocks)) = self.dense.get_mut(section) {
            blocks[local] = block;
        }
    }

    pub fn section_array(&self, section: usize) -> [i32; SECTION_CELLS] {
        match self.dense.get(section) {
            Some(Some(blocks)) => blocks.map(|block| block as i32),
            _ => self.sections.to_array(section),
        }
    }

    pub fn top_block_section(&self) -> usize {
        (0..self.sections.len())
            .rev()
            .find(|&section| self.section_array(section).iter().any(|&block| block != self.air as i32))
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
