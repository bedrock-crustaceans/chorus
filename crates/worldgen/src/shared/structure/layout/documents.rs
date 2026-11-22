use std::collections::HashMap;
use std::error::Error;

use crate::shared::structure::data::jigsaw::{Element, Family};
use serde_json::Value;

use crate::shared::structure::data::{PackBiomes, PackWorldgen, STRUCTURE_DOCUMENT_SUFFIX};
use chorus_structure_data::{PackSet, PackSource, list, read_text};

const FAMILIES_DIRECTORY: &str = "jigsaw";
const NAMESPACE: &str = "minecraft:";
const FEATURE_ELEMENT: &str = "minecraft:feature_pool_element";

pub fn bare(name: &str) -> &str {
    name.strip_prefix(NAMESPACE).unwrap_or(name)
}

#[derive(Default)]
pub struct PoolDocuments {
    pub pools: Vec<Value>,
    pub families: Vec<Family>,
    pub processors: HashMap<String, Value>,
    pub structures: Vec<Value>,
    pub structure_sets: Vec<Value>,
    pub biomes: PackBiomes,
}

fn read_families(source: &dyn PackSource) -> Result<Vec<Family>, Box<dyn Error>> {
    list(source, FAMILIES_DIRECTORY, ".json", true)
        .iter()
        .filter(|name| !name.ends_with(STRUCTURE_DOCUMENT_SUFFIX))
        .map(|name| Family::parse(&read_text(source, name)?).map_err(|error| format!("{name}: {error}").into()))
        .collect()
}

pub fn identifier(document: &Value) -> &str {
    document["description"]["identifier"].as_str().expect("a document identifier")
}

impl PoolDocuments {
    pub fn load(packs: &PackSet) -> Result<Self, Box<dyn Error>> {
        let worldgen = PackWorldgen::load(packs)?;
        Ok(Self {
            pools: worldgen.pools,
            families: read_families(packs)?,
            processors: Self::by_name(worldgen.processors),
            structures: worldgen.structures,
            structure_sets: worldgen.structure_sets,
            biomes: worldgen.biomes,
        })
    }

    fn by_name(processors: Vec<Value>) -> HashMap<String, Value> {
        processors.into_iter().map(|document| (bare(identifier(&document)).to_string(), document)).collect()
    }

    pub fn feature_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .pools
            .iter()
            .flat_map(|pool| pool["elements"].as_array().into_iter().flatten())
            .map(|entry| &entry["element"])
            .filter(|element| element["element_type"] == FEATURE_ELEMENT)
            .map(|element| bare(element["feature"].as_str().expect("a feature name")).to_string())
            .collect();
        names.extend(
            self.families
                .iter()
                .flat_map(|family| family.pools.values())
                .flat_map(|pool| pool.elements.iter())
                .filter_map(|element| match element {
                    Element::Feature { feature, .. } => Some(bare(feature).to_string()),
                    _ => None,
                }),
        );
        names.sort();
        names.dedup();
        names
    }
}
