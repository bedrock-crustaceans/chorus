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
    fn the_chorus_pack_adds_features_without_replacing_vanilla_ones() {
        assert_eq!(json_documents("features", crate::shared::structure::data::CHORUS_PACK).len(), 21);
    }
}
