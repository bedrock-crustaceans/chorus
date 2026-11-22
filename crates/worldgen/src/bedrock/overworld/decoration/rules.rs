use super::filter::{BiomeMask, Filter};
use super::fossil::FossilParts;
use super::placed::{Placed, PlacedFeature};
use super::registry::{FeatureDocument, FeatureRef, FeatureRegistry};
use super::scatter::Scatter;
use crate::shared::blocks::BlockTable;
use crate::shared::hash::fnv1_64;
use crate::shared::structure::data::CHORUS_PACK;
use chorus_structure_data::PackSet;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, LazyLock};

const RULES_FOLDER: &str = "feature_rules";
const FEATURES_FOLDER: &str = "features";
const MISSING_FEATURE: &str = "missing feature";
const NOISE_FEATURE: &str = "needs noise";
const NO_DISTRIBUTION: &str = "rule without distribution";
const BIOME_FILTER: &str = "minecraft:biome_filter";
const MAX_DEPTH: usize = 8;

fn convert(document: &mut Value) {
    let map = document.as_object_mut().expect("an embedded worldgen document is a JSON object");
    let version = map.remove("format_version").expect("an embedded worldgen document has a format_version");
    let (kind, body) = map.iter().next().map(|(kind, body)| (kind.clone(), body.clone())).expect("an embedded worldgen document has a body");
    *document = serde_json::json!({"type": kind, "format_version": version, "body": body});
}

#[cfg(test)]
pub(super) fn documents(json: &str) -> Value {
    let mut all: Value = serde_json::from_str(json).expect("worldgen documents json");
    for document in all.as_object_mut().into_iter().flat_map(|map| map.values_mut()) {
        convert(document);
    }
    all
}

fn pack_documents(folder: &str, owner: &str) -> Value {
    let mut all = serde_json::Map::new();
    for (name, mut document) in crate::shared::resources::json_documents(folder, owner) {
        convert(&mut document);
        let identifier = document["body"]["description"]["identifier"]
            .as_str()
            .unwrap_or_else(|| panic!("{name}: missing identifier"))
            .to_owned();
        all.insert(identifier, document);
    }
    Value::Object(all)
}

pub static RULES: LazyLock<Value> = LazyLock::new(|| pack_documents(RULES_FOLDER, PackSet::VANILLA));
pub static FEATURES: LazyLock<Value> = LazyLock::new(|| pack_documents(FEATURES_FOLDER, PackSet::VANILLA));
pub static CHORUS_FEATURES: LazyLock<Value> = LazyLock::new(|| pack_documents(FEATURES_FOLDER, CHORUS_PACK));

pub const PASSES: [&str; 11] = [
    "first_pass",
    "before_underground_pass",
    "underground_pass",
    "after_underground_pass",
    "before_surface_pass",
    "surface_pass",
    "after_surface_pass",
    "before_sky_pass",
    "sky_pass",
    "after_sky_pass",
    "final_pass",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Blocker {
    pub kind: &'static str,
    pub id: Arc<str>,
}

#[derive(Debug)]
pub enum Feature {
    Placed(Arc<Placed>),
    Unsupported(Blocker),
}

#[derive(Debug)]
pub struct Rule {
    pub identifier: &'static str,
    pub hash: u64,
    pub pass: usize,
    pub biomes: BiomeMask,
    pub scatter: Option<Scatter>,
    pub feature: Feature,
}

#[derive(Default)]
pub struct Report {
    pub rule_count: usize,
    pub feature_count: usize,
    pub unsupported_rules: BTreeMap<&'static str, usize>,
    pub unsupported_features: BTreeMap<&'static str, usize>,
    pub missing_features: BTreeMap<Arc<str>, usize>,
    pub unsupported_expressions: BTreeMap<String, usize>,
    pub unsupported_filters: Vec<String>,
    pub unknown_passes: Vec<&'static str>,
}

pub struct Loaded {
    pub rules: BTreeMap<(usize, &'static str), Rule>,
    pub report: Report,
    pub features: HashMap<Arc<str>, Arc<Placed>>,
}

pub struct FeatureLoader<'t, 'a> {
    table: &'t mut BlockTable<'a>,
    registry: &'t FeatureRegistry,
    cache: HashMap<Arc<str>, Result<Arc<Placed>, Blocker>>,
    fossils: Option<Arc<FossilParts>>,
}

pub(super) fn arc<T: PlacedFeature + 'static>(feature: T) -> Arc<Placed> {
    Arc::new(feature)
}

fn blocked(kind: &'static str, id: &str) -> Blocker {
    Blocker { kind, id: Arc::from(id) }
}

impl<'a> FeatureLoader<'_, 'a> {
    pub fn table(&mut self) -> &mut BlockTable<'a> {
        self.table
    }

    pub(super) fn fossils(&self) -> Option<Arc<FossilParts>> {
        self.fossils.clone()
    }

    pub fn feature(&mut self, id: &str, depth: usize) -> Result<Arc<Placed>, Blocker> {
        if let Some(known) = self.cache.get(id) {
            return known.clone();
        }
        let loaded = if depth > MAX_DEPTH { Err(blocked(MISSING_FEATURE, id)) } else { self.build(id, depth) };
        self.cache.insert(Arc::from(id), loaded.clone());
        loaded
    }

    fn build(&mut self, id: &str, depth: usize) -> Result<Arc<Placed>, Blocker> {
        let registry = self.registry;
        let reference = FeatureRef { id: Arc::from(id), depth };
        if let Some(parser) = registry.override_for(id) {
            return parser(self, reference);
        }
        let Some(entry) = registry.document(id) else {
            return match registry.native(id) {
                Some(parser) => parser(self, reference),
                None => Err(blocked(MISSING_FEATURE, id)),
            };
        };
        let document = FeatureDocument {
            id: Arc::from(id),
            kind: entry["type"].as_str().unwrap_or(MISSING_FEATURE),
            body: &entry["body"],
            depth,
        };
        if let Some(parser) = registry.document_id_parser(id) {
            return parser(self, &document);
        }
        match registry.kind_parser(document.kind) {
            Some(parser) => parser(self, &document),
            None => Err(document.blocked()),
        }
    }
}

pub fn order_rules(rules: Vec<Rule>) -> BTreeMap<(usize, &'static str), Rule> {
    rules.into_iter().map(|rule| ((rule.pass, rule.identifier), rule)).collect()
}

pub fn load_with_registry(table: &mut BlockTable, extra_features: &[Arc<str>], registry: &FeatureRegistry) -> Loaded {
    let mut report = Report::default();
    let features: &'static Value = &FEATURES;
    report.feature_count = features.as_object().map_or(0, |map| map.len());
    let fossils = FossilParts::load(table);
    let mut loader = FeatureLoader {
        table,
        registry,
        cache: HashMap::new(),
        fossils,
    };
    for identifier in extra_features {
        let _ = loader.feature(identifier, 0);
    }
    for (identifier, entry) in features.as_object().into_iter().flatten() {
        let identifier: &str = identifier.as_str();
        if loader.feature(identifier, 0).is_err() {
            *report.unsupported_features.entry(entry["type"].as_str().unwrap_or(MISSING_FEATURE)).or_default() += 1;
        }
    }

    let rules_json: &'static Value = &RULES;
    let mut rules = Vec::new();
    for (identifier, entry) in rules_json.as_object().into_iter().flatten() {
        let body = &entry["body"];
        let Some(pass) = body["conditions"]["placement_pass"].as_str().and_then(|name| PASSES.iter().position(|pass| *pass == name)) else {
            report.unknown_passes.push(identifier.as_str());
            continue;
        };
        let filter = body["conditions"].get(BIOME_FILTER).map(Filter::parse);
        if let Some(filter) = &filter {
            filter.collect_unsupported(&mut report.unsupported_filters);
        }
        let scatter = body.get("distribution").map(Scatter::parse);
        if let Some(scatter) = &scatter {
            scatter.collect_unsupported(&mut report.unsupported_expressions);
        }
        let placed = body["description"]["places_feature"].as_str().unwrap_or_default();
        let feature = match loader.feature(placed, 0) {
            Ok(_) if scatter.is_none() => Feature::Unsupported(blocked(NO_DISTRIBUTION, placed)),
            Ok(feature) => Feature::Placed(feature),
            Err(blocker) => Feature::Unsupported(blocker),
        };
        if let Feature::Unsupported(blocker) = &feature {
            *report.unsupported_rules.entry(blocker.kind).or_default() += 1;
            if blocker.kind == MISSING_FEATURE || blocker.kind == NOISE_FEATURE {
                *report.missing_features.entry(blocker.id.clone()).or_default() += 1;
            }
        }
        rules.push(Rule {
            identifier,
            hash: fnv1_64(identifier),
            pass,
            biomes: filter.map_or(BiomeMask::Any, |filter| filter.mask()),
            scatter,
            feature,
        });
    }
    report.rule_count = rules.len();
    let rules = order_rules(rules);
    let features = loader.cache.into_iter().filter_map(|(id, loaded)| loaded.ok().map(|placed| (id, placed))).collect();
    Loaded { rules, report, features }
}
