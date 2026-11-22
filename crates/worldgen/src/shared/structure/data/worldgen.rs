use std::collections::HashMap;
use std::error::Error;

use serde_json::Value;

use super::biome_ids::pack_biome_id;
use chorus_structure_data::{PackSet, PackSource};

const WORLDGEN_FOLDER: &str = "worldgen";
const BIOMES_FOLDER: &str = "biomes";
const JIGSAW_FOLDER: &str = "jigsaw";
pub const STRUCTURE_DOCUMENT_SUFFIX: &str = "_structure.json";
const BIOME_SUFFIX: &str = ".biome.json";
const JSON_SUFFIX: &str = ".json";
const NAMESPACE: &str = "minecraft:";
const POOL_KEY: &str = "minecraft:template_pool";
const PROCESSOR_KEY: &str = "minecraft:processor_list";
const STRUCTURE_KEY: &str = "minecraft:jigsaw";
const STRUCTURE_SET_KEY: &str = "minecraft:structure_set";
const BIOME_KEY: &str = "minecraft:biome";
const TAGS_KEY: &str = "minecraft:tags";

#[derive(Default)]
pub struct PackBiomes {
    entries: Vec<(String, Vec<String>)>,
}

impl PackBiomes {
    pub fn ids_with_all(&self, tags: &[&str]) -> Vec<i32> {
        let mut ids: Vec<i32> = self
            .entries
            .iter()
            .filter(|(_, own)| tags.iter().all(|tag| own.iter().any(|candidate| candidate == tag)))
            .filter_map(|(identifier, _)| pack_biome_id(identifier))
            .collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    pub fn ids_with(&self, tag: &str) -> Vec<i32> {
        self.ids_with_all(&[tag])
    }

    pub fn tags(&self) -> Vec<&str> {
        let mut tags: Vec<&str> = self.entries.iter().flat_map(|(_, own)| own.iter().map(String::as_str)).collect();
        tags.sort_unstable();
        tags.dedup();
        tags
    }
}

#[derive(Default)]
pub struct PackWorldgen {
    pub pools: Vec<Value>,
    pub processors: Vec<Value>,
    pub structures: Vec<Value>,
    pub structure_sets: Vec<Value>,
    pub biomes: PackBiomes,
}

fn wanted(key: &str) -> bool {
    let mut parts = key.split('/');
    match (parts.next(), parts.next()) {
        (Some(WORLDGEN_FOLDER), Some("template_pools" | "structures" | "structure_sets" | "processors")) => key.ends_with(JSON_SUFFIX),
        (Some(BIOMES_FOLDER), Some(name)) => parts.next().is_none() && name.ends_with(BIOME_SUFFIX),
        (Some(JIGSAW_FOLDER), Some(name)) => parts.next().is_none() && name.ends_with(STRUCTURE_DOCUMENT_SUFFIX),
        _ => false,
    }
}

fn documents(bundle: &dyn PackSource) -> Vec<(String, Vec<u8>)> {
    let mut keys: Vec<String> = bundle.names().into_iter().filter(|key| wanted(key)).collect();
    keys.sort();
    keys.into_iter().filter_map(|key| bundle.read(&key).map(|bytes| (key, bytes))).collect()
}

fn inner(key: &str, bytes: &[u8], root: &str) -> Result<Value, Box<dyn Error>> {
    let mut document: Value = serde_json::from_slice(bytes).map_err(|error| format!("{key}: {error}"))?;
    document.get_mut(root).map(Value::take).ok_or_else(|| format!("{key}: missing {root}").into())
}

impl PackWorldgen {
    pub fn load(packs: &PackSet) -> Result<Self, Box<dyn Error>> {
        let mut loaded = Self::default();
        let mut seen: HashMap<(String, String), String> = HashMap::new();
        for (key, bytes) in documents(packs) {
            let folder = key.split('/').nth(1).unwrap_or_default();
            let section = key.split('/').next();
            let kind = match (section, folder) {
                (Some(WORLDGEN_FOLDER), "template_pools") => POOL_KEY,
                (Some(WORLDGEN_FOLDER), "processors") => PROCESSOR_KEY,
                (Some(WORLDGEN_FOLDER), "structures") => STRUCTURE_KEY,
                (Some(WORLDGEN_FOLDER), "structure_sets") => STRUCTURE_SET_KEY,
                (Some(JIGSAW_FOLDER), _) => STRUCTURE_KEY,
                _ => BIOME_KEY,
            };
            let document = inner(&key, &bytes, kind)?;
            let identifier = document["description"]["identifier"].as_str().ok_or_else(|| format!("{key}: missing identifier"))?;
            let bare = identifier.strip_prefix(NAMESPACE).unwrap_or(identifier).to_owned();
            if let Some(first) = seen.insert((kind.to_owned(), bare.clone()), key.clone()) {
                let owner = |file: &str| packs.owner(file).unwrap_or_default().to_owned();
                return Err(format!("{kind} {identifier} is defined by pack {} ({first}) and pack {} ({key})", owner(&first), owner(&key)).into());
            }
            match kind {
                POOL_KEY => loaded.pools.push(document),
                PROCESSOR_KEY => loaded.processors.push(document),
                STRUCTURE_KEY => loaded.structures.push(document),
                STRUCTURE_SET_KEY => loaded.structure_sets.push(document),
                _ => {
                    let tags = document["components"][TAGS_KEY]["tags"]
                        .as_array()
                        .map(|tags| tags.iter().filter_map(Value::as_str).map(str::to_owned).collect())
                        .unwrap_or_default();
                    loaded.biomes.entries.push((bare, tags));
                }
            }
        }
        if loaded.pools.is_empty() {
            return Err("no worldgen documents in the bundle".into());
        }
        Ok(loaded)
    }
}
