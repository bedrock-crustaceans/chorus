use std::collections::HashMap;
use std::sync::LazyLock;

use bedrock::protocol::v800::packets::{BiomeDefinitionListPacket, BiomeEntry};
use bedrock::protocol::v844::types::{BiomeDefinition, BiomeTagList};
use serde::Deserialize;

use crate::network::BedrockProtocol;

const BIOME_DEFINITIONS: &str = include_str!("../../resources/biome_definitions.json");

#[derive(Deserialize)]
struct ClientBiome {
    name: String,
    id: i16,
    temperature: f32,
    downfall: f32,
    foliage_snow: f32,
    depth: f32,
    scale: f32,
    map_water_color: i32,
    rain: bool,
    tags: Option<Vec<String>>,
}

static DEFINITION_LIST: LazyLock<BiomeDefinitionListPacket<BedrockProtocol>> = LazyLock::new(|| {
    let biomes: Vec<ClientBiome> = serde_json::from_str(BIOME_DEFINITIONS).expect("bundled biome definitions are valid json");
    let mut strings = Vec::new();
    let mut indices = HashMap::new();
    let mut intern = |value: &str| -> u16 {
        *indices.entry(value.to_owned()).or_insert_with(|| {
            strings.push(value.to_owned());
            (strings.len() - 1) as u16
        })
    };
    let entries = biomes
        .iter()
        .map(|biome| BiomeEntry {
            name_index: intern(&biome.name),
            definition: BiomeDefinition {
                id: biome.id as u16,
                temperature: biome.temperature,
                downfall: biome.downfall,
                foliage_snow: biome.foliage_snow,
                depth: biome.depth,
                scale: biome.scale,
                map_water_color: biome.map_water_color,
                rain: biome.rain,
                tags: biome.tags.as_ref().map(|tags| BiomeTagList {
                    tags: tags.iter().map(|tag| intern(tag)).collect(),
                }),
                chunk_gen_data: None,
            },
        })
        .collect();
    BiomeDefinitionListPacket { biomes: entries, strings }
});

pub fn definition_list() -> BiomeDefinitionListPacket<BedrockProtocol> {
    DEFINITION_LIST.clone()
}

#[cfg(test)]
mod tests {
    #[test]
    fn bundled_definitions_cover_every_biome_id() {
        let packet = super::definition_list();
        let names: Vec<&str> = packet.biomes.iter().map(|biome| packet.strings[biome.name_index as usize].as_str()).collect();
        assert_eq!(names.len(), 89);
        for name in ["minecraft:jungle", "minecraft:dappled_forest", "minecraft:sulfur_caves", "minecraft:pale_garden"] {
            assert!(names.contains(&name), "{name} missing");
        }
    }
}
