use crate::shared::blocks::{BlockId, BlockTable, Entry, State};
use atomicow::CowArc;
use chorus_block::state::block_state::BlockState;
use rustc_hash::FxHashMap;
use std::collections::HashMap;

pub const SIDES: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];
pub const SIDE_NAMES: [&str; 4] = crate::shared::blocks::COMPASS;
pub const CORNERS: [&str; 5] = ["none", "inner_left", "inner_right", "outer_left", "outer_right"];
pub const WALL_TYPES: [&str; 3] = ["none", "short", "tall"];
pub const WALL_TYPE_BITS: u16 = 2;
pub const WALL_POST_SHIFT: u16 = 8;
pub const FACINGS: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

const CONNECTION_PREFIX: &str = "minecraft:connection_";
const WALL_PREFIX: &str = "wall_connection_type_";
const WALL_POST: &str = "wall_post_bit";
const CORNER: &str = "minecraft:corner";
const DIRECTION: &str = "weirdo_direction";
const UPSIDE_DOWN: &str = "upside_down_bit";
const CARDINAL: &str = "minecraft:cardinal_direction";
const NETHER_FENCE: &str = "minecraft:nether_brick_fence";
const OAK_GATE: &str = "minecraft:fence_gate";
const GATE_SUFFIX: &str = "_fence_gate";
const FENCE_SUFFIX: &str = "_fence";
const STAIRS_SUFFIX: &str = "_stairs";
const WALL_SUFFIX: &str = "_wall";
const PANE_SUFFIXES: [&str; 2] = ["glass_pane", "_bars"];
const IRON_BARS: &str = "minecraft:iron_bars";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Fence,
    NetherFence,
    Pane,
    Wall,
    Stairs,
    Gate,
}

#[derive(Clone, Copy, Debug)]
pub struct Connector {
    pub kind: Kind,
    pub family: u32,
    pub code: u16,
    pub facing: u8,
    pub upside_down: bool,
}

impl Connector {
    pub fn facing_vector(&self) -> (i32, i32) {
        FACINGS[self.facing as usize]
    }

    pub fn runs_along_x(&self) -> bool {
        self.facing < 2
    }
}

#[derive(Default)]
pub struct Connections {
    members: Vec<Option<Connector>>,
    ids: FxHashMap<(u32, u16), BlockId>,
}

fn kind_of(name: &str) -> Option<Kind> {
    if name == OAK_GATE || name.ends_with(GATE_SUFFIX) {
        Some(Kind::Gate)
    } else if name == NETHER_FENCE {
        Some(Kind::NetherFence)
    } else if name.ends_with(FENCE_SUFFIX) {
        Some(Kind::Fence)
    } else if name.ends_with(STAIRS_SUFFIX) {
        Some(Kind::Stairs)
    } else if name.ends_with(WALL_SUFFIX) {
        Some(Kind::Wall)
    } else if name == IRON_BARS || PANE_SUFFIXES.iter().any(|suffix| name.ends_with(suffix)) {
        Some(Kind::Pane)
    } else {
        None
    }
}

fn text_of(states: &HashMap<CowArc<'static, str>, BlockState>, key: &str) -> Option<String> {
    match states.get(key)? {
        BlockState::Enum(text) => Some(text.to_string()),
        BlockState::Int(number) => Some(number.to_string()),
        BlockState::Bool(flag) => Some(u8::from(*flag).to_string()),
    }
}

fn flag_of(states: &HashMap<CowArc<'static, str>, BlockState>, key: &str) -> Option<bool> {
    match states.get(key)? {
        BlockState::Bool(flag) => Some(*flag),
        BlockState::Int(number) => Some(*number != 0),
        BlockState::Enum(_) => None,
    }
}

fn index_in(options: &[&str], value: &str) -> Option<u16> {
    options.iter().position(|option| *option == value).map(|index| index as u16)
}

fn code_of(kind: Kind, states: &HashMap<CowArc<'static, str>, BlockState>) -> Option<u16> {
    match kind {
        Kind::Fence | Kind::NetherFence | Kind::Pane => SIDE_NAMES.iter().enumerate().try_fold(0u16, |code, (side, name)| {
            let joined = flag_of(states, &format!("{CONNECTION_PREFIX}{name}"))?;
            Some(code | u16::from(joined) << side)
        }),
        Kind::Wall => {
            let sides = SIDE_NAMES.iter().enumerate().try_fold(0u16, |code, (side, name)| {
                let value = text_of(states, &format!("{WALL_PREFIX}{name}"))?;
                Some(code | index_in(&WALL_TYPES, &value)? << (side as u16 * WALL_TYPE_BITS))
            })?;
            Some(sides | u16::from(flag_of(states, WALL_POST)?) << WALL_POST_SHIFT)
        }
        Kind::Stairs => index_in(&CORNERS, &text_of(states, CORNER)?),
        Kind::Gate => Some(0),
    }
}

fn is_axis(kind: Kind, key: &str) -> bool {
    match kind {
        Kind::Fence | Kind::NetherFence | Kind::Pane => key.starts_with(CONNECTION_PREFIX),
        Kind::Wall => key.starts_with(WALL_PREFIX) || key == WALL_POST,
        Kind::Stairs => key == CORNER,
        Kind::Gate => false,
    }
}

fn family_key(name: &str, kind: Kind, states: &HashMap<CowArc<'static, str>, BlockState>, flooded: bool) -> String {
    let mut rest: Vec<String> = states.iter().filter(|(key, _)| !is_axis(kind, key)).map(|(key, value)| format!("{key}={value:?}")).collect();
    rest.sort();
    format!("{name}[{}]{flooded}", rest.join(","))
}

impl Connections {
    pub fn register(table: &mut BlockTable) -> Self {
        let registry = table.registry();
        let mut wanted: HashMap<CowArc<'static, str>, bool> = HashMap::new();
        for entry in table.registered() {
            if kind_of(&entry.name).is_some() {
                *wanted.entry(entry.name.clone()).or_default() |= entry.flooded;
            }
        }
        let mut connections = Self::default();
        let mut families: HashMap<String, u32> = HashMap::new();
        for (name, with_flooded) in wanted {
            let kind = kind_of(&name).expect("a connecting name");
            if kind == Kind::Gate {
                continue;
            }
            for permutation in registry.permutations().filter(|permutation| permutation.get_identifier() == name.as_ref()) {
                let states = permutation.get_states();
                let Some(code) = code_of(kind, states) else { continue };
                let typed: Vec<State> = states.iter().map(|(key, value)| (key.clone(), value.clone())).collect();
                let base = table.get(name.clone(), &typed);
                let flooded = with_flooded.then(|| table.flooded(base));
                for (id, soaked) in std::iter::once((base, false)).chain(flooded.map(|id| (id, true))) {
                    let next = families.len() as u32;
                    let family = *families.entry(family_key(&name, kind, states, soaked)).or_insert(next);
                    let facing = text_of(states, DIRECTION).and_then(|text| text.parse().ok()).unwrap_or(0);
                    let upside_down = flag_of(states, UPSIDE_DOWN).unwrap_or(false);
                    connections.insert(
                        id,
                        Connector {
                            kind,
                            family,
                            code,
                            facing,
                            upside_down,
                        },
                    );
                }
            }
        }
        let gates: Vec<(BlockId, Connector)> = table
            .registered()
            .iter()
            .enumerate()
            .filter(|(_, entry)| kind_of(&entry.name) == Some(Kind::Gate))
            .map(|(id, entry)| (id as BlockId, gate_connector(entry)))
            .collect();
        for (id, connector) in gates {
            connections.insert(id, connector);
        }
        connections
    }

    fn insert(&mut self, id: BlockId, connector: Connector) {
        if self.members.len() <= id as usize {
            self.members.resize(id as usize + 1, None);
        }
        self.members[id as usize] = Some(connector);
        if connector.kind != Kind::Gate {
            self.ids.insert((connector.family, connector.code), id);
        }
    }

    pub fn member(&self, id: BlockId) -> Option<&Connector> {
        self.members.get(id as usize).and_then(Option::as_ref)
    }

    pub fn kind(&self, id: BlockId) -> Option<Kind> {
        self.member(id).map(|connector| connector.kind)
    }

    pub fn recoded(&self, connector: &Connector, code: u16) -> Option<BlockId> {
        self.ids.get(&(connector.family, code)).copied()
    }
}

fn gate_connector(entry: &Entry) -> Connector {
    let across_x = entry
        .states
        .iter()
        .find(|(key, _)| key.as_ref() == CARDINAL)
        .is_some_and(|(_, value)| matches!(value, BlockState::Enum(text) if matches!(text.as_ref(), "north" | "south")));
    Connector {
        kind: Kind::Gate,
        family: u32::MAX,
        code: 0,
        facing: if across_x { 0 } else { 2 },
        upside_down: false,
    }
}
