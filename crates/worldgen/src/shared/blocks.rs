use std::collections::HashMap;

use atomicow::CowArc;
use tracing::debug;

use super::survival::Support;
use super::tags::Tag;
use chorus_block::block_id::*;
use chorus_block::block_registry::BlockRegistry;
use chorus_block::component::collision_box_component::CollisionBoxComponent;
use chorus_block::component::liquid_detection_component::{LiquidDetectionComponent, LiquidTouch, LiquidType};
use chorus_block::component::placement_filter_component::{PlacementFace, PlacementFilterComponent, PlacementRule};
use chorus_block::component::solid_component::SolidComponent;
use chorus_block::component::support_component::{SupportComponent, SupportFaces};
use chorus_block::state::block_state::{BlockState, BlockStateDefinition};
use chorus_block::state::common::{SEA_GRASS_TYPE, UPPER_BLOCK_BIT};

pub type BlockId = u16;

pub const COMPASS: [&str; 4] = ["north", "east", "south", "west"];

pub const AIR: BlockId = 0;

pub type State = (CowArc<'static, str>, BlockState);

pub fn key(definition: &BlockStateDefinition) -> CowArc<'static, str> {
    definition.identifier().clone()
}

pub fn state(definition: &BlockStateDefinition, value: BlockState) -> State {
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

pub struct Entry {
    pub kind: u16,
    pub runtime: i32,
    pub canonical: bool,
    pub name: CowArc<'static, str>,
    pub states: Vec<State>,
    tags: u128,
    pub solid: bool,
    pub transparent: bool,
    pub full: bool,
    pub cube: bool,
    pub vine_support: bool,
    pub placement: &'static [PlacementRule],
    pub placement_denied: &'static [&'static str],
    pub face_support: SupportFaces,
    pub bds_solid: bool,
    pub collision_cube: bool,
    pub top_centre: bool,
    pub raised_top: bool,
    pub liquid: bool,
    pub waterloggable: bool,
    pub water_touch: LiquidTouch,
    pub lava_touch: LiquidTouch,
    pub flooded: bool,
    pub upper: Option<bool>,
    pub support: Support,
    pub map_color: (u8, u8, u8, u8),
    flooded_id: BlockId,
    variants: Vec<(CowArc<'static, str>, BlockState, BlockId)>,
}

fn upper_half(states: &[State]) -> Option<bool> {
    states.iter().find_map(|(name, value)| match value {
        BlockState::Bool(upper) if *name == key(&UPPER_BLOCK_BIT) => Some(*upper),
        BlockState::Enum(kind) if *name == key(&SEA_GRASS_TYPE) => match kind.as_ref() {
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
    kinds: HashMap<CowArc<'static, str>, u16>,
}

pub struct Blocks {
    entries: Vec<Entry>,
}

fn index_key(name: &str, states: &[State], flooded: bool) -> String {
    let mut sorted: Vec<_> = states.iter().map(|(k, v)| format!("{k}={v:?}")).collect();
    sorted.sort();
    format!("{name}[{}]{}", sorted.join(","), if flooded { "~" } else { "" })
}

fn without<'e>(states: &[&'e State], skip: usize) -> Vec<(&'e str, &'e BlockState)> {
    states.iter().enumerate().filter(|(i, _)| *i != skip).map(|(_, state)| (state.0.as_ref(), &state.1)).collect()
}

type Variant = (CowArc<'static, str>, BlockState, BlockId);

fn keyed_variants(entries: &[Entry], members: &[usize]) -> Vec<Vec<Variant>> {
    let sorted: Vec<Vec<&State>> = members
        .iter()
        .map(|&id| {
            let mut states: Vec<&State> = entries[id].states.iter().collect();
            states.sort_by(|a, b| a.0.as_ref().cmp(b.0.as_ref()));
            states
        })
        .collect();
    let mut neighbours: HashMap<Vec<(&str, &BlockState)>, Vec<(usize, usize)>> = HashMap::new();
    for (member, states) in sorted.iter().enumerate() {
        for skip in 0..states.len() {
            neighbours.entry(without(states, skip)).or_default().push((member, skip));
        }
    }
    sorted
        .iter()
        .enumerate()
        .map(|(from, states)| {
            let mut found: Vec<(usize, usize)> = Vec::new();
            let full = without(states, usize::MAX);
            if let Some(extended) = neighbours.get(&full) {
                found.extend(extended.iter().copied());
            }
            for skip in 0..states.len() {
                let Some(siblings) = neighbours.get(&without(states, skip)) else { continue };
                for &(to, at) in siblings {
                    if to != from && sorted[to][at].0 == states[skip].0 && sorted[to][at].1 != states[skip].1 {
                        found.push((to, at));
                    }
                }
            }
            found.sort_by_key(|&(to, _)| to);
            found
                .into_iter()
                .map(|(to, at)| {
                    let (k, v) = sorted[to][at];
                    (k.clone(), v.clone(), members[to] as BlockId)
                })
                .collect()
        })
        .collect()
}

impl<'a> BlockTable<'a> {
    pub fn registry(&self) -> &'a BlockRegistry {
        self.registry
    }

    pub fn new(registry: &'a BlockRegistry) -> Self {
        let mut table = Self {
            registry,
            entries: Vec::new(),
            index: HashMap::new(),
            kinds: HashMap::new(),
        };
        table.name(chorus_block::block_id::AIR);
        table
    }

    pub fn registered(&self) -> &[Entry] {
        &self.entries
    }

    pub fn name(&mut self, name: impl Into<CowArc<'static, str>>) -> BlockId {
        self.get(name, &[])
    }

    pub fn get(&mut self, name: impl Into<CowArc<'static, str>>, states: &[State]) -> BlockId {
        self.register(name, states, false)
    }

    pub fn get_flooded(&mut self, name: impl Into<CowArc<'static, str>>, states: &[State]) -> BlockId {
        let id = self.get(name, states);
        self.flooded(id)
    }

    pub fn is_collision_cube(&self, id: BlockId) -> bool {
        self.entries[id as usize].collision_cube
    }

    pub fn is_waterloggable(&self, id: BlockId) -> bool {
        self.entries[id as usize].waterloggable
    }

    pub fn flooded(&mut self, id: BlockId) -> BlockId {
        let entry = &self.entries[id as usize];
        let (name, states) = (entry.name.clone(), entry.states.clone());
        self.register(name, &states, true)
    }

    fn register(&mut self, name: impl Into<CowArc<'static, str>>, states: &[State], flooded: bool) -> BlockId {
        let name = name.into();
        let index_key = index_key(&name, states, flooded);
        if let Some(&id) = self.index.get(&index_key) {
            return id;
        }
        let runtime = resolve(self.registry, &name, states);
        let tags = Tag::ALL
            .iter()
            .enumerate()
            .filter(|(_, tag)| tag.members().iter().any(|member| *member == name.as_ref()))
            .fold(0u128, |mask, (i, _)| mask | 1 << i);
        let liquid = name.as_ref() == WATER || name.as_ref() == LAVA;
        let next_kind = self.kinds.len() as u16;
        let kind = *self.kinds.entry(name.clone()).or_insert(next_kind);
        let components = self.registry.get_components(runtime);
        let face_support = components
            .and_then(|components| components.get::<SupportComponent>())
            .map_or(SupportFaces::None, |support| support.faces);
        let bds_solid = components.is_none_or(|components| components.get::<SolidComponent>().is_none_or(|component| component.solid));
        let filter = components.and_then(|components| components.get::<PlacementFilterComponent>());
        let placement = filter.map_or(&[][..], |filter| filter.conditions);
        let placement_denied = filter.map_or(&[][..], |filter| filter.denied);
        let solid = bds_solid;
        let map_color = components
            .and_then(|components| components.get::<chorus_block::component::map_color_component::MapColorComponent>())
            .map(|color| (color.r, color.g, color.b, color.a))
            .unwrap_or((0, 0, 0, 255));
        let touch = |liquid| {
            components
                .and_then(|components| components.get::<LiquidDetectionComponent>())
                .map_or(LiquidTouch::Blocking, |rules| rules.touches(liquid))
        };
        let vine_support = face_support == SupportFaces::All && bds_solid;
        let collision_cube = components.is_none_or(|components| {
            components.get::<SolidComponent>().is_none_or(|component| component.solid)
                && components
                    .get::<CollisionBoxComponent>()
                    .is_none_or(|box_| box_.enabled && box_.origin == glam::Vec3::ZERO && box_.size == glam::Vec3::ONE)
        });
        let transparent = components
            .and_then(|components| components.get::<chorus_block::component::transparent_component::TransparentComponent>())
            .is_some_and(|component| component.transparent);
        let support = Support::of(&name, |tag| tags & (1 << tag as u32) != 0, !placement.is_empty());
        self.entries.push(Entry {
            kind,
            runtime,
            canonical: false,
            name,
            states: states.to_vec(),
            tags,
            solid,
            transparent,
            full: face_support == SupportFaces::All,
            cube: face_support == SupportFaces::All,
            vine_support,
            placement,
            placement_denied,
            face_support,
            bds_solid,
            collision_cube,
            top_centre: face_support != SupportFaces::None,
            raised_top: face_support == SupportFaces::Top,
            liquid,
            waterloggable: components
                .and_then(|components| components.get::<LiquidDetectionComponent>())
                .is_some_and(|rules| rules.can_contain(LiquidType::Water)),
            water_touch: touch(LiquidType::Water),
            lava_touch: touch(LiquidType::Lava),
            flooded,
            upper: upper_half(states),
            support,
            flooded_id: 0,
            variants: Vec::new(),
            map_color,
        });
        let id = (self.entries.len() - 1) as BlockId;
        self.index.insert(index_key, id);
        id
    }

    pub fn kind(&mut self, name: impl Into<CowArc<'static, str>>) -> u16 {
        let id = self.name(name);
        self.entries[id as usize].kind
    }

    pub fn finish(mut self) -> Blocks {
        for id in 0..self.entries.len() {
            let entry = &self.entries[id];
            let canonical = self.registry.get_block_id_with_states(entry.name.as_ref(), &as_registry_states(&entry.states)) == Some(entry.runtime);
            let flooded_key = index_key(entry.name.as_ref(), &entry.states, true);
            self.entries[id].canonical = canonical;
            self.entries[id].flooded_id = self.index.get(&flooded_key).copied().unwrap_or(id as BlockId);
        }
        let mut groups: HashMap<(CowArc<'static, str>, bool), Vec<usize>> = HashMap::new();
        for (id, entry) in self.entries.iter().enumerate() {
            groups.entry((entry.name.clone(), entry.flooded)).or_default().push(id);
        }
        for members in groups.values() {
            let keyed = keyed_variants(&self.entries, members);
            for (&from, variants) in members.iter().zip(keyed) {
                self.entries[from].variants = variants;
            }
        }
        Blocks { entries: self.entries }
    }
}

impl Blocks {
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn entry(&self, id: BlockId) -> &Entry {
        &self.entries[id as usize]
    }

    pub fn runtime(&self, id: BlockId) -> i32 {
        self.entries[id as usize].runtime
    }

    pub fn map_color(&self, id: BlockId) -> (u8, u8, u8, u8) {
        self.entries[id as usize].map_color
    }

    pub fn canonical_runtime_ids(&self) -> impl Iterator<Item = (i32, BlockId)> + '_ {
        self.entries.iter().enumerate().filter(|(_, entry)| entry.canonical).map(|(id, entry)| (entry.runtime, id as BlockId))
    }

    pub fn runtime_name(&self, runtime: i32) -> Option<&str> {
        self.entries.iter().find(|entry| entry.runtime == runtime).map(|entry| entry.name.as_ref())
    }

    pub fn placed_on(&self, block: BlockId, neighbour: BlockId, face: PlacementFace) -> bool {
        let name = &self.entries[neighbour as usize].name;
        self.entries[block as usize]
            .placement
            .iter()
            .any(|rule| rule.allowed_faces.contains(&face) && rule.block_filter.iter().any(|block| *block == name.as_ref()))
    }

    pub fn denied_on(&self, block: BlockId, neighbour: BlockId) -> bool {
        let name = &self.entries[neighbour as usize].name;
        self.entries[block as usize].placement_denied.iter().any(|block| *block == name.as_ref())
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

    pub fn name_of(&self, id: BlockId) -> &str {
        self.entries[id as usize].name.as_ref()
    }

    pub fn state(&self, id: BlockId, definition: &BlockStateDefinition) -> Option<&BlockState> {
        let wanted = key(definition);
        self.entries[id as usize].states.iter().find(|(k, _)| *k == wanted).map(|(_, v)| v)
    }

    pub fn flag(&self, id: BlockId, definition: &BlockStateDefinition) -> bool {
        matches!(self.state(id, definition), Some(BlockState::Bool(true)))
    }

    pub fn int(&self, id: BlockId, definition: &BlockStateDefinition) -> Option<i32> {
        match self.state(id, definition) {
            Some(BlockState::Int(value)) => Some(*value),
            _ => None,
        }
    }

    pub fn text(&self, id: BlockId, definition: &BlockStateDefinition) -> Option<&str> {
        match self.state(id, definition) {
            Some(BlockState::Enum(value)) => Some(value.as_ref()),
            _ => None,
        }
    }

    pub fn with(&self, id: BlockId, (wanted, value): State) -> BlockId {
        let entry = &self.entries[id as usize];
        entry.variants.iter().find(|(k, v, _)| *k == wanted && *v == value).map_or(id, |&(_, _, variant)| variant)
    }

    pub fn flooded(&self, id: BlockId) -> BlockId {
        self.entries[id as usize].flooded_id
    }
}

fn as_registry_states(states: &[State]) -> Vec<(&str, BlockState)> {
    states.iter().map(|(key, value)| (key.as_ref(), value.clone())).collect()
}

fn resolve(registry: &BlockRegistry, name: &str, states: &[State]) -> i32 {
    let air = registry.get_block_id(chorus_block::block_id::AIR).unwrap_or(0);
    let Some(default) = registry.get_block_id(name) else {
        debug!("no bedrock block for {name}, using air");
        return air;
    };
    if states.is_empty() {
        return default;
    }
    registry.get_block_id_with_states(name, &as_registry_states(states)).unwrap_or_else(|| {
        debug!("no {name} state matching {states:?}, using its default");
        default
    })
}
