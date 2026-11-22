use super::blocks::{AIR, BlockId};
use super::chunk::column::ColumnChunk;
use super::decoration_class::class_of;
use super::{CHUNK_MASK, CHUNK_SHIFT};

pub const SIDE: i32 = 3;
pub const SLOTS: usize = 9;

pub struct Write {
    pub dx: i32,
    pub dz: i32,
    pub index: u32,
    pub seen: BlockId,
    pub block: BlockId,
}

pub struct Writes {
    cells: Vec<Write>,
}

impl Writes {
    pub fn apply(&self, chunk: &mut ColumnChunk, offset: (i32, i32)) {
        for cell in &self.cells {
            if cell.dx == offset.0 && cell.dz == offset.1 && chunk.get_at(cell.index as usize) == cell.seen {
                chunk.set_at(cell.index as usize, cell.block);
            }
        }
    }
}

pub struct Window<'a> {
    chunks: &'a mut [ColumnChunk; SLOTS],
    source: (i32, i32),
    writes: Vec<Write>,
}

impl<'a> Window<'a> {
    pub fn new(chunks: &'a mut [ColumnChunk; SLOTS], source: (i32, i32)) -> Self {
        Self { chunks, source, writes: Vec::new() }
    }

    #[inline]
    pub fn slot(&self, x: i32, z: i32) -> Option<usize> {
        slot_of(x, z, self.source)
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32, z: i32) -> BlockId {
        if !self.chunks[0].contains_y(y) {
            return AIR;
        }
        let Some(slot) = self.slot(x, z) else { return AIR };
        self.chunks[slot].get(x & CHUNK_MASK, y, z & CHUNK_MASK)
    }

    #[inline]
    pub fn set(&mut self, x: i32, y: i32, z: i32, block: BlockId) -> bool {
        self.place(x, y, z, block, false)
    }

    #[inline]
    pub fn set_changed(&mut self, x: i32, y: i32, z: i32, block: BlockId) -> bool {
        self.place(x, y, z, block, true)
    }

    #[inline]
    fn place(&mut self, x: i32, y: i32, z: i32, block: BlockId, skip_unchanged: bool) -> bool {
        if !self.chunks[0].contains_y(y) {
            return false;
        }
        let Some(slot) = self.slot(x, z) else { return false };
        let (local_x, local_z) = (x & CHUNK_MASK, z & CHUNK_MASK);
        let seen = self.chunks[slot].get(local_x, y, local_z);
        if skip_unchanged && seen == block {
            return false;
        }
        self.writes.push(Write {
            dx: (x >> CHUNK_SHIFT) - self.source.0,
            dz: (z >> CHUNK_SHIFT) - self.source.1,
            index: self.chunks[slot].index(local_x, y, local_z) as u32,
            seen,
            block,
        });
        self.chunks[slot].set(local_x, y, local_z, block);
        true
    }

    pub fn has_block(&self, x: i32, y: i32, z: i32) -> bool {
        self.chunks[0].contains_y(y) && self.slot(x, z).is_some()
    }

    pub fn min_y(&self) -> i32 {
        self.chunks[0].min_y()
    }

    pub fn max_y(&self) -> i32 {
        self.chunks[0].max_y()
    }

    pub fn take_writes(&mut self) -> Writes {
        Writes {
            cells: std::mem::take(&mut self.writes),
        }
    }
}

pub fn slot_of(x: i32, z: i32, source: (i32, i32)) -> Option<usize> {
    let (dx, dz) = ((x >> CHUNK_SHIFT) - source.0, (z >> CHUNK_SHIFT) - source.1);
    (dx.abs() < 2 && dz.abs() < 2).then(|| ((dx + 1) * SIDE + (dz + 1)) as usize)
}

pub fn window_chunks<'w>(source: (i32, i32), earlier: impl Iterator<Item = ((i32, i32), &'w Writes)> + Clone, base: impl Fn(usize) -> ColumnChunk) -> [ColumnChunk; SLOTS] {
    std::array::from_fn(|slot| {
        let target = (source.0 + slot as i32 / SIDE - 1, source.1 + slot as i32 % SIDE - 1);
        let mut chunk = base(slot);
        for (cell, writes) in earlier.clone() {
            if (cell.0 - target.0).abs() <= 1 && (cell.1 - target.1).abs() <= 1 {
                writes.apply(&mut chunk, (target.0 - cell.0, target.1 - cell.1));
            }
        }
        chunk
    })
}

pub fn assemble<'w>(cell: (i32, i32), mut chunk: ColumnChunk, around: impl Fn(usize) -> &'w Writes) -> ColumnChunk {
    for source in sources_near(cell) {
        let index = ((source.0 - cell.0 + 1) * SIDE + (source.1 - cell.1 + 1)) as usize;
        around(index).apply(&mut chunk, (cell.0 - source.0, cell.1 - source.1));
    }
    chunk
}

pub fn sources_near(target: (i32, i32)) -> Vec<(i32, i32)> {
    let mut sources: Vec<(u8, i32, i32)> = (target.0 - 1..=target.0 + 1)
        .flat_map(|x| (target.1 - 1..=target.1 + 1).map(move |z| (x, z)))
        .map(|source| (class_of(source), source.0, source.1))
        .collect();
    sources.sort_unstable();
    sources.into_iter().map(|(_, x, z)| (x, z)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slots_cover_only_the_three_by_three_chunks() {
        for dx in -3..=3 {
            for dz in -3..=3 {
                let slot = slot_of(dx * 16 + 5, dz * 16 + 7, (0, 0));
                let inside = (-1..=1).contains(&dx) && (-1..=1).contains(&dz);
                assert_eq!(slot.is_some(), inside, "{dx} {dz}");
                if let Some(slot) = slot {
                    assert!(slot < SLOTS, "{dx} {dz}");
                    assert_eq!(slot, ((dx + 1) * 3 + (dz + 1)) as usize);
                }
            }
        }
    }
}
