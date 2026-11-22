use crate::bedrock::overworld::surface::data;
use crate::shared::biome::Biome;
use rustc_hash::FxHashSet;
use serde_json::Value;

const ANY_OF: &str = "any_of";
const ALL_OF: &str = "all_of";
const TEST: &str = "test";
const OPERATOR: &str = "operator";
const VALUE: &str = "value";
const HAS_BIOME_TAG: &str = "has_biome_tag";
const POSITIVE_OPERATORS: [&str; 2] = ["==", "equals"];
const NEGATIVE_OPERATORS: [&str; 3] = ["!=", "not", "not_equals"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BiomeMask {
    Any,
    Only(FxHashSet<Biome>),
}

impl Default for BiomeMask {
    fn default() -> Self {
        Self::Only(FxHashSet::default())
    }
}

impl BiomeMask {
    pub fn insert(&mut self, biome: Biome) {
        if let Self::Only(biomes) = self {
            biomes.insert(biome);
        }
    }

    pub fn contains(&self, biome: &Biome) -> bool {
        match self {
            Self::Any => true,
            Self::Only(biomes) => biomes.contains(biome),
        }
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Only(biomes) if biomes.is_empty())
    }

    #[cfg(test)]
    pub fn count(&self) -> usize {
        match self {
            Self::Any => Biome::ALL.len(),
            Self::Only(biomes) => biomes.len(),
        }
    }
}

#[derive(Debug)]
pub enum Filter {
    Tag { tag: String, negated: bool },
    AnyOf(Vec<Filter>),
    AllOf(Vec<Filter>),
    Unsupported(String),
}

impl Filter {
    pub fn parse(value: &Value) -> Self {
        match value {
            Value::Array(items) => Self::AllOf(items.iter().map(Self::parse).collect()),
            Value::Object(map) => {
                if let Some(items) = map.get(ANY_OF).and_then(Value::as_array) {
                    return Self::AnyOf(items.iter().map(Self::parse).collect());
                }
                if let Some(items) = map.get(ALL_OF).and_then(Value::as_array) {
                    return Self::AllOf(items.iter().map(Self::parse).collect());
                }
                Self::parse_test(value)
            }
            _ => Self::Unsupported(value.to_string()),
        }
    }

    fn parse_test(value: &Value) -> Self {
        let test = value[TEST].as_str().unwrap_or_default();
        let operator = value[OPERATOR].as_str().unwrap_or(POSITIVE_OPERATORS[0]);
        let Some(tag) = value[VALUE].as_str().filter(|_| test == HAS_BIOME_TAG) else {
            return Self::Unsupported(format!("test {test}"));
        };
        if POSITIVE_OPERATORS.contains(&operator) {
            Self::Tag { tag: tag.to_string(), negated: false }
        } else if NEGATIVE_OPERATORS.contains(&operator) {
            Self::Tag { tag: tag.to_string(), negated: true }
        } else {
            Self::Unsupported(format!("operator {operator}"))
        }
    }

    pub fn matches(&self, tags: &[&str]) -> bool {
        match self {
            Self::Tag { tag, negated } => tags.contains(&tag.as_str()) != *negated,
            Self::AnyOf(filters) => filters.iter().any(|filter| filter.matches(tags)),
            Self::AllOf(filters) => filters.iter().all(|filter| filter.matches(tags)),
            Self::Unsupported(_) => false,
        }
    }

    pub fn collect_unsupported(&self, into: &mut Vec<String>) {
        match self {
            Self::Unsupported(reason) => into.push(reason.clone()),
            Self::AnyOf(filters) | Self::AllOf(filters) => filters.iter().for_each(|filter| filter.collect_unsupported(into)),
            Self::Tag { .. } => {}
        }
    }

    pub fn mask(&self) -> BiomeMask {
        let mut mask = BiomeMask::default();
        for biome in Biome::ALL {
            let tags: Vec<&str> = data::tags(&biome).collect();
            if self.matches(&tags) {
                mask.insert(biome);
            }
        }
        mask
    }
}
