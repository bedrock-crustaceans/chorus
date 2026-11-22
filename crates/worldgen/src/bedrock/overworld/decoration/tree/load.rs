use super::Tree;
use super::beehive::{self, OptionalBeehive};
use super::foliage::ForestFoliage;
use super::legacy::SwampFoliage;
use super::legacy::{LegacyTrees, Picker};
use super::litter::{self, FlowerBed};
use super::mushroom::{self, HugeMushroom};
use super::rock::{self, ForestRock};
use super::shelf::Shelf;
use super::{Search, Sequence};
use crate::bedrock::overworld::decoration::placed::Placed;
use crate::bedrock::overworld::decoration::registry::{FeatureRef, FeatureRegistry};
use crate::bedrock::overworld::decoration::rules::{Blocker, FeatureLoader, arc};
use std::sync::Arc;

const TREE_FEATURE: &str = "minecraft:tree_feature";
const SWAMP_FOLIAGE: &str = "minecraft:legacy:swamp_foliage_feature";
const FOREST_FOLIAGE: &str = "minecraft:legacy:forest_foliage_feature";
const FLOWER_FOREST_FOLIAGE: &str = "minecraft:legacy:flower_forest_foliage_feature";
const SEARCH_FEATURE: &str = "minecraft:search_feature";
const SEQUENCE_FEATURE: &str = "minecraft:sequence_feature";
const HORIZONTAL_FEATURE: &str = "minecraft:horizontal_tree_decoration_feature";
const MISSING_FEATURE: &str = "missing feature";
const MALFORMED_SEARCH: &str = "malformed search feature";

struct TreeSet {
    id: &'static str,
    count: f32,
    picker: Picker,
    features: &'static [&'static str],
}

const TREE_SETS: [TreeSet; 17] = [
    TreeSet {
        id: "minecraft:legacy:bamboo_jungle_tree_feature",
        count: 30.0,
        picker: Picker::BambooJungle,
        features: &[
            "minecraft:fancy_oak_tree_feature",
            "minecraft:jungle_bush_feature",
            "minecraft:mega_jungle_tree_feature",
            "minecraft:scatter_tall_grass_around_tree_feature",
        ],
    },
    TreeSet {
        id: "minecraft:legacy:birch_forest_tree_feature",
        count: 10.0,
        picker: Picker::First,
        features: &["minecraft:select_birch_tree_feature", "minecraft:birch_tree_with_beehive_feature"],
    },
    TreeSet {
        id: "minecraft:legacy:birch_forest_mutated_tree_feature",
        count: 10.0,
        picker: Picker::BirchMutated,
        features: &["minecraft:select_birch_tree_feature", "minecraft:select_super_birch_tree_feature"],
    },
    TreeSet {
        id: "minecraft:legacy:extreme_hills_plus_trees_tree_feature",
        count: 3.0,
        picker: Picker::ExtremeHills,
        features: &["minecraft:select_spruce_tree_feature", "minecraft:fancy_oak_tree_feature", "minecraft:select_oak_tree_feature"],
    },
    TreeSet {
        id: "minecraft:legacy:flower_forest_tree_feature",
        count: 6.0,
        picker: Picker::Forest,
        features: &[
            "minecraft:select_birch_tree_feature",
            "minecraft:fancy_oak_tree_with_optional_beehive_feature",
            "minecraft:oak_tree_with_optional_beehive_feature",
        ],
    },
    TreeSet {
        id: "minecraft:legacy:meadow_trees_feature",
        count: 0.01,
        picker: Picker::Meadow,
        features: &["minecraft:fancy_oak_tree_with_beehive_feature", "minecraft:super_birch_tree_with_beehive_feature"],
    },
    TreeSet {
        id: "minecraft:legacy:forest_tree_feature",
        count: 10.0,
        picker: Picker::Forest,
        features: &[
            "minecraft:select_birch_tree_with_leaf_litter_feature",
            "minecraft:fancy_oak_tree_with_leaf_litter_and_optional_beehive",
            "minecraft:select_oak_tree_with_leaf_litter_feature",
        ],
    },
    TreeSet {
        id: "minecraft:legacy:ice_tree_feature",
        count: 0.1,
        picker: Picker::First,
        features: &["minecraft:select_spruce_tree_feature"],
    },
    TreeSet {
        id: "minecraft:legacy:jungle_tree_feature",
        count: 50.0,
        picker: Picker::Jungle,
        features: &[
            "minecraft:fancy_oak_tree_feature",
            "minecraft:jungle_bush_feature",
            "minecraft:mega_jungle_tree_feature",
            "minecraft:select_jungle_tree_feature",
        ],
    },
    TreeSet {
        id: "minecraft:legacy:jungle_edge_tree_feature",
        count: 2.0,
        picker: Picker::JungleEdge,
        features: &[
            "minecraft:fancy_oak_tree_with_optional_beehive_feature",
            "minecraft:jungle_bush_feature",
            "minecraft:select_jungle_tree_feature",
        ],
    },
    TreeSet {
        id: "minecraft:legacy:mesa_plateau_stone_tree_feature",
        count: 5.0,
        picker: Picker::First,
        features: &["minecraft:select_oak_tree_with_leaf_litter_feature"],
    },
    TreeSet {
        id: "minecraft:legacy:plains_tree_feature",
        count: 0.05,
        picker: Picker::OneInThree,
        features: &[
            "minecraft:fancy_oak_tree_with_optional_beehive_feature",
            "minecraft:select_oak_tree_feature",
            "minecraft:oak_tree_with_beehive_feature",
        ],
    },
    TreeSet {
        id: "minecraft:legacy:savanna_tree_feature",
        count: 1.0,
        picker: Picker::FourInFive,
        features: &["minecraft:savanna_tree_feature", "minecraft:select_oak_tree_feature"],
    },
    TreeSet {
        id: "minecraft:legacy:savanna_mutated_tree_feature",
        count: 2.0,
        picker: Picker::FourInFive,
        features: &["minecraft:savanna_tree_feature", "minecraft:select_oak_tree_feature"],
    },
    TreeSet {
        id: "minecraft:legacy:taiga_tree_feature",
        count: 10.0,
        picker: Picker::OneInThree,
        features: &["minecraft:pine_tree_feature", "minecraft:select_spruce_tree_feature"],
    },
    TreeSet {
        id: "minecraft:legacy:mega_taiga_tree_feature",
        count: 10.0,
        picker: Picker::MegaTaiga,
        features: &[
            "minecraft:mega_spruce_tree_feature",
            "minecraft:mega_pine_tree_feature",
            "minecraft:pine_tree_feature",
            "minecraft:select_spruce_tree_feature",
        ],
    },
    TreeSet {
        id: "minecraft:legacy:redwood_taiga_mutated_tree_feature",
        count: 10.0,
        picker: Picker::RedwoodMutated,
        features: &["minecraft:mega_spruce_tree_feature", "minecraft:pine_tree_feature", "minecraft:select_spruce_tree_feature"],
    },
];

pub fn register(registry: &mut FeatureRegistry) {
    for &(id, _) in &litter::FEATURES {
        registry.register_override(id, flower_bed);
    }
    registry.register_override(rock::FEATURE, |loader, _| Ok(arc(ForestRock::new(loader.table()))));
    registry.register_override(mushroom::FEATURE, |loader, _| Ok(arc(HugeMushroom::new(loader.table()))));
    registry.register_override(FOREST_FOLIAGE, |loader, _| Ok(arc(ForestFoliage::new(loader.table(), false))));
    registry.register_override(FLOWER_FOREST_FOLIAGE, |loader, _| Ok(arc(ForestFoliage::new(loader.table(), true))));
    registry.register_override(SWAMP_FOLIAGE, swamp_foliage);
    for set in &TREE_SETS {
        registry.register_override(set.id, tree_set);
    }
    registry.register_document_id(beehive::FEATURE, |loader, doc| {
        let search = loader.feature(doc.body["places_feature"].as_str().unwrap_or_default(), doc.depth + 1)?;
        Ok(arc(OptionalBeehive { search }))
    });
    registry.register(TREE_FEATURE, |loader, doc| {
        Tree::parse(loader, doc.depth, doc.body).map(arc).map_err(|reason| Blocker { kind: reason, id: doc.id.clone() })
    });
    registry.register(SEARCH_FEATURE, |loader, doc| {
        let feature = loader.feature(doc.body["places_feature"].as_str().unwrap_or_default(), doc.depth + 1)?;
        Search::parse(doc.body, feature).map(arc).ok_or(Blocker {
            kind: MALFORMED_SEARCH,
            id: doc.id.clone(),
        })
    });
    registry.register(SEQUENCE_FEATURE, |loader, doc| {
        let features = doc.body["features"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|item| loader.feature(item.as_str().unwrap_or_default(), doc.depth + 1))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(arc(Sequence { features }))
    });
    registry.register(HORIZONTAL_FEATURE, |loader, doc| {
        Shelf::parse(loader.table(), doc.body).map(arc).map_err(|kind| Blocker { kind, id: doc.id.clone() })
    });
}

fn flower_bed(loader: &mut FeatureLoader, reference: FeatureRef) -> Result<Arc<Placed>, Blocker> {
    let &(_, name) = litter::FEATURES.iter().find(|(feature, _)| *feature == reference.id.as_ref()).ok_or(Blocker {
        kind: MISSING_FEATURE,
        id: reference.id,
    })?;
    Ok(arc(FlowerBed::new(loader.table(), name)))
}

fn swamp_foliage(loader: &mut FeatureLoader, reference: FeatureRef) -> Result<Arc<Placed>, Blocker> {
    let mut load = |name: &'static str| loader.feature(name, reference.depth + 1);
    let parts = (load("minecraft:seagrass_feature"), load("minecraft:swamp_tree_feature"), load(mushroom::FEATURE));
    match parts {
        (Ok(seagrass), Ok(tree), Ok(mushroom)) => Ok(arc(SwampFoliage { seagrass, tree, mushroom })),
        (Err(blocker), ..) | (_, Err(blocker), _) | (.., Err(blocker)) => Err(blocker),
    }
}

fn tree_set(loader: &mut FeatureLoader, reference: FeatureRef) -> Result<Arc<Placed>, Blocker> {
    let set = TREE_SETS.iter().find(|set| set.id == reference.id.as_ref()).ok_or(Blocker {
        kind: MISSING_FEATURE,
        id: reference.id,
    })?;
    let features = set.features.iter().map(|name| loader.feature(name, reference.depth + 1)).collect::<Result<Vec<_>, _>>()?;
    Ok(arc(LegacyTrees {
        count: set.count,
        picker: set.picker,
        features,
    }))
}
