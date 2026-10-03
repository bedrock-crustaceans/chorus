use std::collections::HashMap;

use atomicow::CowArc;
use tracing::debug;

use super::survival::Support;
use super::tags::Tag;
use crate::block::block_id::*;
use crate::block::state::block_state::{BlockState, BlockStateDefinition};
use crate::block::state::common::{SEA_GRASS_TYPE, UPPER_BLOCK_BIT};
use crate::registry::block_registry::BlockRegistry;

pub type BlockId = u16;

pub const AIR: BlockId = 0;

pub type State = (&'static str, BlockState);

pub fn key(definition: BlockStateDefinition) -> &'static str {
    match definition.identifier() {
        CowArc::Static(identifier) => identifier,
        other => Box::leak(other.to_string().into_boxed_str()),
    }
}

pub fn state(definition: BlockStateDefinition, value: BlockState) -> State {
    (key(definition), value)
}

pub fn flag(value: bool) -> BlockState {
    BlockState::Bool(value)
}

pub fn int(value: i32) -> BlockState {
    BlockState::Int(value)
}

pub fn text(value: &'static str) -> BlockState {
    BlockState::Enum(CowArc::Static(value))
}

const PASSABLE: &[&str] = &[
    REEDS,
    KELP,
    CAVE_VINES,
    CAVE_VINES_BODY_WITH_BERRIES,
    CAVE_VINES_HEAD_WITH_BERRIES,
    BROWN_MUSHROOM,
    RED_MUSHROOM,
    SWEET_BERRY_BUSH,
    WATERLILY,
    SMALL_DRIPLEAF_BLOCK,
    SPORE_BLOSSOM,
    HANGING_ROOTS,
    PALE_HANGING_MOSS,
    LEAF_LITTER,
    WILDFLOWERS,
    PINK_PETALS,
    WEB,
    RESIN_CLUMP,
    SCULK_VEIN,
    GLOW_LICHEN,
    FIREFLY_BUSH,
    BUSH,
    CACTUS_FLOWER,
    SEA_PICKLE,
    PITCHER_PLANT,
    TORCHFLOWER,
];

const PARTIAL: &[&str] = &[
    SNOW_LAYER,
    MOSS_CARPET,
    PALE_MOSS_CARPET,
    POINTED_DRIPSTONE,
    BAMBOO,
    CACTUS,
    BIG_DRIPLEAF,
    SMALL_DRIPLEAF_BLOCK,
    AMETHYST_CLUSTER,
    LARGE_AMETHYST_BUD,
    MEDIUM_AMETHYST_BUD,
    SMALL_AMETHYST_BUD,
    COCOA,
    AZALEA,
    FLOWERING_AZALEA,
    MANGROVE_PROPAGULE,
    SEA_PICKLE,
    TURTLE_EGG,
    IRON_CHAIN,
    LANTERN,
    MANGROVE_ROOTS,
    GRASS_PATH,
    FARMLAND,
    CREAKING_HEART,
    DECORATED_POT,
    CHEST,
    SCULK_SHRIEKER,
    SCULK_SENSOR,
    CALIBRATED_SCULK_SENSOR,
];

const ALWAYS_FLOODED: &[&str] = &[SEAGRASS, KELP, BUBBLE_COLUMN];

pub struct Entry {
    pub kind: u16,
    pub runtime: i32,
    pub name: &'static str,
    pub states: Vec<State>,
    tags: u128,
    pub solid: bool,
    pub full: bool,
    pub liquid: bool,
    pub flooded: bool,
    pub upper: Option<bool>,
    pub support: Support,
}

fn upper_half(states: &[State]) -> Option<bool> {
    states.iter().find_map(|(name, value)| match value {
        BlockState::Bool(upper) if *name == key(UPPER_BLOCK_BIT) => Some(*upper),
        BlockState::Enum(kind) if *name == key(SEA_GRASS_TYPE) => match kind.as_ref() {
            "double_top" => Some(true),
            "double_bot" => Some(false),
            _ => None,
        },
        _ => None,
    })
}

pub struct BlockTable<'a> {
    registry: &'a BlockRegistry,
    entries: Vec<Entry>,
    index: HashMap<String, BlockId>,
    kinds: HashMap<&'static str, u16>,
}

pub struct Blocks {
    entries: Vec<Entry>,
    index: HashMap<String, BlockId>,
}

fn index_key(name: &str, states: &[State], flooded: bool) -> String {
    let mut sorted: Vec<_> = states.iter().map(|(k, v)| format!("{k}={v:?}")).collect();
    sorted.sort();
    format!("{name}[{}]{}", sorted.join(","), if flooded { "~" } else { "" })
}

impl<'a> BlockTable<'a> {
    pub fn new(registry: &'a BlockRegistry) -> Self {
        let mut table = Self {
            registry,
            entries: Vec::new(),
            index: HashMap::new(),
            kinds: HashMap::new(),
        };
        table.name(crate::block::block_id::AIR);
        table
    }

    pub fn name(&mut self, name: &'static str) -> BlockId {
        self.get(name, &[])
    }

    pub fn get(&mut self, name: &'static str, states: &[State]) -> BlockId {
        self.register(name, states, ALWAYS_FLOODED.contains(&name))
    }

    pub fn flooded(&mut self, id: BlockId) -> BlockId {
        let entry = &self.entries[id as usize];
        let (name, states) = (entry.name, entry.states.clone());
        self.register(name, &states, true)
    }

    fn register(&mut self, name: &'static str, states: &[State], flooded: bool) -> BlockId {
        let index_key = index_key(name, states, flooded);
        if let Some(&id) = self.index.get(&index_key) {
            return id;
        }
        let runtime = resolve(self.registry, name, states);
        let tags = Tag::ALL.iter().enumerate().filter(|(_, tag)| tag.members().contains(&name)).fold(0u128, |mask, (i, _)| mask | 1 << i);
        let liquid = name == WATER || name == LAVA;
        let air = tags & (1 << Tag::Air as u32) != 0;
        let soft = (1 << Tag::Replaceable as u32) | (1 << Tag::Flowers as u32) | (1 << Tag::Saplings as u32);
        let solid = !air && !liquid && tags & soft == 0 && !PASSABLE.contains(&name);
        let next_kind = self.kinds.len() as u16;
        let kind = *self.kinds.entry(name).or_insert(next_kind);
        self.entries.push(Entry {
            kind,
            runtime,
            name,
            states: states.to_vec(),
            tags,
            solid,
            full: solid && !PARTIAL.contains(&name),
            liquid,
            flooded,
            upper: upper_half(states),
            support: Support::of(name, |tag| tags & (1 << tag as u32) != 0),
        });
        let id = (self.entries.len() - 1) as BlockId;
        self.index.insert(index_key, id);
        id
    }

    pub fn kind(&mut self, name: &'static str) -> u16 {
        let id = self.name(name);
        self.entries[id as usize].kind
    }

    pub fn finish(self) -> Blocks {
        Blocks {
            entries: self.entries,
            index: self.index,
        }
    }
}

impl Blocks {
    #[cfg(test)]
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn entry(&self, id: BlockId) -> &Entry {
        &self.entries[id as usize]
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn runtime(&self, id: BlockId) -> i32 {
        self.entries[id as usize].runtime
    }

    pub fn is(&self, id: BlockId, tag: Tag) -> bool {
        self.entries[id as usize].tags & (1 << tag as u32) != 0
    }

    pub fn is_air(&self, id: BlockId) -> bool {
        self.is(id, Tag::Air)
    }

    pub fn is_kind(&self, id: BlockId, kind: u16) -> bool {
        self.entries[id as usize].kind == kind
    }

    pub fn same_kind(&self, a: BlockId, b: BlockId) -> bool {
        self.entries[a as usize].kind == self.entries[b as usize].kind
    }

    pub fn name_of(&self, id: BlockId) -> &'static str {
        self.entries[id as usize].name
    }

    pub fn state(&self, id: BlockId, definition: BlockStateDefinition) -> Option<&BlockState> {
        let wanted = key(definition);
        self.entries[id as usize].states.iter().find(|(k, _)| *k == wanted).map(|(_, v)| v)
    }

    pub fn flag(&self, id: BlockId, definition: BlockStateDefinition) -> bool {
        matches!(self.state(id, definition), Some(BlockState::Bool(true)))
    }

    pub fn int(&self, id: BlockId, definition: BlockStateDefinition) -> Option<i32> {
        match self.state(id, definition) {
            Some(BlockState::Int(value)) => Some(*value),
            _ => None,
        }
    }

    pub fn text(&self, id: BlockId, definition: BlockStateDefinition) -> Option<&str> {
        match self.state(id, definition) {
            Some(BlockState::Enum(value)) => Some(value.as_ref()),
            _ => None,
        }
    }

    pub fn with(&self, id: BlockId, (wanted, value): State) -> BlockId {
        let entry = &self.entries[id as usize];
        let mut states: Vec<State> = entry.states.iter().filter(|(k, _)| *k != wanted).cloned().collect();
        states.push((wanted, value));
        self.index.get(&index_key(entry.name, &states, entry.flooded)).copied().unwrap_or(id)
    }

    pub fn flooded(&self, id: BlockId) -> BlockId {
        let entry = &self.entries[id as usize];
        self.index.get(&index_key(entry.name, &entry.states, true)).copied().unwrap_or(id)
    }
}

fn resolve(registry: &BlockRegistry, name: &str, states: &[State]) -> i32 {
    let air = registry.get_block_id(crate::block::block_id::AIR).unwrap_or(0);
    let Some(default) = registry.get_block_id(name) else {
        debug!("no bedrock block for {name}, using air");
        return air;
    };
    if states.is_empty() {
        return default;
    }
    registry.get_block_id_with_states(name, states).unwrap_or_else(|| {
        debug!("no {name} state matching {states:?}, using its default");
        default
    })
}
