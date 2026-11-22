use super::structure::data::embedded_packs;
use chorus_structure_data::{PackSet, list, read_text};
use serde_json::Value;
use std::sync::LazyLock;

include!(concat!(env!("OUT_DIR"), "/chorus_files.rs"));

const JSON_SUFFIX: &str = ".json";

pub static PACKS: LazyLock<PackSet> = LazyLock::new(embedded_packs);

pub fn text(name: &str) -> String {
    read_text(&*PACKS, name).unwrap_or_else(|error| panic!("embedded resource {name} is unreadable: {error}"))
}

pub fn json_documents(folder: &str, owner: &str) -> Vec<(String, Value)> {
    list(&*PACKS, folder, JSON_SUFFIX, false)
        .into_iter()
        .filter(|name| PACKS.owner(name) == Some(owner))
        .map(|name| {
            let document = serde_json::from_str(&text(&name)).unwrap_or_else(|error| panic!("embedded document {name} is not valid JSON: {error}"));
            (name, document)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chorus_files_are_sorted_and_cover_every_folder() {
        let names: Vec<&str> = CHORUS_FILES.iter().map(|(name, _)| *name).collect();
        assert!(names.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(!names.contains(&"README.md"));
        for folder in ["features/", "structure_sets/"] {
            assert!(names.iter().any(|name| name.starts_with(folder)), "{folder}");
        }
        assert!(names.iter().any(|name| name.starts_with("features/") && name.ends_with(".json")));
        assert!(names.contains(&"structure_sets/trial_chambers.json"));
    }

    #[test]
    fn vanilla_documents_are_present() {
        assert!(text("climate/biome_climate.json").starts_with('{'));
        assert_eq!(json_documents("features", PackSet::VANILLA).len(), 290);
        assert_eq!(json_documents("feature_rules", PackSet::VANILLA).len(), 267);
        assert_eq!(json_documents("biomes", PackSet::VANILLA).len(), 89);
    }

    #[test]
    fn the_chorus_pack_adds_features_without_replacing_vanilla_ones() {
        assert_eq!(json_documents("features", crate::shared::structure::data::CHORUS_PACK).len(), 21);
    }
}
