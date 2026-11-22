use super::super::placed::Placed;
use super::super::registry::{FeatureDocument, FeatureRef, FeatureRegistry};
use super::super::rules::{Blocker, FeatureLoader, arc};
use super::blob::Blob;
use super::lush::{azalea_roots, dripleaf, moss_vegetation, spore_blossom, vines_single_face};
use super::multiface::{Multiface, Threshold};
use super::patch::Patch;
use super::plant::Plant;
use super::sculk::Sculk;
use super::snap::Snap;
use super::speleothem::{Family, Speleothem};
use super::{BlockSet, Cave};
use serde_json::Value;
use std::sync::Arc;

const MISSING_FEATURE: &str = "missing feature";
const SNAP_FEATURE: &str = "minecraft:snap_to_surface_feature";
const PATCH_FEATURE: &str = "minecraft:vegetation_patch_feature";
const PLANT_FEATURE: &str = "minecraft:growing_plant_feature";
const MULTIFACE_FEATURE: &str = "minecraft:multiface_feature";
const THRESHOLD_FEATURE: &str = "minecraft:surface_relative_threshold_feature";
const BLOB_FEATURE: &str = "minecraft:partially_exposed_blob_feature";
const SCULK_FEATURE: &str = "minecraft:sculk_patch_feature";
const WEIGHTED_FEATURE: &str = "minecraft:weighted_random_feature";
const MOSS_VEGETATION: &str = "minecraft:moss_vegetation_feature";
const DRIPLEAF: &str = "minecraft:dripleaf_feature";
const VINES_SINGLE_FACE: &str = "minecraft:vines_single_face_feature";
const SPORE_BLOSSOM: &str = "minecraft:spore_blossom_feature";
const AZALEA_ROOT_SYSTEM: &str = "minecraft:azalea_root_system_feature";
const AZALEA_TREE: &str = "minecraft:azalea_tree_feature";
const POINTED_DRIPSTONE: &str = "minecraft:pointed_dripstone_feature";
const DRIPSTONE_CLUSTER: &str = "minecraft:dripstone_cluster_feature";
const LARGE_DRIPSTONE: &str = "minecraft:large_dripstone_feature";
const SULFUR_SPIKE: &str = "minecraft:sulfur_spike_feature";
const SULFUR_SPIKE_CLUSTER: &str = "minecraft:sulfur_spike_cluster_feature";
const BIG_DRIPLEAFS: [&str; 4] = [
    "minecraft:big_dripleaf_south_feature",
    "minecraft:big_dripleaf_west_feature",
    "minecraft:big_dripleaf_north_feature",
    "minecraft:big_dripleaf_east_feature",
];

fn invalid(kind: &'static str, id: Arc<str>) -> Blocker {
    Blocker { kind, id }
}

pub fn register(registry: &mut FeatureRegistry) {
    for id in [POINTED_DRIPSTONE, DRIPSTONE_CLUSTER, LARGE_DRIPSTONE, SULFUR_SPIKE, SULFUR_SPIKE_CLUSTER] {
        registry.register_override(id, speleothem);
    }
    for id in [MOSS_VEGETATION, VINES_SINGLE_FACE, SPORE_BLOSSOM, AZALEA_ROOT_SYSTEM, DRIPLEAF] {
        registry.register_override(id, lush);
    }
    registry.register(SNAP_FEATURE, snap);
    registry.register(PATCH_FEATURE, patch);
    registry.register(PLANT_FEATURE, |loader, doc| {
        Plant::parse(loader.table(), doc.body).map(|plant| arc(Cave::Plant(plant))).ok_or(doc.blocked())
    });
    registry.register(BLOB_FEATURE, |loader, doc| Blob::parse(loader.table(), doc.body).map(|blob| arc(Cave::Blob(blob))).ok_or(doc.blocked()));
    registry.register(SCULK_FEATURE, |loader, doc| {
        Sculk::parse(loader.table(), doc.body).map(|sculk| arc(Cave::Sculk(sculk))).ok_or(doc.blocked())
    });
    registry.register(WEIGHTED_FEATURE, weighted);
    registry.register(MULTIFACE_FEATURE, |loader, doc| {
        Multiface::parse(loader.table(), doc.body).map(|face| arc(Cave::Multiface(face))).ok_or(doc.blocked())
    });
    registry.register(THRESHOLD_FEATURE, threshold);
}

fn speleothem(loader: &mut FeatureLoader, reference: FeatureRef) -> Result<Arc<Placed>, Blocker> {
    let speleothem = match reference.id.as_ref() {
        POINTED_DRIPSTONE => Speleothem::Pointed(Family::dripstone(loader.table())),
        DRIPSTONE_CLUSTER => Speleothem::Cluster {
            family: Family::dripstone(loader.table()),
            height: [3, 5],
            wetness: [0.1, 0.3, 0.1, 1.0],
        },
        LARGE_DRIPSTONE => Speleothem::Large(Family::dripstone(loader.table())),
        SULFUR_SPIKE => Speleothem::Pointed(Family::sulfur(loader.table())),
        SULFUR_SPIKE_CLUSTER => Speleothem::Cluster {
            family: Family::sulfur(loader.table()),
            height: [1, 4],
            wetness: [0.0, 0.0, 0.0, 0.0],
        },
        _ => return Err(invalid(MISSING_FEATURE, reference.id.clone())),
    };
    Ok(arc(Cave::Speleothem(speleothem)))
}

fn lush(loader: &mut FeatureLoader, reference: FeatureRef) -> Result<Arc<Placed>, Blocker> {
    let lush = match reference.id.as_ref() {
        MOSS_VEGETATION => moss_vegetation(loader.table()),
        VINES_SINGLE_FACE => vines_single_face(loader.table()),
        SPORE_BLOSSOM => spore_blossom(loader.table()),
        AZALEA_ROOT_SYSTEM => {
            let tree = loader.feature(AZALEA_TREE, reference.depth + 1).ok();
            azalea_roots(loader.table(), tree)
        }
        DRIPLEAF => {
            let big = BIG_DRIPLEAFS.iter().map(|name| loader.feature(name, reference.depth + 1)).collect::<Result<Vec<_>, _>>()?;
            dripleaf(loader.table(), big)
        }
        _ => return Err(invalid(MISSING_FEATURE, reference.id.clone())),
    };
    Ok(arc(Cave::Lush(lush)))
}

fn child(loader: &mut FeatureLoader, doc: &FeatureDocument, key: &str) -> Result<Arc<Placed>, Blocker> {
    loader.feature(doc.body.get(key).and_then(Value::as_str).unwrap_or_default(), doc.depth + 1)
}

fn snap(loader: &mut FeatureLoader, doc: &FeatureDocument) -> Result<Arc<Placed>, Blocker> {
    let feature = child(loader, doc, "feature_to_snap")?;
    let allowed = BlockSet::parse(loader.table(), doc.body.get("allowed_surface_blocks"));
    Snap::parse(feature, doc.body, allowed).map(|snap| arc(Cave::Snap(snap))).ok_or(doc.blocked())
}

fn patch(loader: &mut FeatureLoader, doc: &FeatureDocument) -> Result<Arc<Placed>, Blocker> {
    let vegetation = child(loader, doc, "vegetation_feature")?;
    Patch::parse(loader.table(), vegetation, doc.body).map(|patch| arc(Cave::Patch(patch))).ok_or(doc.blocked())
}

fn weighted(loader: &mut FeatureLoader, doc: &FeatureDocument) -> Result<Arc<Placed>, Blocker> {
    let entries = doc.body.get("features").and_then(Value::as_array).ok_or(doc.blocked())?;
    let mut features = Vec::with_capacity(entries.len());
    for entry in entries {
        let pair = entry.as_array().ok_or(doc.blocked())?;
        let name = pair.first().and_then(Value::as_str).ok_or(doc.blocked())?;
        let weight = pair.get(1).and_then(Value::as_f64).ok_or(doc.blocked())? as f32;
        features.push((loader.feature(name, doc.depth + 1)?, weight));
    }
    Ok(arc(Cave::Weighted(features)))
}

fn threshold(loader: &mut FeatureLoader, doc: &FeatureDocument) -> Result<Arc<Placed>, Blocker> {
    let feature = child(loader, doc, "feature_to_place")?;
    let depth = doc.body.get("minimum_distance_below_surface").and_then(Value::as_i64).ok_or(doc.blocked())? as i32;
    Ok(arc(Cave::Threshold(Threshold { feature, depth })))
}
