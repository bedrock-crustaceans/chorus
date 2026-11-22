use serde_json::Value;

use crate::bedrock::overworld::Filter;
use crate::bedrock::overworld::expression::Variables;
use crate::bedrock::overworld::feature_rules::{FEATURES, PASSES, RULES};
use crate::bedrock::overworld::ore::Ore;
use crate::bedrock::overworld::rule_seed;
use crate::bedrock::overworld::scatter::{Draws, Scatter};
use crate::shared::biome::BiomeRegistry;
use crate::shared::biome_documents;
use crate::shared::blocks::BlockTable;
use crate::shared::hash::fnv1_64;
use crate::shared::rng::mersenne::MersenneTwister;
use glam::IVec3;

use super::biome::BIOMES;
use super::decorate::View;
use super::feature::{self, Feature, Materials};
use crate::shared::CHUNK_WIDTH;

const BIOME_FILTER: &str = "minecraft:biome_filter";
const PLACEMENT_PASS: &str = "placement_pass";
const PLACED_FEATURE: &str = "places_feature";
const DISTRIBUTION: &str = "distribution";
const SURFACE_PASS_NAME: &str = "surface_pass";
const UNDERGROUND_PASS_NAME: &str = "underground_pass";
const AFTER_SURFACE_PASS_NAME: &str = "after_surface_pass";
const ALL_BIOMES: u8 = 0b1_1111;
const FEATURE_ID: &str = "minecraft:nether_fossil_feature";
const BIRCH_TREES: &str = "minecraft:legacy:birch_forest_tree_feature";
const NO_FEATURE: &str = "no feature";
const MAX_DEPTH: usize = 4;

fn tags(biome: i32) -> Vec<&'static str> {
    let pack = BiomeRegistry::vanilla().pack_name(biome).unwrap_or_else(|| panic!("the nether biome {biome} is not in the registry"));
    biome_documents::tags(pack).collect()
}

fn bit(biome: i32) -> Option<u8> {
    BIOMES.iter().position(|&known| known == biome).map(|index| 1 << index)
}

fn mask(filter: Option<&Value>) -> u8 {
    let Some(filter) = filter else { return ALL_BIOMES };
    let filter = Filter::parse(filter);
    BIOMES
        .iter()
        .enumerate()
        .filter(|&(_, &biome)| filter.matches(&tags(biome)))
        .fold(0, |mask, (index, _)| mask | 1 << index)
}

pub struct Rule {
    hash: u64,
    biomes: u8,
    scatter: Scatter,
    feature: Feature,
}

pub struct NetherRules {
    rules: Vec<Rule>,
}

fn feature_of(table: &mut BlockTable, name: &'static str, depth: usize) -> Feature {
    if depth > MAX_DEPTH || name == BIRCH_TREES {
        return Feature::Nothing;
    }
    match name {
        "minecraft:crimson_fungus_scatter_feature"
        | "minecraft:warped_fungus_scatter_feature"
        | "minecraft:crimson_roots_scatter_feature"
        | "minecraft:warped_roots_scatter_feature"
        | "minecraft:nether_sprouts_scatter_feature" => {
            let body: &'static Value = &FEATURES[name]["body"];
            let inner = body[PLACED_FEATURE].as_str().unwrap_or(NO_FEATURE);
            let Some(inner) = FEATURES.get(inner).map(|_| inner) else { return Feature::Nothing };
            Feature::Scatter {
                scatter: Scatter::parse(body),
                inner: Box::new(feature_of(table, inner, depth + 1)),
            }
        }
        "minecraft:crimson_roots_feature" | "minecraft:warped_roots_feature" | "minecraft:nether_sprouts_feature" | "minecraft:crimson_fungus_feature" | "minecraft:warped_fungus_feature" => {
            let body: &'static Value = &FEATURES[name]["body"];
            feature::plant(table, body).map_or(Feature::Nothing, Feature::Plant)
        }
        "minecraft:_huge_crimson_fungus_feature" => Feature::Fungus { crimson: true },
        "minecraft:_huge_warped_fungus_feature" => Feature::Fungus { crimson: false },
        FEATURE_ID => Feature::Fossil,
        "minecraft:_basalt_pillar_feature" => Feature::Pillar,
        "minecraft:basalt_columns_feature" => Feature::Columns,
        "minecraft:delta_feature" => Feature::Delta,
        "minecraft:_weeping_vines_cluster_feature" => Feature::WeepingVines,
        "minecraft:_twisting_vines_cluster_feature" => Feature::TwistingVines,
        "minecraft:nether_soul_sand_underground_feature" => {
            let body: &'static Value = &FEATURES[name]["body"];
            Ore::parse(table, body).map_or(Feature::Nothing, Feature::Ore)
        }
        _ => Feature::Nothing,
    }
}

impl NetherRules {
    pub fn new(table: &mut BlockTable) -> Self {
        let mut parsed: Vec<(&'static str, u8, usize)> = Vec::new();
        for (name, entry) in RULES.as_object().into_iter().flatten() {
            let name: &'static str = name.as_str();
            let conditions = &entry["body"]["conditions"];
            let Some(pass) = conditions[PLACEMENT_PASS].as_str() else { continue };
            if !matches!(pass, SURFACE_PASS_NAME | UNDERGROUND_PASS_NAME | AFTER_SURFACE_PASS_NAME) {
                continue;
            }
            let Some(pass) = PASSES.iter().position(|known| *known == pass) else { continue };
            let biomes = mask(conditions.get(BIOME_FILTER));
            parsed.push((name, biomes, pass));
        }
        let mut order: Vec<usize> = (0..parsed.len()).collect();
        order.sort_by_key(|&index| (parsed[index].2, parsed[index].0));
        let mut rules = Vec::with_capacity(parsed.len());
        for index in order {
            let (name, biomes, _) = parsed[index];
            let body: &'static Value = &RULES[name]["body"];
            let placed = body["description"][PLACED_FEATURE].as_str().unwrap_or(NO_FEATURE);
            rules.push(Rule {
                hash: fnv1_64(name),
                biomes,
                scatter: Scatter::parse(&body[DISTRIBUTION]),
                feature: feature_of(table, placed, 0),
            });
        }
        tracing::debug!("nether decoration runs {} feature rules", rules.len());
        Self { rules }
    }

    pub fn place(&self, decoration_seed: u32, chunk_random: &mut MersenneTwister, source: (i32, i32), biome: i32, view: &mut View, materials: Option<&Materials>) {
        let Some(wanted) = bit(biome) else { return };
        let corner = IVec3::new(source.0 * CHUNK_WIDTH, 0, source.1 * CHUNK_WIDTH);
        for rule in self.rules.iter().filter(|rule| rule.biomes & wanted != 0) {
            let seed = rule_seed(rule.hash, decoration_seed);
            let mut positions = MersenneTwister::new(seed);
            let mut placing: Option<MersenneTwister> = None;
            let mut draws = Draws {
                positions: &mut positions,
                molang: Some(chunk_random),
            };
            let variables = Variables {
                origin: [corner.x as f32, 0.0, corner.z as f32],
                world: [0.0; 3],
            };
            rule.scatter.run(&mut draws, variables, corner, view, |view, _, position| {
                let placing = placing.get_or_insert_with(|| MersenneTwister::new(seed));
                feature::place(&rule.feature, view, placing, position, materials);
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::biome::{BASALT_DELTAS, CRIMSON_FOREST, HELL, WARPED_FOREST};
    use super::*;

    fn has(tag: &str) -> Value {
        serde_json::json!({"test": "has_biome_tag", "operator": "==", "value": tag})
    }

    #[test]
    fn no_nether_rule_filter_holds_a_test_the_shared_filter_cannot_evaluate() {
        let mut checked = 0;
        for (name, entry) in RULES.as_object().expect("rules") {
            let conditions = &entry["body"]["conditions"];
            let Some(pass) = conditions[PLACEMENT_PASS].as_str() else { continue };
            if !matches!(pass, SURFACE_PASS_NAME | UNDERGROUND_PASS_NAME | AFTER_SURFACE_PASS_NAME) {
                continue;
            }
            let Some(filter) = conditions.get(BIOME_FILTER) else { continue };
            let mut unsupported = Vec::new();
            Filter::parse(filter).collect_unsupported(&mut unsupported);
            assert!(unsupported.is_empty() || mask(Some(filter)) == 0, "{name} uses {unsupported:?}");
            checked += 1;
        }
        assert!(checked > 100);
    }

    #[test]
    fn masks_follow_the_biome_document_tags() {
        let only = |biome: i32| bit(biome).expect("a nether biome");
        assert_eq!(mask(Some(&has("nether_wastes"))), only(HELL));
        assert_eq!(mask(Some(&has("netherwart_forest"))), only(CRIMSON_FOREST) | only(WARPED_FOREST));
        assert_eq!(mask(Some(&has("basalt_deltas"))), only(BASALT_DELTAS));
        assert_eq!(mask(Some(&has("nether"))), ALL_BIOMES);
        assert_eq!(mask(Some(&has("overworld"))), 0);
        assert_eq!(mask(None), ALL_BIOMES);
    }
}
