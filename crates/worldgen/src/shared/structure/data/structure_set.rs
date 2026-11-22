use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

fn parse<T: serde::de::DeserializeOwned>(text: &str) -> Result<T, serde_json::Error> {
    serde_json::from_str(text)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructureSet {
    pub identifier: String,
    pub kind: String,
    pub rule: PlacementRule,
    pub placement: Placement,
    pub reach: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub biome_filter: Option<BiomeFilter>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adaptation: Option<Adaptation>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub biome_tags: BTreeMap<String, Vec<String>>,
}

impl StructureSet {
    pub fn parse(text: &str) -> Result<Self, serde_json::Error> {
        parse(text)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlacementRule {
    Bedrock,
    Java,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Spread {
    Linear,
    Triangular,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Placement {
    RandomSpread { spacing: i32, separation: i32, salt: i32, spread: Spread },
    ConcentricRings,
    PerChunk,
    Pack,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BiomeFilter {
    pub tag: String,
    pub radius: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Adaptation {
    pub mode: String,
    pub order: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_structure_set_parses() {
        let set = StructureSet::parse(
            r#"{"identifier": "village", "kind": "village", "rule": "bedrock", "placement": {"random_spread": {"spacing": 34, "separation": 8, "salt": 10387312, "spread": "triangular"}}, "reach": 9, "biome_filter": {"tag": "village", "radius": 2}, "biome_tags": {"village": ["plains", "desert"]}}"#,
        )
        .expect("parses");
        assert_eq!(
            set.placement,
            Placement::RandomSpread {
                spacing: 34,
                separation: 8,
                salt: 10387312,
                spread: Spread::Triangular
            }
        );
        assert_eq!(set.biome_filter.expect("a filter").radius, 2);
        assert_eq!(set.biome_tags["village"], ["plains", "desert"]);
    }
}
