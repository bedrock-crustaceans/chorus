use crate::shared::biome::BiomeRegistry;

pub fn pack_biome_id(identifier: &str) -> Option<i32> {
    BiomeRegistry::vanilla().pack_id(identifier)
}
