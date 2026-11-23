use super::blocks::{AIR, BlockId};
use super::chunk::proto::{ProtoChunk, RoutedTick};
use super::chunk::region::{Changes, Write, replay_ordered};
use super::decoration_class::class_of;
use super::{CHUNK_MASK, CHUNK_SHIFT};

pub const SIDE: i32 = 3;
pub const SLOTS: usize = 9;

pub type SourceChanges = [Changes; SLOTS];

pub struct Window<'a> {
    chunks: &'a mut [ProtoChunk; SLOTS],
    source: (i32, i32),
    writes: [Vec<Write>; SLOTS],
    routed: [Vec<RoutedTick>; SLOTS],
}

impl<'a> Window<'a> {
    pub fn new(chunks: &'a mut [ProtoChunk; SLOTS], source: (i32, i32)) -> Self {
        Self {
            chunks,
            source,
            writes: Default::default(),
            routed: Default::default(),
        }
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
        if seen != block {
            self.writes[slot].push(Write {
                x: local_x as u8,
                y: y as i16,
                z: local_z as u8,
                block,
                seen,
                rule: 0,
            });
            self.chunks[slot].set(local_x, y, local_z, block);
        }
        true
    }

    pub fn route_tick(&mut self, tick: RoutedTick) {
        if let Some(slot) = self.slot(tick.pos.x, tick.pos.z) {
            self.routed[slot].push(tick);
        }
    }

    pub fn min_y(&self) -> i32 {
        self.chunks[0].min_y
    }

    pub fn max_y(&self) -> i32 {
        self.chunks[0].max_y()
    }

    pub fn take_changes(&mut self) -> SourceChanges {
        let writes = std::mem::take(&mut self.writes);
        let routed = std::mem::take(&mut self.routed);
        let mut routed = routed.into_iter();
        writes.map(|writes| Changes::from_writes(&writes, routed.next().unwrap_or_default()))
    }
}

pub fn slot_at(dx: i32, dz: i32) -> Option<usize> {
    (dx.abs() < 2 && dz.abs() < 2).then(|| ((dx + 1) * SIDE + (dz + 1)) as usize)
}

pub fn slot_of(x: i32, z: i32, source: (i32, i32)) -> Option<usize> {
    slot_at((x >> CHUNK_SHIFT) - source.0, (z >> CHUNK_SHIFT) - source.1)
}

pub fn offset_slot(from: (i32, i32), to: (i32, i32)) -> usize {
    ((to.0 - from.0 + 1) * SIDE + (to.1 - from.1 + 1)) as usize
}

pub fn window_chunks<'w>(source: (i32, i32), earlier: impl Iterator<Item = ((i32, i32), &'w SourceChanges)> + Clone, base: impl Fn(usize) -> ProtoChunk) -> [ProtoChunk; SLOTS] {
    std::array::from_fn(|slot| {
        let target = (source.0 + slot as i32 / SIDE - 1, source.1 + slot as i32 % SIDE - 1);
        let mut chunk = base(slot);
        let mail: Vec<&Changes> = earlier
            .clone()
            .filter(|(cell, _)| (cell.0 - target.0).abs() <= 1 && (cell.1 - target.1).abs() <= 1)
            .map(|(cell, changes)| &changes[offset_slot(cell, target)])
            .collect();
        replay_ordered(&mut chunk, &mail);
        chunk
    })
}

pub fn assemble<'w>(cell: (i32, i32), mut chunk: ProtoChunk, around: impl Fn(usize) -> &'w SourceChanges) -> ProtoChunk {
    let mail: Vec<&Changes> = sources_near(cell).into_iter().map(|source| &around(offset_slot(cell, source))[offset_slot(source, cell)]).collect();
    replay_ordered(&mut chunk, &mail);
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
