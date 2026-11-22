use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

const fn one() -> u32 {
    1
}

const fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Family {
    pub rule_sets: BTreeMap<String, RuleSet>,
    pub pools: BTreeMap<String, Pool>,
}

impl Family {
    pub fn parse(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }

    pub fn render(&self) -> String {
        let mut text = serde_json::to_string_pretty(self).expect("a family renders");
        text.push('\n');
        text
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleSet {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<Rule>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub not_in: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub swap_pools: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub swap_actors: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protect: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub from: Input,
    pub to: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub to_states: BTreeMap<String, State>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<Location>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fade: Option<Fade>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Input {
    Block(String),
    Chance { block: String, chance: f32 },
    AnyOf { blocks: Vec<String>, chance: f32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Location {
    NotIn(Vec<String>),
    Block(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    pub const fn index(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Z => 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fade {
    pub axis: Axis,
    pub chance: (f32, f32),
    pub distance: (i32, i32),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum State {
    Bool(bool),
    Int(i64),
    Text(String),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Projection {
    #[default]
    Rigid,
    TerrainMatching,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PostProcess {
    #[default]
    None,
    SupportBlock,
    Beard,
    All,
}

impl PostProcess {
    pub const fn flags(self) -> u8 {
        match self {
            Self::None => 0,
            Self::SupportBlock => 1,
            Self::Beard => 2,
            Self::All => 3,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pool {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback: Option<String>,
    #[serde(default)]
    pub projection: Projection,
    #[serde(default)]
    pub post_process: PostProcess,
    pub elements: Vec<Element>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Element {
    Template {
        location: String,
        #[serde(default = "one")]
        weight: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rules: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        projection: Option<Projection>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        post_process: Option<PostProcess>,
        #[serde(default, skip_serializing_if = "is_false")]
        reload_actor_loot_tables: bool,
    },
    Feature {
        feature: String,
        #[serde(default = "one")]
        weight: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        projection: Option<Projection>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        post_process: Option<PostProcess>,
    },
    Empty {
        #[serde(default = "one")]
        weight: u32,
    },
}

impl Element {
    pub const fn weight(&self) -> u32 {
        match self {
            Self::Template { weight, .. } | Self::Feature { weight, .. } | Self::Empty { weight } => *weight,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_family_survives_a_round_trip() {
        let mut family = Family::default();
        family.rule_sets.insert(
            "moss".to_owned(),
            RuleSet {
                rules: vec![Rule {
                    from: Input::Chance {
                        block: "cobblestone".to_owned(),
                        chance: 0.1,
                    },
                    to: "mossy_cobblestone".to_owned(),
                    to_states: BTreeMap::from([
                        ("growth".to_owned(), State::Int(7)),
                        ("lit".to_owned(), State::Bool(true)),
                        ("facing".to_owned(), State::Text("south".to_owned())),
                    ]),
                    at: Some(Location::NotIn(vec!["bedrock".to_owned()])),
                    fade: Some(Fade {
                        axis: Axis::Y,
                        chance: (0.0, 0.05),
                        distance: (0, 100),
                    }),
                }],
                ..RuleSet::default()
            },
        );
        family.pools.insert(
            "village/plains/houses".to_owned(),
            Pool {
                fallback: None,
                projection: Projection::TerrainMatching,
                post_process: PostProcess::All,
                elements: vec![
                    Element::Template {
                        location: "a".to_owned(),
                        weight: 2,
                        rules: Some("moss".to_owned()),
                        projection: None,
                        post_process: None,
                        reload_actor_loot_tables: true,
                    },
                    Element::Empty { weight: 1 },
                ],
            },
        );
        let text = family.render();
        let parsed = Family::parse(&text).expect("parses");
        assert_eq!(parsed.render(), text);
        assert_eq!(parsed.rule_sets["moss"].rules[0].to_states["growth"], State::Int(7));
    }
}
