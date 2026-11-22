use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use super::read::{join_states, split_states};
use super::template::{PaletteEntry, StateKind};
use super::transform::STATE_TRANSFORM;

pub const ROTATIONS: usize = 4;
pub const AIR: u16 = 0;
const AIR_NAME: &str = "air";
const JIGSAW_NAME: &str = "jigsaw";
const STRUCTURE_BLOCK_NAME: &str = "structure_block";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatePoolFull {
    pub limit: usize,
}

impl fmt::Display for StatePoolFull {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "the structure data uses more than {} distinct block states", self.limit)
    }
}

impl Error for StatePoolFull {}

pub struct StatePoolBuilder {
    entries: Vec<PaletteEntry>,
    kinds: Vec<StateKind>,
    index: HashMap<PaletteEntry, u16>,
    overflowed: bool,
}

impl Default for StatePoolBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl StatePoolBuilder {
    pub fn new() -> Self {
        let mut builder = Self {
            entries: Vec::new(),
            kinds: Vec::new(),
            index: HashMap::new(),
            overflowed: false,
        };
        builder.intern(AIR_NAME, "", false);
        builder
    }

    fn push(&mut self, entry: PaletteEntry, kind: StateKind) -> u16 {
        if let Some(&known) = self.index.get(&entry) {
            return known;
        }
        let Ok(next) = u16::try_from(self.entries.len()) else {
            self.overflowed = true;
            return AIR;
        };
        self.index.insert(entry.clone(), next);
        self.entries.push(entry);
        self.kinds.push(kind);
        next
    }

    pub fn intern(&mut self, name: &str, states: &str, waterlogged: bool) -> u16 {
        self.push(PaletteEntry::new(name, states, waterlogged), StateKind::Block)
    }

    pub fn intern_kind(&mut self, kind: StateKind) -> u16 {
        match kind {
            StateKind::Block => AIR,
            StateKind::Jigsaw => self.push(PaletteEntry::new(JIGSAW_NAME, "", false), StateKind::Jigsaw),
            StateKind::StructureBlock => self.push(PaletteEntry::new(STRUCTURE_BLOCK_NAME, "", false), StateKind::StructureBlock),
        }
    }

    pub fn entries(&self) -> &[PaletteEntry] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn find(&self, name: &str, states: &str, waterlogged: bool) -> Option<u16> {
        self.index.get(&PaletteEntry::new(name, states, waterlogged)).copied()
    }

    pub fn freeze(mut self) -> Result<StatePool, StatePoolFull> {
        let mut rotated: Vec<[u16; ROTATIONS]> = Vec::new();
        let mut mirrored: Vec<[u16; 2]> = Vec::new();
        let mut at = 0;
        while at < self.entries.len() {
            if self.kinds[at] != StateKind::Block {
                rotated.push([AIR; ROTATIONS]);
                mirrored.push([at as u16; 2]);
                at += 1;
                continue;
            }
            let entry = self.entries[at].clone();
            let states = split_states(&entry.states);
            let turns = std::array::from_fn(|rotation| {
                let turned = join_states(&STATE_TRANSFORM.rotate(&entry.name, &states, rotation));
                self.intern(&entry.name, &turned, entry.waterlogged)
            });
            let mut mirrors = [at as u16; 2];
            if !entry.states.is_empty() {
                for (slot, along_z) in [true, false].into_iter().enumerate() {
                    let flipped = STATE_TRANSFORM.mirror(&entry.states, along_z);
                    if flipped != entry.states {
                        mirrors[slot] = self.intern(&entry.name, &flipped, entry.waterlogged);
                    }
                }
            }
            rotated.push(turns);
            mirrored.push(mirrors);
            at += 1;
        }
        if self.overflowed {
            return Err(StatePoolFull { limit: usize::from(u16::MAX) + 1 });
        }
        Ok(StatePool {
            entries: self.entries,
            kinds: self.kinds,
            index: self.index,
            rotated,
            mirrored,
        })
    }
}

pub struct StatePool {
    entries: Vec<PaletteEntry>,
    kinds: Vec<StateKind>,
    index: HashMap<PaletteEntry, u16>,
    rotated: Vec<[u16; ROTATIONS]>,
    mirrored: Vec<[u16; 2]>,
}

impl StatePool {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entries(&self) -> &[PaletteEntry] {
        &self.entries
    }

    pub fn entry(&self, state: u16) -> &PaletteEntry {
        &self.entries[state as usize]
    }

    pub fn kind(&self, state: u16) -> StateKind {
        self.kinds[state as usize]
    }

    pub fn rotated(&self, state: u16, rotation: usize) -> u16 {
        self.rotated[state as usize][rotation]
    }

    pub fn mirrored(&self, state: u16, along_z: bool) -> u16 {
        self.mirrored[state as usize][usize::from(!along_z)]
    }

    pub fn find(&self, name: &str, states: &str, waterlogged: bool) -> Option<u16> {
        self.index.get(&PaletteEntry::new(name, states, waterlogged)).copied()
    }

    pub fn air(&self) -> u16 {
        AIR
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interning_the_same_state_twice_gives_one_id() {
        let mut builder = StatePoolBuilder::new();
        let first = builder.intern("stone", "", false);
        assert_eq!(builder.intern("stone", "", false), first);
        assert_ne!(builder.intern("stone", "", true), first);
        assert_eq!(builder.intern("air", "", false), AIR);
    }

    #[test]
    fn freezing_closes_the_pool_over_rotation_and_mirror() {
        let mut builder = StatePoolBuilder::new();
        let door = builder.intern("acacia_door", "minecraft:cardinal_direction=south", false);
        let pool = builder.freeze().unwrap();
        let turned = pool.rotated(door, 1);
        assert_eq!(pool.entry(turned).states, "minecraft:cardinal_direction=west");
        assert_eq!(pool.rotated(door, 0), door);
        let flipped = pool.mirrored(door, true);
        assert_eq!(pool.entry(flipped).states, "minecraft:cardinal_direction=north");
        assert_eq!(pool.mirrored(door, false), door);
        for rotation in 0..ROTATIONS {
            let at = pool.rotated(door, rotation);
            assert_eq!(pool.find(&pool.entry(at).name, &pool.entry(at).states, false), Some(at));
        }
    }

    #[test]
    fn marker_kinds_turn_into_air_and_keep_their_kind() {
        let mut builder = StatePoolBuilder::new();
        let jigsaw = builder.intern_kind(StateKind::Jigsaw);
        let pool = builder.freeze().unwrap();
        assert_eq!(pool.kind(jigsaw), StateKind::Jigsaw);
        assert_eq!(pool.rotated(jigsaw, 2), AIR);
        assert_eq!(pool.mirrored(jigsaw, true), jigsaw);
    }

    #[test]
    fn a_pool_past_the_u16_limit_is_an_error() {
        let mut builder = StatePoolBuilder::new();
        for number in 0..70_000u32 {
            builder.intern("stone", &format!("count={number}"), false);
        }
        assert!(builder.freeze().is_err());
    }
}
