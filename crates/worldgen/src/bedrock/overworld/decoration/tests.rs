use super::expression::{Expression, Host, Variables};
use super::ore::Ore;
use super::registry::FeatureRegistry;
use super::rules::{self, Feature, Loaded, load_with_registry};
use crate::shared::biome::Biome;
use crate::shared::blocks::{AIR, BlockTable};
use crate::shared::rng::mersenne::MersenneTwister;
use chorus_block::block_registry::BlockRegistry;
use std::sync::Arc;
use std::sync::LazyLock;

pub(super) struct NoHost;

impl Host for NoHost {
    fn heightmap(&mut self, _x: i32, _z: i32) -> i32 {
        0
    }

    fn above_top_solid(&mut self, _x: i32, _z: i32) -> i32 {
        0
    }
}

pub(super) fn load(table: &mut BlockTable) -> Loaded {
    load_with(table, &[])
}

pub(super) fn load_with(table: &mut BlockTable, extra_features: &[Arc<str>]) -> Loaded {
    load_with_registry(table, extra_features, &FeatureRegistry::vanilla())
}

fn evaluate(text: &str) -> f32 {
    let variables = Variables {
        origin: [32.0, 64.0, -16.0],
        world: [1.0, 2.0, 3.0],
    };
    Expression::parse(text).evaluate(&variables, &mut NoHost, &mut MersenneTwister::new(7))
}

#[test]
fn expressions_cover_what_the_rules_use() {
    assert_eq!(evaluate("17"), 17.0);
    assert_eq!(evaluate("variable.worldx + variable.worldz * 2"), 7.0);
    assert_eq!(evaluate("variable.is_legacy ? 0.0 : 100.0"), 100.0);
    assert_eq!(evaluate("variable.is_legacy == 0.0 ? 1 : 0"), 1.0);
    assert_eq!(evaluate("(variable.originx + 8) / 4"), 10.0);
    assert_eq!(evaluate("variable.originy"), 64.0);
    assert_eq!(evaluate("math.ceil(2.1) + math.max(1, 5) - math.min(4, 2)"), 6.0);
    assert_eq!(evaluate("(1 < 2) && variable.is_legacy == 0.0 ? 7 : 0"), 7.0);
    assert_eq!(evaluate("-variable.worldy"), -2.0);
}

pub(super) static REGISTRY: LazyLock<BlockRegistry> = LazyLock::new(|| {
    let mut registry = BlockRegistry::new();
    registry.register_all(chorus_block::r#impl::DEFINITIONS.iter().copied());
    registry
});

fn ore_of(feature: &Feature) -> Option<&Ore> {
    match feature {
        Feature::Placed(placed) => placed.as_any().downcast_ref::<Ore>(),
        Feature::Unsupported(_) => None,
    }
}

fn loaded() -> (Loaded, BlockTable<'static>) {
    let mut table = BlockTable::new(&REGISTRY);
    let loaded = load(&mut table);
    (loaded, table)
}

#[test]
fn loader_parses_every_rule_and_feature() {
    let (loaded, mut table) = loaded();
    assert!(loaded.report.unsupported_filters.is_empty(), "{:?}", loaded.report.unsupported_filters);
    let explicit = load_with_registry(&mut table, &[], &FeatureRegistry::vanilla());
    assert_eq!(explicit.report.feature_count, loaded.report.feature_count);
    assert_eq!(explicit.features.len(), loaded.features.len());
    assert_eq!(explicit.report.rule_count, loaded.report.rule_count);
    if crate::shared::structure::StructureData::shared().is_some() {
        assert!(loaded.features.contains_key("minecraft:fossil_feature"));
        assert!(loaded.features.contains_key("minecraft:deepslate_diamond_fossil_feature"));
    }
}

fn pass_order(loaded: &Loaded, pass: &str) -> Vec<&'static str> {
    let pass = rules::PASSES.iter().position(|name| *name == pass).expect("pass");
    loaded.rules.values().filter(|rule| rule.pass == pass).map(|rule| rule.identifier).collect()
}

#[test]
fn rules_are_sorted_by_identifier_within_each_pass() {
    let (loaded, _) = loaded();
    let passes: Vec<usize> = loaded.rules.values().map(|rule| rule.pass).collect();
    assert!(passes.is_sorted());
    for pass in rules::PASSES {
        let identifiers = pass_order(&loaded, pass);
        assert!(identifiers.windows(2).all(|pair| pair[0] <= pair[1]), "{} rules are not identifier-sorted", pass);
    }
}

#[test]
fn every_ore_block_exists_in_the_registry() {
    let (loaded, table) = loaded();
    let blocks = table.finish();
    let air_runtime = blocks.entry(AIR).runtime;
    for rule in loaded.rules.values() {
        if let Some(ore) = ore_of(&rule.feature) {
            for block in ore.place_blocks() {
                assert_ne!(blocks.entry(block).runtime, air_runtime, "{} places a block the registry does not have", rule.identifier);
            }
        }
    }
}

#[test]
fn biome_filters_resolve_through_the_biome_tags() {
    let (loaded, _) = loaded();
    let mask = |name: &str| loaded.rules.values().find(|rule| rule.identifier == name).expect("rule").biomes.clone();
    let generic_copper = mask("minecraft:overworld_underground_copper_ore_feature");
    assert!(generic_copper.contains(&Biome::PLAINS));
    assert!(!generic_copper.contains(&Biome::DRIPSTONE_CAVES));
    let dripstone_copper = mask("minecraft:dripstone_caves_underground_copper_ore_feature");
    assert!(dripstone_copper.contains(&Biome::DRIPSTONE_CAVES));
    assert_eq!(dripstone_copper.count(), 1);
    assert!(mask("minecraft:overworld_underground_emerald_ore_feature").contains(&Biome::WINDSWEPT_HILLS));
    assert!(!mask("minecraft:overworld_underground_emerald_ore_feature").contains(&Biome::PLAINS));
    assert!(mask("minecraft:overworld_underground_coal_ore_lower_feature").count() > 40);
}

#[test]
fn every_structure_feature_element_loads() {
    let names = crate::shared::structure::StructureData::shared().expect("structure data").feature_element_names();
    let ids: Vec<Arc<str>> = names.iter().map(|name| Arc::from(format!("minecraft:{name}"))).collect();
    let mut table = BlockTable::new(&REGISTRY);
    let loaded = load_with(&mut table, &ids);
    let missing: Vec<&Arc<str>> = ids.iter().filter(|id| !loaded.features.contains_key(id.as_ref())).collect();
    assert!(missing.iter().all(|id| id.contains("poplar")), "{missing:?}");
}
