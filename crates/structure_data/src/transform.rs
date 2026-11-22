use std::collections::BTreeMap;

const COMPASS: [&str; 4] = ["north", "east", "south", "west"];
const TORCH: [&str; 4] = ["south", "west", "north", "east"];
const CARDINAL_KEY: &str = "minecraft:cardinal_direction";
const DIRECTION_KEY: &str = "direction";
const UPPER_HALF_KEY: &str = "upper_block_bit";
const TRAPDOOR_CLOCKWISE: [usize; 4] = [2, 3, 1, 0];
const TRAPDOOR_BASE: [usize; 4] = [3, 1, 0, 2];

type Swap = [(&'static str, &'static str); 2];

const NORTH_SOUTH: Swap = [("north", "south"), ("south", "north")];
const EAST_WEST: Swap = [("east", "west"), ("west", "east")];
const NUMBERED_Z: Swap = [("2", "3"), ("3", "2")];
const NUMBERED_X: Swap = [("4", "5"), ("5", "4")];
const WEIRDO_Z: Swap = [("2", "3"), ("3", "2")];
const WEIRDO_X: Swap = [("0", "1"), ("1", "0")];
const DIRECTION_Z: Swap = [("0", "2"), ("2", "0")];
const DIRECTION_X: Swap = [("1", "3"), ("3", "1")];

#[derive(Clone, Copy, Debug)]
pub enum Cycle {
    Compass,
    Torch,
    Weirdo,
    Facing,
    Quarter,
    Axis,
    VineBits,
}

#[derive(Clone, Copy, Debug)]
pub enum Names {
    Any,
    Exact(&'static str),
    Suffix(&'static str),
}

impl Names {
    fn matches(self, name: &str) -> bool {
        match self {
            Names::Any => true,
            Names::Exact(exact) => name == exact,
            Names::Suffix(suffix) => name.ends_with(suffix),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Rotation {
    pub key: &'static str,
    pub names: Names,
    pub cycle: Cycle,
}

#[derive(Clone, Copy, Debug)]
pub struct Mirror {
    pub key: &'static str,
    pub along_z: Swap,
    pub along_x: Swap,
}

#[derive(Clone, Copy, Debug)]
pub struct Hold {
    pub names: Names,
    pub when_key: &'static str,
    pub when_value: &'static str,
    pub key: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub enum Fixup {
    SetValue { key: &'static str, value: &'static str },
    HalfTurn { key: &'static str },
    TrapdoorFacing,
    DoorHalf,
}

#[derive(Clone, Copy, Debug)]
pub struct Placement {
    pub names: Names,
    pub fixup: Fixup,
}

pub struct StateTransform {
    pub rotations: &'static [Rotation],
    pub mirrors: &'static [Mirror],
    pub sides: &'static [&'static str],
    pub holds: &'static [Hold],
    pub placements: &'static [Placement],
}

pub static STATE_TRANSFORM: StateTransform = StateTransform {
    rotations: &[
        Rotation {
            key: CARDINAL_KEY,
            names: Names::Any,
            cycle: Cycle::Compass,
        },
        Rotation {
            key: "pillar_axis",
            names: Names::Any,
            cycle: Cycle::Axis,
        },
        Rotation {
            key: "weirdo_direction",
            names: Names::Any,
            cycle: Cycle::Weirdo,
        },
        Rotation {
            key: "facing_direction",
            names: Names::Any,
            cycle: Cycle::Facing,
        },
        Rotation {
            key: DIRECTION_KEY,
            names: Names::Suffix("trapdoor"),
            cycle: Cycle::Weirdo,
        },
        Rotation {
            key: DIRECTION_KEY,
            names: Names::Any,
            cycle: Cycle::Quarter,
        },
        Rotation {
            key: "torch_facing_direction",
            names: Names::Any,
            cycle: Cycle::Torch,
        },
        Rotation {
            key: "lever_direction",
            names: Names::Any,
            cycle: Cycle::Compass,
        },
        Rotation {
            key: "vine_direction_bits",
            names: Names::Any,
            cycle: Cycle::VineBits,
        },
    ],
    mirrors: &[
        Mirror {
            key: CARDINAL_KEY,
            along_z: NORTH_SOUTH,
            along_x: EAST_WEST,
        },
        Mirror {
            key: "weirdo_direction",
            along_z: WEIRDO_Z,
            along_x: WEIRDO_X,
        },
        Mirror {
            key: "facing_direction",
            along_z: NUMBERED_Z,
            along_x: NUMBERED_X,
        },
        Mirror {
            key: DIRECTION_KEY,
            along_z: DIRECTION_Z,
            along_x: DIRECTION_X,
        },
        Mirror {
            key: "torch_facing_direction",
            along_z: NORTH_SOUTH,
            along_x: EAST_WEST,
        },
    ],
    sides: &["wall_connection_type_", "minecraft:connection_"],
    holds: &[Hold {
        names: Names::Suffix("_door"),
        when_key: UPPER_HALF_KEY,
        when_value: "1",
        key: CARDINAL_KEY,
    }],
    placements: &[
        Placement {
            names: Names::Exact("trial_spawner"),
            fixup: Fixup::SetValue {
                key: "trial_spawner_state",
                value: "1",
            },
        },
        Placement {
            names: Names::Exact("decorated_pot"),
            fixup: Fixup::HalfTurn { key: DIRECTION_KEY },
        },
        Placement {
            names: Names::Suffix("trapdoor"),
            fixup: Fixup::TrapdoorFacing,
        },
        Placement {
            names: Names::Suffix("_door"),
            fixup: Fixup::DoorHalf,
        },
    ],
};

fn clockwise(order: &[&str; 4], value: &str, turns: usize) -> String {
    order
        .iter()
        .position(|name| *name == value)
        .map_or_else(|| value.to_owned(), |index| order[(index + turns) % 4].to_owned())
}

fn weirdo_step(value: u32) -> u32 {
    match value {
        0 => 2,
        2 => 1,
        1 => 3,
        3 => 0,
        other => other,
    }
}

fn facing_step(value: u32) -> u32 {
    match value {
        5 => 3,
        3 => 4,
        4 => 2,
        2 => 5,
        other => other,
    }
}

fn repeat(value: &str, turns: usize, step: fn(u32) -> u32) -> String {
    value
        .parse::<u32>()
        .map_or_else(|_| value.to_owned(), |number| (0..turns).fold(number, |current, _| step(current)).to_string())
}

fn vine_bits(value: &str, turns: usize) -> String {
    value
        .parse::<u32>()
        .map_or_else(|_| value.to_owned(), |bits| (((bits << turns) | (bits >> (4 - turns))) & 15).to_string())
}

impl Cycle {
    fn turn(self, value: &str, turns: usize) -> String {
        match self {
            Cycle::Compass => clockwise(&COMPASS, value, turns),
            Cycle::Torch => clockwise(&TORCH, value, turns),
            Cycle::Weirdo => repeat(value, turns, weirdo_step),
            Cycle::Facing => repeat(value, turns, facing_step),
            Cycle::Quarter => value.parse::<usize>().map_or_else(|_| value.to_owned(), |number| ((number + turns) % 4).to_string()),
            Cycle::Axis if turns % 2 == 1 => match value {
                "x" => "z".to_owned(),
                "z" => "x".to_owned(),
                other => other.to_owned(),
            },
            Cycle::Axis => value.to_owned(),
            Cycle::VineBits => vine_bits(value, turns),
        }
    }
}

fn value<'a>(states: &'a str, key: &str) -> Option<&'a str> {
    states.split(',').filter_map(|pair| pair.split_once('=')).find(|(name, _)| *name == key).map(|(_, value)| value)
}

fn with_value(states: &str, key: &str, new: &str) -> String {
    states
        .split(',')
        .map(|pair| match pair.split_once('=') {
            Some((name, _)) if name == key => format!("{name}={new}"),
            _ => pair.to_string(),
        })
        .collect::<Vec<_>>()
        .join(",")
}

impl Fixup {
    fn apply(self, unturned: &str, rotated: &str, rotation: usize) -> Option<String> {
        match self {
            Fixup::SetValue { key, value } => Some(with_value(rotated, key, value)),
            Fixup::HalfTurn { key } => {
                let turn: usize = value(rotated, key)?.parse().ok()?;
                Some(with_value(rotated, key, &((turn + 2) % 4).to_string()))
            }
            Fixup::TrapdoorFacing => {
                let direction: usize = value(unturned, DIRECTION_KEY)?.parse().ok()?;
                let turned = (0..rotation).fold(TRAPDOOR_BASE[direction], |facing, _| TRAPDOOR_CLOCKWISE[facing]);
                Some(with_value(rotated, DIRECTION_KEY, &turned.to_string()))
            }
            Fixup::DoorHalf => {
                let at = COMPASS.iter().position(|name| Some(*name) == value(rotated, CARDINAL_KEY))?;
                let turned = if value(rotated, UPPER_HALF_KEY) == Some("1") { (2 + rotation) % 4 } else { (at + 3) % 4 };
                Some(with_value(rotated, CARDINAL_KEY, COMPASS[turned]))
            }
        }
    }
}

impl StateTransform {
    fn side_of(&self, key: &str) -> Option<(&'static str, usize)> {
        self.sides
            .iter()
            .find_map(|prefix| key.strip_prefix(prefix).and_then(|side| COMPASS.iter().position(|name| *name == side).map(|index| (*prefix, index))))
    }

    pub fn rotate(&self, name: &str, states: &BTreeMap<String, String>, rotation: usize) -> BTreeMap<String, String> {
        let turns = rotation % 4;
        let held = self
            .holds
            .iter()
            .find(|hold| hold.names.matches(name) && states.get(hold.when_key).is_some_and(|bit| bit == hold.when_value));
        let mut turned = BTreeMap::new();
        for (key, value) in states {
            if held.is_some_and(|hold| hold.key == key) {
                turned.insert(key.clone(), value.clone());
                continue;
            }
            match self.side_of(key) {
                Some((prefix, index)) => {
                    turned.insert(format!("{prefix}{}", COMPASS[(index + turns) % 4]), value.clone());
                }
                None => {
                    let rule = self.rotations.iter().find(|rule| rule.key == key && rule.names.matches(name));
                    turned.insert(key.clone(), rule.map_or_else(|| value.clone(), |rule| rule.cycle.turn(value, turns)));
                }
            }
        }
        turned
    }

    pub fn mirror(&self, states: &str, along_z: bool) -> String {
        let (a, b) = if along_z { ("north", "south") } else { ("east", "west") };
        states
            .split(',')
            .map(|pair| {
                let Some((key, current)) = pair.split_once('=') else { return pair.to_string() };
                let sided = self.sides.iter().find_map(|prefix| key.strip_prefix(prefix).map(|side| (prefix, side)));
                if let Some((prefix, side)) = sided {
                    let other = if side == a {
                        value(states, &format!("{prefix}{b}"))
                    } else if side == b {
                        value(states, &format!("{prefix}{a}"))
                    } else {
                        None
                    };
                    if let Some(other) = other {
                        return format!("{key}={other}");
                    }
                }
                let swaps = self
                    .mirrors
                    .iter()
                    .find(|mirror| mirror.key == key)
                    .map(|mirror| if along_z { &mirror.along_z } else { &mirror.along_x });
                let swapped = swaps.and_then(|pairs| pairs.iter().find(|(from, _)| *from == current)).map_or(current, |(_, to)| to);
                format!("{key}={swapped}")
            })
            .collect::<Vec<_>>()
            .join(",")
    }

    pub fn placement(&self, name: &str, unturned: &str, rotated: &str, rotation: usize) -> Option<String> {
        let placement = self.placements.iter().find(|placement| placement.names.matches(name))?;
        placement.fixup.apply(unturned, rotated, rotation)
    }

    pub fn state_keys(&self) -> Vec<String> {
        let mut keys = Vec::new();
        keys.extend(self.rotations.iter().map(|rule| rule.key.to_owned()));
        keys.extend(self.mirrors.iter().map(|mirror| mirror.key.to_owned()));
        keys.extend(self.holds.iter().flat_map(|hold| [hold.when_key.to_owned(), hold.key.to_owned()]));
        keys.extend(self.placements.iter().flat_map(|placement| match placement.fixup {
            Fixup::SetValue { key, .. } | Fixup::HalfTurn { key } => vec![key.to_owned()],
            Fixup::TrapdoorFacing => vec![DIRECTION_KEY.to_owned()],
            Fixup::DoorHalf => vec![CARDINAL_KEY.to_owned(), UPPER_HALF_KEY.to_owned()],
        }));
        keys.extend(self.sides.iter().flat_map(|prefix| COMPASS.iter().map(move |side| format!("{prefix}{side}"))));
        keys.sort();
        keys.dedup();
        keys
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn states(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs.iter().map(|(key, value)| ((*key).to_owned(), (*value).to_owned())).collect()
    }

    #[test]
    fn doors_and_stairs_turn_clockwise() {
        let door = states(&[(CARDINAL_KEY, "south")]);
        assert_eq!(STATE_TRANSFORM.rotate("acacia_door", &door, 1)[CARDINAL_KEY], "west");
        let stairs = states(&[("weirdo_direction", "0")]);
        assert_eq!(STATE_TRANSFORM.rotate("stone_stairs", &stairs, 2)["weirdo_direction"], "1");
    }

    #[test]
    fn sides_move_with_the_turn() {
        let wall = states(&[("wall_connection_type_north", "tall"), ("wall_connection_type_east", "none")]);
        let turned = STATE_TRANSFORM.rotate("cobblestone_wall", &wall, 1);
        assert_eq!(turned["wall_connection_type_east"], "tall");
        assert_eq!(turned["wall_connection_type_south"], "none");
    }

    #[test]
    fn trapdoors_use_their_own_cycle() {
        let trapdoor = states(&[("direction", "0")]);
        assert_eq!(STATE_TRANSFORM.rotate("spruce_trapdoor", &trapdoor, 1)["direction"], "2");
        assert_eq!(STATE_TRANSFORM.rotate("bed", &trapdoor, 1)["direction"], "1");
    }

    #[test]
    fn mirroring_swaps_values_and_sides() {
        assert_eq!(STATE_TRANSFORM.mirror("minecraft:cardinal_direction=north", true), "minecraft:cardinal_direction=south");
        assert_eq!(STATE_TRANSFORM.mirror("minecraft:cardinal_direction=north", false), "minecraft:cardinal_direction=north");
        let sides = "wall_connection_type_east=tall,wall_connection_type_north=none,wall_connection_type_south=short,wall_connection_type_west=none";
        assert_eq!(
            STATE_TRANSFORM.mirror(sides, true),
            "wall_connection_type_east=tall,wall_connection_type_north=short,wall_connection_type_south=none,wall_connection_type_west=none"
        );
    }

    #[test]
    fn placement_fixups_follow_the_block_name() {
        let spawner = STATE_TRANSFORM.placement("trial_spawner", "ominous=0,trial_spawner_state=0", "ominous=0,trial_spawner_state=0", 0);
        assert_eq!(spawner.as_deref(), Some("ominous=0,trial_spawner_state=1"));
        assert_eq!(STATE_TRANSFORM.placement("stone", "", "", 0), None);
    }
}
