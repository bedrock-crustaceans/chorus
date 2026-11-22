use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::LazyLock;

use atomicow::CowArc;
use rustc_hash::FxHashMap;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

const REGISTRY_FILE: &str = "biomes.json";

#[derive(Clone, Eq)]
pub struct Biome(CowArc<'static, str>);

include!(concat!(env!("OUT_DIR"), "/biome_constants.rs"));

#[derive(Deserialize)]
struct Document {
    generated: Vec<GeneratedRow>,
    other: Vec<OtherRow>,
}

#[derive(Deserialize)]
struct GeneratedRow {
    name: String,
    pack: Option<String>,
    id: i32,
}

#[derive(Deserialize)]
struct OtherRow {
    pack: String,
    id: i32,
}

#[derive(Debug)]
pub struct BiomeEntry {
    pub name: String,
    pub pack: String,
    pub id: i32,
}

#[derive(Debug)]
pub struct BiomeRegistry {
    generated: Vec<BiomeEntry>,
    positions: FxHashMap<String, usize>,
    id_positions: FxHashMap<i32, usize>,
    ids_by_pack: FxHashMap<String, i32>,
    packs_by_id: FxHashMap<i32, String>,
}

impl BiomeRegistry {
    pub fn parse(json: &str) -> Result<Self, String> {
        let document: Document = serde_json::from_str(json).map_err(|error| error.to_string())?;
        let generated: Vec<BiomeEntry> = document
            .generated
            .into_iter()
            .map(|row| BiomeEntry {
                pack: row.pack.unwrap_or_else(|| row.name.clone()),
                name: row.name,
                id: row.id,
            })
            .collect();
        let mut ids_by_pack = FxHashMap::default();
        let mut packs_by_id = FxHashMap::default();
        let rows = generated
            .iter()
            .map(|entry| (entry.pack.as_str(), entry.id))
            .chain(document.other.iter().map(|row| (row.pack.as_str(), row.id)));
        for (pack, id) in rows {
            if ids_by_pack.insert(pack.to_owned(), id).is_some() || packs_by_id.insert(id, pack.to_owned()).is_some() {
                return Err(format!("biome {pack} with id {id} repeats a pack name or an id"));
            }
        }
        let mut positions = FxHashMap::default();
        let mut id_positions = FxHashMap::default();
        for (position, entry) in generated.iter().enumerate() {
            if positions.insert(entry.name.clone(), position).is_some() {
                return Err(format!("the biome name {} repeats", entry.name));
            }
            id_positions.insert(entry.id, position);
        }
        Ok(Self {
            generated,
            positions,
            id_positions,
            ids_by_pack,
            packs_by_id,
        })
    }

    pub fn vanilla() -> &'static Self {
        static VANILLA: LazyLock<BiomeRegistry> = LazyLock::new(|| {
            let registry = BiomeRegistry::parse(&crate::shared::resources::text(REGISTRY_FILE)).unwrap_or_else(|error| panic!("{REGISTRY_FILE}: {error}"));
            assert!(
                registry.generated.len() == Biome::ALL.len() && registry.generated.iter().zip(&BIOME_KEYS).all(|(entry, key)| entry.name == key.name()),
                "{REGISTRY_FILE} does not match the compiled biome constants"
            );
            registry
        });
        &VANILLA
    }

    pub fn pack_id(&self, pack: &str) -> Option<i32> {
        self.ids_by_pack.get(pack).copied()
    }

    pub fn pack_name(&self, id: i32) -> Option<&str> {
        self.packs_by_id.get(&id).map(String::as_str)
    }

    pub fn position_of_id(&self, id: i32) -> Option<usize> {
        self.id_positions.get(&id).copied()
    }

    pub fn entry(&self, name: &str) -> Option<&BiomeEntry> {
        self.positions.get(name).map(|&position| &self.generated[position])
    }
}

impl Biome {
    pub const fn new(name: &'static str) -> Self {
        Self(CowArc::Static(name))
    }

    pub fn name(&self) -> &str {
        &self.0
    }

    pub fn pack_name(&self) -> &'static str {
        &self.entry().pack
    }

    pub fn bedrock_id(&self) -> i32 {
        self.entry().id
    }

    pub fn from_id(id: i32) -> Option<Biome> {
        BiomeRegistry::vanilla().position_of_id(id).map(|position| BIOME_KEYS[position].clone())
    }

    pub fn generated(id: i32) -> Biome {
        Self::from_id(id).unwrap_or_else(|| panic!("the biome id {id} is not a generated biome"))
    }

    pub fn from_name(name: &str) -> Option<Biome> {
        let bare = name.strip_prefix("minecraft:").unwrap_or(name);
        Self::ALL
            .into_iter()
            .find(|biome| biome.name() == bare)
            .or_else(|| BiomeRegistry::vanilla().pack_id(bare).and_then(Self::from_id))
    }

    fn entry(&self) -> &'static BiomeEntry {
        BiomeRegistry::vanilla()
            .entry(self.name())
            .unwrap_or_else(|| panic!("the biome {} is not in the registry", self.name()))
    }
}

impl PartialEq for Biome {
    fn eq(&self, other: &Self) -> bool {
        let (left, right) = (self.name(), other.name());
        (std::ptr::eq(left.as_ptr(), right.as_ptr()) && left.len() == right.len()) || left == right
    }
}

impl Hash for Biome {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name().hash(state);
    }
}

impl fmt::Debug for Biome {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for word in self.name().split('_') {
            let mut letters = word.chars();
            if let Some(first) = letters.next() {
                write!(formatter, "{}{}", first.to_ascii_uppercase(), letters.as_str())?;
            }
        }
        Ok(())
    }
}

impl Serialize for Biome {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.name())
    }
}

impl<'de> Deserialize<'de> for Biome {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let name = String::deserialize(deserializer)?;
        Self::from_name(&name).ok_or_else(|| D::Error::custom(format!("unknown biome \"{name}\"")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_constants_follow_the_registry_rows() {
        let registry = BiomeRegistry::vanilla();
        for biome in Biome::ALL {
            assert!(registry.entry(biome.name()).is_some(), "{}", biome.name());
        }
        assert_eq!(Biome::OCEAN.name(), "ocean");
        assert_eq!(Biome::PLAINS.name(), "plains");
        assert_eq!(Biome::DEEP_DARK.name(), "deep_dark");
        assert_eq!(Biome::ALL.len(), registry.generated.len());
        assert!(Biome::ALL.iter().zip(&registry.generated).all(|(biome, entry)| biome.name() == entry.name));
    }

    #[test]
    fn every_biome_has_a_pack_document_and_ids_are_unique() {
        let registry = BiomeRegistry::vanilla();
        let mut ids: Vec<i32> = Biome::ALL.iter().map(Biome::bedrock_id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), Biome::ALL.len());
        for biome in Biome::ALL {
            assert_eq!(registry.pack_id(biome.pack_name()), Some(biome.bedrock_id()), "{}", biome.name());
            assert_eq!(registry.pack_name(biome.bedrock_id()), Some(biome.pack_name()));
        }
        assert_eq!(registry.pack_id("the_end"), Some(9));
        assert_eq!(registry.pack_id("hell"), Some(8));
        assert_eq!(registry.pack_id("not_a_biome"), None);
    }

    #[test]
    fn names_resolve_with_or_without_the_namespace_and_debug_is_camel_case() {
        assert_eq!(Biome::from_name("minecraft:windswept_hills"), Some(Biome::WINDSWEPT_HILLS));
        assert_eq!(Biome::from_name("deep_dark"), Some(Biome::DEEP_DARK));
        assert_eq!(Biome::from_name("deep dark"), None);
        assert_eq!(Biome::from_name("swampland"), Some(Biome::SWAMP));
        assert_eq!(Biome::from_name("minecraft:mesa"), Some(Biome::BADLANDS));
        assert_eq!(format!("{:?}", Biome::OLD_GROWTH_PINE_TAIGA), "OldGrowthPineTaiga");
    }

    #[test]
    fn the_generated_pack_ids_match_the_registry() {
        let registry = BiomeRegistry::vanilla();
        assert_eq!(registry.pack_id("the_end"), Some(pack_ids::THE_END));
        assert_eq!(registry.pack_id("hell"), Some(pack_ids::HELL));
        assert_eq!(registry.pack_id("crimson_forest"), Some(pack_ids::CRIMSON_FOREST));
        assert_eq!(registry.pack_id("plains"), Some(pack_ids::PLAINS));
        assert_eq!(pack_ids::BASALT_DELTAS, 181);
    }

    #[test]
    fn ids_resolve_back_to_their_keys() {
        for biome in Biome::ALL {
            assert_eq!(Biome::from_id(biome.bedrock_id()), Some(biome));
        }
        assert_eq!(Biome::from_id(-12345), None);
        assert_eq!(Biome::from_id(9), None);
    }

    #[test]
    fn keys_compare_by_name_whatever_their_storage() {
        let owned = Biome(CowArc::Owned(std::sync::Arc::from("plains")));
        assert_eq!(owned, Biome::PLAINS);
        assert_ne!(owned, Biome::FOREST);
        assert_eq!(owned.bedrock_id(), Biome::PLAINS.bedrock_id());
        let mut set = std::collections::HashSet::new();
        set.insert(owned);
        assert!(set.contains(&Biome::PLAINS));
    }

    #[test]
    fn a_registry_with_a_repeated_id_is_refused() {
        assert!(BiomeRegistry::parse(r#"{"generated": [{"name": "a", "id": 1}, {"name": "b", "id": 1}], "other": []}"#).is_err());
        assert!(BiomeRegistry::parse(r#"{"generated": [{"name": "a", "id": 1}, {"name": "a", "pack": "x", "id": 2}], "other": []}"#).is_err());
        assert!(BiomeRegistry::parse(r#"{"generated": [{"name": "a", "id": 1}], "other": [{"pack": "a", "id": 2}]}"#).is_err());
        assert!(BiomeRegistry::parse(r#"{"generated": [{"name": "a", "id": 1}], "other": [{"pack": "b", "id": 2}]}"#).is_ok());
    }
}
