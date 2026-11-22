use super::template::{FULL_CUBE, PaletteEntry, StateKind, TemplateLibrary};
use crate::shared::blocks::{AIR, BlockId, BlockTable, State};
use atomicow::CowArc;
use chorus_block::state::block_state::BlockState;
use chorus_structure_data::{ROTATIONS, STATE_TRANSFORM, StatePool};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::Arc;

const NAMESPACE: &str = "minecraft:";
const AIR_NAME: &str = "air";

fn typed_states(table: &BlockTable, name: &str, states: &str) -> Vec<State> {
    let registry = table.registry();
    let Some(defaults) = registry.get_block_id(name).and_then(|hash| registry.get_permutation(hash)) else {
        return Vec::new();
    };
    states
        .split(',')
        .filter_map(|pair| pair.split_once('='))
        .filter_map(|(key, value)| {
            let default = defaults.get_states().get(key)?;
            let typed = match default {
                BlockState::Bool(_) => BlockState::Bool(value == "1" || value == "true"),
                BlockState::Int(_) => BlockState::Int(value.parse().ok()?),
                BlockState::Enum(_) => BlockState::Enum(CowArc::Owned(Arc::from(value))),
            };
            Some((CowArc::Owned(Arc::from(key)), typed))
        })
        .collect()
}

pub fn register_named(table: &mut BlockTable, name: &str, states: &str) -> BlockId {
    let full = format!("{NAMESPACE}{name}");
    let typed = typed_states(table, &full, states);
    let wanted = states.split(',').filter(|pair| !pair.is_empty()).count();
    assert_eq!(typed.len(), wanted, "unresolved state in {name}[{states}]");
    table.get(full, &typed)
}

pub fn full_name(entry: &PaletteEntry) -> String {
    format!("{NAMESPACE}{}", entry.name)
}

pub struct PaletteBlocks {
    pool: Arc<StatePool>,
    ids: Vec<BlockId>,
    soaked: Vec<BlockId>,
    flags: Vec<u8>,
    by_block: HashMap<BlockId, u16>,
    variants: Vec<PaletteEntry>,
    air: Vec<bool>,
    unknown: Vec<String>,
    turned: Vec<[BlockId; ROTATIONS]>,
}

impl PaletteBlocks {
    pub fn register(table: &mut BlockTable, library: &TemplateLibrary) -> Self {
        let pool = library.pool().clone();
        let mut unknown = BTreeSet::new();
        let mut ids = Vec::with_capacity(pool.len());
        let mut flags = Vec::with_capacity(pool.len());
        let mut air = Vec::with_capacity(pool.len());
        for (position, entry) in pool.entries().iter().enumerate() {
            if pool.kind(position as u16) != StateKind::Block {
                ids.push(AIR);
                flags.push(0);
                air.push(false);
                continue;
            }
            let name = full_name(entry);
            if table.registry().get_block_id(&name).is_none() {
                unknown.insert(entry.name.clone());
            }
            let states = typed_states(table, &name, &entry.states);
            let mut id = table.get(name, &states);
            if entry.waterlogged {
                id = table.flooded(id);
            }
            ids.push(id);
            flags.push(if table.is_collision_cube(id) { FULL_CUBE } else { 0 });
            air.push(entry.name == AIR_NAME);
        }
        if !unknown.is_empty() {
            tracing::debug!(blocks = ?unknown, "structure templates use blocks the registry does not define");
        }
        let variants = parity_variants(&pool);
        for entry in &variants {
            let name = full_name(entry);
            let states = typed_states(table, &name, &entry.states);
            let mut id = table.get(name, &states);
            if entry.waterlogged {
                id = table.flooded(id);
            }
            ids.push(id);
            flags.push(if table.is_collision_cube(id) { FULL_CUBE } else { 0 });
            air.push(false);
        }
        let soaked: Vec<BlockId> = ids.iter().map(|&id| if table.is_waterloggable(id) { table.flooded(id) } else { id }).collect();
        let mut by_block = HashMap::with_capacity(ids.len());
        for (position, &id) in ids.iter().enumerate() {
            by_block.entry(id).or_insert(position as u16);
        }
        for (position, &id) in soaked.iter().enumerate() {
            by_block.entry(id).or_insert(position as u16);
        }
        let turned = (0..pool.len()).map(|state| turned_variants(table, &pool, &ids, state as u16)).collect();
        Self {
            pool,
            ids,
            soaked,
            flags,
            by_block,
            variants,
            air,
            unknown: unknown.into_iter().collect(),
            turned,
        }
    }

    pub fn unknown_blocks(&self) -> &[String] {
        &self.unknown
    }

    pub fn mirrored(&self, palette_index: u16, along_z: bool) -> u16 {
        self.pool.mirrored(palette_index, along_z)
    }

    pub fn name(&self, palette_index: u16) -> &str {
        &self.pool.entry(palette_index).name
    }

    pub fn states(&self, palette_index: u16) -> &str {
        &self.pool.entry(palette_index).states
    }

    pub fn is_waterlogged(&self, palette_index: u16) -> bool {
        self.pool.entry(palette_index).waterlogged
    }

    pub fn is_air(&self, palette_index: u16) -> bool {
        self.air[palette_index as usize]
    }

    pub fn turned_block(&self, state: u16, rotation: usize) -> BlockId {
        self.turned[state as usize][rotation]
    }

    pub fn soaked(&self, palette_index: u16) -> BlockId {
        self.soaked[palette_index as usize]
    }

    pub fn block(&self, palette_index: u16) -> BlockId {
        self.ids[palette_index as usize]
    }

    pub fn flags_of(&self, block: BlockId) -> Option<u8> {
        self.by_block.get(&block).map(|&index| self.flags[index as usize])
    }

    pub fn flags(&self, palette_index: u16) -> u8 {
        self.flags[palette_index as usize]
    }

    pub fn states_of(&self, block: BlockId) -> Option<&str> {
        self.by_block.get(&block).map(|&index| self.states_at(index))
    }

    fn states_at(&self, index: u16) -> &str {
        match (index as usize).checked_sub(self.pool.len()) {
            Some(extra) => &self.variants[extra].states,
            None => &self.pool.entry(index).states,
        }
    }

    pub fn find(&self, name: &str, states: &str, waterlogged: bool) -> Option<u16> {
        self.pool.find(name, states, waterlogged)
    }

    pub fn named(&self, name: &str, states: &str, waterlogged: bool) -> BlockId {
        self.find(name, states, waterlogged)
            .map(|index| self.block(index))
            .unwrap_or_else(|| panic!("no declared template block {name}[{states}]"))
    }
}

type Properties = BTreeMap<String, String>;

fn properties_of(states: &str) -> Properties {
    states
        .split(',')
        .filter(|pair| !pair.is_empty())
        .filter_map(|pair| pair.split_once('='))
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect()
}

fn option_set(pairs: &[(&str, &str)]) -> Properties {
    pairs.iter().map(|(key, value)| ((*key).to_owned(), (*value).to_owned())).collect()
}

fn parity_variant_options() -> Vec<(&'static str, Vec<Properties>)> {
    let connection_keys = ["minecraft:connection_east", "minecraft:connection_north", "minecraft:connection_south", "minecraft:connection_west"];
    let connections = (0..16)
        .map(|bits| {
            let pairs = connection_keys
                .into_iter()
                .enumerate()
                .map(|(index, key)| (key, if (bits >> (3 - index)) & 1 == 0 { "0" } else { "1" }))
                .collect::<Vec<_>>();
            option_set(&pairs)
        })
        .collect();
    let wall_keys = ["wall_connection_type_east", "wall_connection_type_north", "wall_connection_type_south", "wall_connection_type_west"];
    let wall_sides = ["none", "short", "tall"];
    let mut walls = Vec::new();
    for combination in 0..81 {
        let mut pairs = (0..4)
            .map(|index| {
                let digit = (combination / 3usize.pow(3 - index as u32)) % 3;
                (wall_keys[index], wall_sides[digit])
            })
            .collect::<Vec<_>>();
        for post in ["0", "1"] {
            pairs.push(("wall_post_bit", post));
            walls.push(option_set(&pairs));
            pairs.pop();
        }
    }
    vec![
        ("minecraft:connection_east", connections),
        ("wall_post_bit", walls),
        (
            "minecraft:corner",
            ["none", "inner_left", "inner_right", "outer_left", "outer_right"]
                .into_iter()
                .map(|corner| option_set(&[("minecraft:corner", corner)]))
                .collect(),
        ),
        (
            "upper_block_bit",
            ["north", "east", "south", "west"]
                .into_iter()
                .map(|facing| option_set(&[("minecraft:cardinal_direction", facing)]))
                .collect(),
        ),
        ("upside_down_bit", ["0", "1", "2", "3"].into_iter().map(|turn| option_set(&[("direction", turn)])).collect()),
    ]
}

fn parity_variants(pool: &StatePool) -> Vec<PaletteEntry> {
    let options = parity_variant_options();
    let mut bases: HashSet<(&str, bool, usize, String)> = HashSet::new();
    let mut seen: HashSet<PaletteEntry> = HashSet::new();
    let mut found = Vec::new();
    for (position, entry) in pool.entries().iter().enumerate() {
        if pool.kind(position as u16) != StateKind::Block {
            continue;
        }
        let properties = properties_of(&entry.states);
        for (slot, (trigger, choices)) in options.iter().enumerate() {
            if !properties.contains_key(*trigger) || !choices[0].keys().all(|key| properties.contains_key(key)) {
                continue;
            }
            let rest: Properties = properties
                .iter()
                .filter(|(key, _)| !choices[0].contains_key(*key))
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect();
            let base = rest.iter().map(|(key, value)| format!("{key}={value}")).collect::<Vec<_>>().join(",");
            if !bases.insert((entry.name.as_str(), entry.waterlogged, slot, base)) {
                continue;
            }
            for choice in choices {
                let mut merged = rest.clone();
                merged.extend(choice.iter().map(|(key, value)| (key.clone(), value.clone())));
                let states = merged.iter().map(|(key, value)| format!("{key}={value}")).collect::<Vec<_>>().join(",");
                let variant = PaletteEntry { states, ..entry.clone() };
                if pool.find(&variant.name, &variant.states, variant.waterlogged).is_none() && seen.insert(variant.clone()) {
                    found.push(variant);
                }
            }
        }
    }
    found
}

fn turned_variants(table: &mut BlockTable, pool: &StatePool, ids: &[BlockId], state: u16) -> [BlockId; ROTATIONS] {
    std::array::from_fn(|rotation| {
        let base = pool.rotated(state, rotation);
        if pool.kind(state) != StateKind::Block {
            return ids[base as usize];
        }
        let entry = pool.entry(base);
        let unturned = pool.entry(state);
        let Some(states) = STATE_TRANSFORM.placement(&entry.name, &unturned.states, &entry.states, rotation) else {
            return ids[base as usize];
        };
        let name = full_name(entry);
        let typed = typed_states(table, &name, &states);
        if typed.len() != states.split(',').filter(|pair| !pair.is_empty()).count() {
            return ids[base as usize];
        }
        let id = table.get(name, &typed);
        if entry.waterlogged { table.flooded(id) } else { id }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::structure::data::StructureData;
    use chorus_block::block_registry::BlockRegistry;

    fn registry() -> BlockRegistry {
        let mut registry = BlockRegistry::new();
        registry.register_all(chorus_block::r#impl::DEFINITIONS.iter().copied());
        registry
    }

    #[test]
    fn only_blackstone_bricks_is_unknown_to_the_registry() {
        let registry = registry();
        let data = StructureData::load().expect("vanilla templates");
        let library = data.library.as_ref().expect("the structure data holds the templates");
        let mut table = BlockTable::new(&registry);
        let palette = PaletteBlocks::register(&mut table, library);
        assert_eq!(palette.unknown_blocks(), ["blackstone_bricks".to_owned()]);
    }
}
