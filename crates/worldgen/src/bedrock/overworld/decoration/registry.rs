use super::cave;
use super::coral::{Coral, CoralCrust};
use super::disk::Disk;
use super::fossil::Fossil;
use super::geode::Geode;
use super::ice::{IcePatch, IceSpike};
use super::iceberg::IcebergFeature;
use super::ore::Ore;
use super::pale::Pale;
use super::pile::BlockPile;
use super::placed::{Aggregate, CoralHang, EarlyOut, Placed, ScatterFeature, SeaAnemone};
use super::plants::{Bamboo, Cactus, Flower, FlowerMode};
use super::pool::SulfurPool;
use super::rules::{self, Blocker, FeatureLoader, arc};
use super::scatter::Scatter;
use super::single_block::SingleBlock;
use super::springs::Springs;
use super::structure_template::StructureTemplate;
use super::tree;
use super::vegetation::Vegetation;
use super::well::DesertWell;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

const BLOCK_PILE_FEATURE: &str = "chorus:block_pile";
const DISK_FEATURE: &str = "chorus:disk";
const ORE_FEATURE: &str = "minecraft:ore_feature";
const SCATTER_FEATURE: &str = "minecraft:scatter_feature";
const AGGREGATE_FEATURE: &str = "minecraft:aggregate_feature";
const GEODE_FEATURE: &str = "minecraft:geode_feature";
const FOSSIL_FEATURE: &str = "minecraft:fossil_feature";
const STRUCTURE_TEMPLATE_FEATURE: &str = "minecraft:structure_template_feature";
const SINGLE_BLOCK_FEATURE: &str = "minecraft:single_block_feature";

#[derive(Clone, Debug)]
pub struct FeatureDocument {
    pub id: Arc<str>,
    pub kind: &'static str,
    pub body: &'static Value,
    pub depth: usize,
}

impl FeatureDocument {
    pub fn blocked(&self) -> Blocker {
        Blocker { kind: self.kind, id: self.id.clone() }
    }
}

#[derive(Clone, Debug)]
pub struct FeatureRef {
    pub id: Arc<str>,
    pub depth: usize,
}

pub type KindParser = fn(&mut FeatureLoader, &FeatureDocument) -> Result<Arc<Placed>, Blocker>;
pub type IdParser = fn(&mut FeatureLoader, FeatureRef) -> Result<Arc<Placed>, Blocker>;

#[derive(Default)]
pub struct FeatureRegistry {
    overrides: HashMap<&'static str, IdParser>,
    document_ids: HashMap<&'static str, KindParser>,
    kinds: HashMap<&'static str, KindParser>,
    natives: HashMap<&'static str, IdParser>,
    #[cfg(test)]
    extra_documents: Vec<&'static Value>,
}

impl FeatureRegistry {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn vanilla() -> Self {
        let mut registry = Self::empty();
        register_kinds(&mut registry);
        register_natives(&mut registry);
        cave::register(&mut registry);
        tree::register(&mut registry);
        Vegetation::register(&mut registry);
        registry
    }

    pub fn register(&mut self, kind: &'static str, parser: KindParser) -> &mut Self {
        self.kinds.insert(kind, parser);
        self
    }

    pub fn register_id(&mut self, id: &'static str, parser: IdParser) -> &mut Self {
        self.natives.insert(id, parser);
        self
    }

    pub fn register_override(&mut self, id: &'static str, parser: IdParser) -> &mut Self {
        self.overrides.insert(id, parser);
        self
    }

    pub fn register_document_id(&mut self, id: &'static str, parser: KindParser) -> &mut Self {
        self.document_ids.insert(id, parser);
        self
    }

    #[cfg(test)]
    pub fn add_documents(&mut self, json: &'static str) -> &mut Self {
        let parsed: &'static Value = Box::leak(Box::new(rules::documents(json)));
        self.extra_documents.push(parsed);
        self
    }

    #[cfg(test)]
    pub fn with_documents(mut self, json: &'static str) -> Self {
        self.add_documents(json);
        self
    }

    pub(super) fn override_for(&self, id: &str) -> Option<IdParser> {
        self.overrides.get(id).copied()
    }

    pub(super) fn document_id_parser(&self, id: &str) -> Option<KindParser> {
        self.document_ids.get(id).copied()
    }

    pub(super) fn kind_parser(&self, kind: &str) -> Option<KindParser> {
        self.kinds.get(kind).copied()
    }

    pub(super) fn native(&self, id: &str) -> Option<IdParser> {
        self.natives.get(id).copied()
    }

    pub(super) fn document(&self, id: &str) -> Option<&'static Value> {
        let features: &'static Value = &rules::FEATURES;
        let chorus: &'static Value = &rules::CHORUS_FEATURES;
        let found = features.get(id).or_else(|| chorus.get(id));
        #[cfg(test)]
        let found = found.or_else(|| self.extra_documents.iter().find_map(|documents| documents.get(id)));
        found
    }
}

fn register_kinds(registry: &mut FeatureRegistry) {
    registry.register(ORE_FEATURE, |loader, doc| Ore::parse(loader.table(), doc.body).map(arc).ok_or(doc.blocked()));
    registry.register(FOSSIL_FEATURE, |loader, doc| {
        let fossils = loader.fossils();
        Fossil::parse(loader.table(), doc.body, fossils).map(arc).ok_or(doc.blocked())
    });
    registry.register(STRUCTURE_TEMPLATE_FEATURE, |loader, doc| {
        StructureTemplate::parse(loader.table(), doc.body).map(arc).ok_or(doc.blocked())
    });
    registry.register(GEODE_FEATURE, |loader, doc| Geode::parse(loader.table(), doc.body).map(arc).ok_or(doc.blocked()));
    registry.register(DISK_FEATURE, |loader, doc| Disk::parse(loader.table(), doc.body).map(arc).ok_or(doc.blocked()));
    registry.register(BLOCK_PILE_FEATURE, |loader, doc| {
        BlockPile::parse(loader.table(), doc.body).map(|pile| arc(Vegetation::BlockPile(pile))).ok_or(doc.blocked())
    });
    registry.register(SINGLE_BLOCK_FEATURE, |loader, doc| {
        SingleBlock::parse(loader.table(), doc.body).map(|single| arc(Vegetation::SingleBlock(single))).ok_or(doc.blocked())
    });
    registry.register(SCATTER_FEATURE, |loader, doc| {
        let inner = loader.feature(doc.body["places_feature"].as_str().unwrap_or_default(), doc.depth + 1)?;
        let scatter = Scatter::parse(doc.body);
        if !scatter.is_supported() {
            return Err(doc.blocked());
        }
        Ok(arc(ScatterFeature {
            scatter,
            project_to_floor: doc.body["project_input_to_floor"].as_bool().unwrap_or(false),
            inner,
        }))
    });
    registry.register(AGGREGATE_FEATURE, |loader, doc| {
        let early_out = match doc.body["early_out"].as_str() {
            Some("first_failure") => EarlyOut::FirstFailure,
            Some("first_success") => EarlyOut::FirstSuccess,
            _ => EarlyOut::None,
        };
        let features = doc.body["features"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|item| loader.feature(item.as_str().unwrap_or_default(), doc.depth + 1))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(arc(Aggregate { early_out, features }))
    });
}

fn register_natives(registry: &mut FeatureRegistry) {
    registry.register_id("minecraft:legacy:springs_feature", |loader, _| Ok(arc(Springs::new(loader.table()))));
    registry.register_id("minecraft:sulfur_pool", |loader, _| Ok(arc(SulfurPool::new(loader.table()))));
    registry.register_id("minecraft:cactus_feature", |loader, _| Ok(arc(Cactus::new(loader.table()))));
    registry.register_id("minecraft:bamboo_feature", |loader, _| Ok(arc(Bamboo::new(loader.table()))));
    registry.register_id("minecraft:desert_well_feature", |loader, _| Ok(arc(DesertWell::new(loader.table()))));
    registry.register_id("minecraft:legacy:ice_feature", |loader, _| Ok(arc(IcebergFeature::new(loader.table()))));
    registry.register_id("minecraft:ice_patch_feature", |loader, _| Ok(arc(IcePatch::new(loader.table()))));
    registry.register_id("minecraft:ice_spike_feature", |loader, _| Ok(arc(IceSpike::new(loader.table()))));
    registry.register_id("minecraft:coral_feature", |loader, _| Ok(arc(Coral::new(loader.table()))));
    registry.register_id("minecraft:coral_hang_feature", |loader, _| Ok(arc(CoralHang(Coral::new(loader.table())))));
    registry.register_id("minecraft:coral_crust_feature", |loader, _| Ok(arc(CoralCrust::new(loader.table()))));
    registry.register_id("minecraft:sea_anemone_feature", |loader, _| Ok(arc(SeaAnemone(Coral::new(loader.table())))));
    registry.register_id("minecraft:legacy:forest_flower_feature", |loader, _| Ok(arc(Flower::new(loader.table(), FlowerMode::Forest))));
    registry.register_id("minecraft:legacy:overworld_flower_feature", |loader, _| Ok(arc(Flower::new(loader.table(), FlowerMode::Default))));
    registry.register_id("minecraft:legacy:swamp_flower_feature", |loader, _| Ok(arc(Flower::new(loader.table(), FlowerMode::Swamp))));
    registry.register_id("minecraft:legacy:plains_flower_feature", |loader, _| Ok(arc(Flower::new(loader.table(), FlowerMode::Plains))));
    registry.register_id("minecraft:legacy:flower_forest_flower_feature", |loader, _| {
        Ok(arc(Flower::new(loader.table(), FlowerMode::FlowerForest)))
    });
    Pale::register(registry);
}
