use chorus_structure_data::PackSet;
use serde_json::Value;
use std::sync::LazyLock;

const BIOMES_FOLDER: &str = "biomes";
const BIOME: &str = "minecraft:biome";
const NAMESPACE: &str = "minecraft:";
const COMPONENTS: &str = "components";
const TAGS: &str = "minecraft:tags";

pub static ROOT: LazyLock<Value> = LazyLock::new(components_by_pack_name);

fn components_by_pack_name() -> Value {
    let mut biomes = serde_json::Map::new();
    for (name, mut document) in crate::shared::resources::json_documents(BIOMES_FOLDER, PackSet::VANILLA) {
        let body = document.get_mut(BIOME).unwrap_or_else(|| panic!("{name}: missing {BIOME}"));
        let identifier = body["description"]["identifier"].as_str().unwrap_or_else(|| panic!("{name}: missing identifier"));
        let bare = identifier.strip_prefix(NAMESPACE).unwrap_or(identifier).to_owned();
        biomes.insert(bare, serde_json::json!({ COMPONENTS: body[COMPONENTS].take() }));
    }
    Value::Object(biomes)
}

pub fn tags(pack: &str) -> impl Iterator<Item = &'static str> {
    let root: &'static Value = &ROOT;
    root.get(pack)
        .and_then(|entry| entry[COMPONENTS].get(TAGS))
        .and_then(|tags| tags["tags"].as_array())
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_pack_biome_has_a_document_and_the_nether_biomes_carry_their_tags() {
        assert_eq!(ROOT.as_object().map(|map| map.len()), Some(89));
        assert!(tags("hell").any(|tag| tag == "nether_wastes"));
        assert!(tags("crimson_forest").any(|tag| tag == "netherwart_forest"));
        assert_eq!(tags("not_a_biome").count(), 0);
    }
}
