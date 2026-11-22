use super::rules::RuleKind;
use super::{CENTER, Region, SPARE_DIRTY};
use crate::shared::blocks::BlockId;
use crate::shared::chunk::packed::Packed;
use crate::shared::chunk::proto::{PlacedEntity, ProtoChunk, RoutedTick};
use std::sync::Arc;

pub(super) fn section_position(x: u8, y: i16, z: u8) -> (i8, u16) {
    (y.div_euclid(16) as i8, ((y.rem_euclid(16) as u16) << 8) | ((z as u16) << 4) | x as u16)
}

#[derive(Clone)]
pub(super) struct Sectioned {
    sections: Box<[(i8, u16)]>,
    positions: Box<[u16]>,
}

impl Sectioned {
    fn new(dirty: &mut Vec<(u8, i16, u8)>) -> Self {
        dirty.sort_unstable_by_key(|&(x, y, z)| section_position(x, y, z));
        dirty.dedup();
        let mut sections: Vec<(i8, u16)> = Vec::new();
        let mut positions = Vec::with_capacity(dirty.len());
        for &(x, y, z) in dirty.iter() {
            let (section, position) = section_position(x, y, z);
            match sections.last_mut() {
                Some((last, count)) if *last == section => *count += 1,
                _ => sections.push((section, 1)),
            }
            positions.push(position);
        }
        Self {
            sections: sections.into_boxed_slice(),
            positions: positions.into_boxed_slice(),
        }
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = (u8, i16, u8)> + '_ {
        let mut rest = &self.positions[..];
        self.sections.iter().flat_map(move |&(section, count)| {
            let (head, tail) = rest.split_at(count as usize);
            rest = tail;
            head.iter()
                .map(move |&position| ((position & 15) as u8, section as i16 * 16 + (position >> 8) as i16, ((position >> 4) & 15) as u8))
        })
    }
}

#[derive(Clone, Copy)]
pub(super) struct Write {
    pub(super) x: u8,
    pub(super) y: i16,
    pub(super) z: u8,
    pub(super) block: BlockId,
    pub(super) seen: BlockId,
    pub(super) rule: u16,
}

#[derive(Clone)]
pub(super) struct Replay {
    pub(super) positions: Box<[(u8, i16, u8)]>,
    pub(super) palette: Box<[BlockId]>,
    pub(super) blocks: Packed,
    pub(super) seen: Packed,
    pub(super) rules: Packed,
}

#[derive(Clone)]
pub struct Changes {
    positions: Sectioned,
    palette: Box<[BlockId]>,
    indices: Packed,
    pub(super) marks: Sectioned,
    pub(super) replay: Option<Replay>,
    pub(super) rules: Arc<[(Box<[u16]>, RuleKind)]>,
    pub entities: Vec<PlacedEntity>,
    pub routed_ticks: Vec<RoutedTick>,
    pub consumed_ticks: Vec<RoutedTick>,
    pub pending_fluid: Vec<RoutedTick>,
}

impl Replay {
    fn new(writes: &[Write]) -> Self {
        let mut palette: Vec<BlockId> = Vec::new();
        let mut blocks = Vec::with_capacity(writes.len());
        let mut seen = Vec::with_capacity(writes.len());
        for write in writes {
            for (source, target) in [(&mut blocks, write.block), (&mut seen, write.seen)] {
                let index = palette.iter().position(|&known| known == target).unwrap_or_else(|| {
                    palette.push(target);
                    palette.len() - 1
                }) as u16;
                source.push(index);
            }
        }
        let rules: Vec<u16> = writes.iter().map(|write| write.rule).collect();
        let largest_rule = rules.iter().copied().max().unwrap_or(0) as usize;
        Self {
            positions: writes.iter().map(|write| (write.x, write.y, write.z)).collect(),
            blocks: Packed::new(palette.len().saturating_sub(1), &blocks),
            seen: Packed::new(palette.len().saturating_sub(1), &seen),
            rules: Packed::new(largest_rule, &rules),
            palette: palette.into_boxed_slice(),
        }
    }
}

struct Taken {
    entities: Vec<PlacedEntity>,
    marks: Vec<(u8, i16, u8)>,
    routed_ticks: Vec<RoutedTick>,
    consumed_ticks: Vec<RoutedTick>,
    pending_fluid: Vec<RoutedTick>,
    writes: Vec<Write>,
}

impl Changes {
    fn new(dirty: &mut Vec<(u8, i16, u8)>, chunk: &ProtoChunk, taken: Taken, rules: Arc<[(Box<[u16]>, RuleKind)]>) -> Self {
        let Taken {
            entities,
            mut marks,
            routed_ticks,
            consumed_ticks,
            pending_fluid,
            writes,
        } = taken;
        let positions = Sectioned::new(dirty);
        let mut palette: Vec<BlockId> = Vec::new();
        let indices: Vec<u16> = positions
            .iter()
            .map(|(x, y, z)| {
                let block = chunk.get(x as i32, y as i32, z as i32);
                palette.iter().position(|&known| known == block).unwrap_or_else(|| {
                    palette.push(block);
                    palette.len() - 1
                }) as u16
            })
            .collect();
        let replay = (!writes.is_empty()).then(|| Replay::new(&writes));
        Self {
            indices: Packed::new(palette.len().saturating_sub(1), &indices),
            palette: palette.into_boxed_slice(),
            positions,
            marks: Sectioned::new(&mut marks),
            replay,
            rules,
            entities,
            routed_ticks,
            consumed_ticks,
            pending_fluid,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.positions.positions.is_empty() && self.marks.positions.is_empty() && self.entities.is_empty() && self.routed_ticks.is_empty()
    }

    pub fn changes(&self) -> impl Iterator<Item = (u8, i16, u8, BlockId)> + '_ {
        self.positions.iter().enumerate().map(|(index, (x, y, z))| (x, y, z, self.palette[self.indices.get(index) as usize]))
    }

    pub fn apply_to(&self, target: &mut ProtoChunk) {
        for (x, y, z, block) in self.changes() {
            target.set(x as i32, y as i32, z as i32, block);
        }
        for entity in &self.entities {
            target.add_entity(entity.clone());
        }
        for (x, y, z) in self.marks.iter() {
            target.mark_post_process(x, y, z);
        }
    }
}

impl Region<'_> {
    pub fn into_changes(mut self) -> [Changes; 9] {
        let rules: Arc<[(Box<[u16]>, RuleKind)]> = std::mem::take(&mut self.rules).into();
        let chunks = std::array::from_fn(|slot| {
            let consumed = if slot == CENTER { self.chunks[slot].inherited_ticks().to_vec() } else { Vec::new() };
            let pending = if slot == CENTER { self.chunks[slot].take_pending_fluid() } else { Vec::new() };
            Changes::new(
                &mut self.dirty[slot],
                &self.chunks[slot],
                Taken {
                    entities: std::mem::take(&mut self.entities[slot]),
                    marks: std::mem::take(&mut self.marks[slot]),
                    routed_ticks: std::mem::take(&mut self.routed[slot]),
                    consumed_ticks: consumed,
                    pending_fluid: pending,
                    writes: std::mem::take(&mut self.writes[slot]),
                },
                Arc::clone(&rules),
            )
        });
        for chunk in &mut self.chunks {
            chunk.discard_dense();
        }
        for mut dirty in std::mem::take(&mut self.dirty) {
            dirty.clear();
            let _ = SPARE_DIRTY.try_with(|spare| spare.borrow_mut().push(dirty));
        }
        chunks
    }
}
