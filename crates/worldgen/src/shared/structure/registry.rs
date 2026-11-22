use chorus_level::dimension_definition::DimensionDefinition;
use std::collections::BTreeMap;
use std::error::Error;

use rustc_hash::FxHashMap;
use serde_json::Value;

use crate::shared::structure::data::structure_set::{Placement, PlacementRule as SetRule, Spread as SetSpread, StructureSet};

use super::builder::{StructureRegistration, built_in_registrations};
use super::data::{PackWorldgen, pack_biome_id};
use super::placement::{Grid, Spread};
use super::{BoundingBox, StructureKind};
use crate::shared::CHUNK_WIDTH;
use chorus_structure_data::{PackSet, list, read_text};

const SETS_DIRECTORY: &str = "structure_sets";
const NAMESPACE: &str = "minecraft:";
const STRUCTURE_TAG_PREFIX: &str = "has_structure_";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementRule {
    Bedrock,
    Java,
}

#[derive(Clone)]
pub struct BiomeFilter {
    pub tag: String,
    pub radius: i32,
}

#[derive(Clone)]
pub struct AdaptationRule {
    pub mode: String,
    pub order: i32,
}

#[derive(Clone)]
pub struct StructureDef {
    pub identifier: String,
    pub kind: StructureKind,
    pub grid: Option<Grid>,
    pub rule: PlacementRule,
    pub reach: i32,
    pub filter: Option<BiomeFilter>,
    pub adaptation: Option<AdaptationRule>,
}

#[derive(Clone)]
pub struct StructureRegistry {
    defs: Vec<StructureDef>,
    pub(super) tags: FxHashMap<String, Vec<i32>>,
    builders: Vec<(StructureKind, StructureRegistration)>,
}

fn field<'a>(value: &'a Value, path: &[&str]) -> Result<&'a Value, Box<dyn Error>> {
    path.iter()
        .try_fold(value, |current, key| current.get(*key).ok_or_else(|| format!("missing {}", path.join(".")).into()))
}

fn integer(value: &Value, path: &[&str]) -> Result<i32, Box<dyn Error>> {
    Ok(field(value, path)?.as_i64().ok_or_else(|| format!("{} is not an integer", path.join(".")))? as i32)
}

fn text<'a>(value: &'a Value, path: &[&str]) -> Result<&'a str, Box<dyn Error>> {
    field(value, path)?.as_str().ok_or_else(|| format!("{} is not text", path.join(".")).into())
}

impl StructureRegistry {
    pub fn load_default() -> Result<Self, Box<dyn Error>> {
        Self::load(&super::data::embedded_packs())
    }

    pub fn load(packs: &PackSet) -> Result<Self, Box<dyn Error>> {
        Self::load_with(packs, Vec::new())
    }

    pub fn load_with(packs: &PackSet, registrations: Vec<StructureRegistration>) -> Result<Self, Box<dyn Error>> {
        let mut registry = Self {
            defs: Vec::new(),
            tags: FxHashMap::default(),
            builders: Vec::new(),
        };
        for registration in built_in_registrations().into_iter().chain(registrations) {
            registry.register_builder(registration)?;
        }
        let worldgen = match PackWorldgen::load(packs) {
            Ok(worldgen) => Some(worldgen),
            Err(error) => {
                tracing::debug!("no worldgen documents in the bundle: {error}");
                None
            }
        };
        let mut set_tags: BTreeMap<String, Vec<i32>> = BTreeMap::new();
        let mut set_files: BTreeMap<String, String> = BTreeMap::new();
        for name in list(packs, SETS_DIRECTORY, ".json", false) {
            let set = StructureSet::parse(&read_text(packs, &name)?).map_err(|error| format!("{name}: {error}"))?;
            let def = registry.definition(&set, worldgen.as_ref()).map_err(|error| format!("{name}: {error}"))?;
            if let Some(first) = set_files.get(&def.identifier) {
                let owner = |file: &str| packs.owner(file).unwrap_or_default().to_owned();
                return Err(format!(
                    "{name}: structure set {} is defined more than once, by pack {} ({first}) and pack {} ({name})",
                    def.identifier,
                    owner(first),
                    owner(&name)
                )
                .into());
            }
            set_files.insert(def.identifier.clone(), name.clone());
            if registry.defs.iter().any(|known| known.kind == def.kind) {
                return Err(format!("{name}: structure kind {} is defined by more than one structure set", set.kind).into());
            }
            registry.defs.push(def);
            for (tag, biomes) in &set.biome_tags {
                let ids = Self::resolve_biomes(tag, biomes).map_err(|error| format!("{name}: {error}"))?;
                if set_tags.insert(tag.clone(), ids).is_some() {
                    return Err(format!("{name}: biome tag {tag} is defined by more than one structure set").into());
                }
            }
        }
        let mut tags = FxHashMap::default();
        if let Some(worldgen) = &worldgen {
            for tag in worldgen.biomes.tags() {
                if let Some(name) = tag.strip_prefix(STRUCTURE_TAG_PREFIX) {
                    tags.insert(name.to_owned(), worldgen.biomes.ids_with(tag));
                }
            }
        }
        tags.extend(set_tags);
        registry.tags = tags;
        for (kind, registration) in &registry.builders {
            if !registry.defs.iter().any(|def| def.kind == *kind) {
                return Err(format!("structure builder {} has no structure set", registration.identifier).into());
            }
        }
        Ok(registry)
    }

    pub fn register_builder(&mut self, registration: StructureRegistration) -> Result<StructureKind, Box<dyn Error>> {
        if self.builders.iter().any(|(_, known)| known.identifier == registration.identifier) {
            return Err(format!("structure builder {} is registered more than once", registration.identifier).into());
        }
        let kind = StructureKind::from_variant(&registration.identifier).unwrap_or_else(|| {
            let next = self.builders.iter().map(|(kind, _)| kind.0 + 1).max().unwrap_or(0);
            StructureKind(next.max(StructureKind::BUILT_IN_COUNT))
        });
        self.builders.push((kind, registration));
        Ok(kind)
    }

    pub fn registrations(&self) -> impl Iterator<Item = (StructureKind, &StructureRegistration)> {
        self.builders.iter().map(|(kind, registration)| (*kind, registration))
    }

    pub fn registered_kind(&self, identifier: &str) -> Option<StructureKind> {
        self.builders.iter().find(|(_, registration)| registration.identifier == identifier).map(|(kind, _)| *kind)
    }

    fn resolve_biomes(tag: &str, biomes: &[String]) -> Result<Vec<i32>, Box<dyn Error>> {
        biomes
            .iter()
            .map(|biome| pack_biome_id(biome).ok_or_else(|| format!("biome tag {tag} names unknown biome {biome}").into()))
            .collect()
    }

    pub(super) fn pack_placement<'a>(identifier: &str, packs: Option<&'a PackWorldgen>) -> Option<&'a Value> {
        packs?.structure_sets.iter().find(|set| set["description"]["identifier"] == identifier).map(|set| &set["placement"])
    }

    pub(super) fn grid_of(placement: &Value) -> Result<Option<Grid>, Box<dyn Error>> {
        if text(placement, &["type"])? != "minecraft:random_spread" {
            return Ok(None);
        }
        let spread = match text(placement, &["spread_type"])? {
            "linear" => Spread::Linear,
            "triangular" => Spread::Triangular,
            other => return Err(format!("unknown spread type {other}").into()),
        };
        Ok(Some(Grid::new(
            integer(placement, &["spacing"])?,
            integer(placement, &["separation"])?,
            integer(placement, &["salt"])?,
            spread,
        )))
    }

    fn definition(&self, set: &StructureSet, packs: Option<&PackWorldgen>) -> Result<StructureDef, Box<dyn Error>> {
        let identifier = if set.identifier.contains(':') {
            set.identifier.clone()
        } else {
            format!("{NAMESPACE}{}", set.identifier)
        };
        let kind = self
            .registered_kind(&set.kind)
            .ok_or_else(|| format!("structure set {} names kind {}, which no registered builder provides", set.identifier, set.kind))?;
        let rule = match set.rule {
            SetRule::Bedrock => PlacementRule::Bedrock,
            SetRule::Java => PlacementRule::Java,
        };
        let grid = match &set.placement {
            Placement::RandomSpread { spacing, separation, salt, spread } => {
                let spread = match spread {
                    SetSpread::Linear => Spread::Linear,
                    SetSpread::Triangular => Spread::Triangular,
                };
                Some(Grid::new(*spacing, *separation, *salt, spread))
            }
            Placement::ConcentricRings | Placement::PerChunk => None,
            Placement::Pack => match Self::pack_placement(&identifier, packs) {
                Some(placement) => Self::grid_of(placement)?,
                None if packs.is_some() => return Err(format!("no placement for {identifier}").into()),
                None => None,
            },
        };
        Ok(StructureDef {
            identifier,
            kind,
            grid,
            rule,
            reach: set.reach,
            filter: set.biome_filter.as_ref().map(|filter| BiomeFilter {
                tag: filter.tag.clone(),
                radius: filter.radius,
            }),
            adaptation: set.adaptation.as_ref().map(|rule| AdaptationRule {
                mode: rule.mode.clone(),
                order: rule.order,
            }),
        })
    }

    pub fn definitions(&self) -> &[StructureDef] {
        &self.defs
    }

    fn def(&self, kind: StructureKind) -> &StructureDef {
        self.defs.iter().find(|def| def.kind == kind).expect("every structure kind is defined, checked when the registry loads")
    }

    pub fn grid(&self, kind: StructureKind) -> Option<Grid> {
        let def = self.def(kind);
        def.grid.filter(|_| def.rule == PlacementRule::Bedrock)
    }

    pub fn java_grid(&self, kind: StructureKind) -> Option<Grid> {
        let def = self.def(kind);
        def.grid.filter(|_| def.rule == PlacementRule::Java)
    }

    pub fn any_grid(&self, kind: StructureKind) -> Option<Grid> {
        self.def(kind).grid
    }

    pub fn adaptation(&self, kind: StructureKind) -> Option<&AdaptationRule> {
        self.def(kind).adaptation.as_ref()
    }

    pub fn filter(&self, kind: StructureKind) -> Option<&BiomeFilter> {
        self.def(kind).filter.as_ref()
    }

    pub fn reach(&self, kind: StructureKind) -> i32 {
        self.def(kind).reach
    }

    pub fn identifier(&self, kind: StructureKind) -> &str {
        &self.def(kind).identifier
    }

    pub fn name(&self, kind: StructureKind) -> &str {
        let identifier = &self.def(kind).identifier;
        identifier.strip_prefix(NAMESPACE).unwrap_or(identifier)
    }

    pub fn kind_named(&self, name: &str) -> Option<StructureKind> {
        self.defs
            .iter()
            .find(|def| def.identifier.strip_prefix(NAMESPACE).unwrap_or(&def.identifier) == name)
            .map(|def| def.kind)
    }

    pub fn biomes(&self, tag: &str) -> &[i32] {
        self.tags.get(tag).map_or(&[], |ids| ids.as_slice())
    }

    pub fn rough_area(&self, kind: StructureKind, (x, z): (i32, i32), definition: DimensionDefinition) -> BoundingBox {
        let reach = self.reach(kind);
        BoundingBox::new(
            (x - reach) * CHUNK_WIDTH,
            definition.min_y,
            (z - reach) * CHUNK_WIDTH,
            (x + reach) * CHUNK_WIDTH + CHUNK_WIDTH - 1,
            definition.max_y(),
            (z + reach) * CHUNK_WIDTH + CHUNK_WIDTH - 1,
        )
    }
}

#[cfg(test)]
mod tag_tests {
    use super::*;

    const SET_TAGS: [&str; 19] = [
        "beaches",
        "buried_treasure",
        "deep_dark",
        "deep_oceans",
        "desert_pyramid",
        "jungle_temple",
        "mansion",
        "mesa",
        "monument_surroundings",
        "ocean_ruins",
        "outpost",
        "shipwreck",
        "swamp_hut",
        "temple",
        "village",
        "village_desert",
        "village_savanna",
        "village_snowy",
        "village_taiga",
    ];

    #[test]
    fn every_structure_set_tag_resolves_to_biomes() {
        let registry = StructureRegistry::load_default().expect("loads");
        for tag in SET_TAGS {
            assert!(!registry.biomes(tag).is_empty(), "{tag} is empty");
        }
    }

    #[test]
    fn unknown_biome_names_fail_with_the_tag_and_name() {
        let error = StructureRegistry::resolve_biomes("temple", &["desert".to_owned(), "nowhere".to_owned()])
            .expect_err("fails")
            .to_string();
        assert!(error.contains("temple") && error.contains("nowhere"));
        assert_eq!(StructureRegistry::resolve_biomes("temple", &["desert".to_owned(), "jungle".to_owned()]).expect("resolves"), [2, 21]);
    }
}
