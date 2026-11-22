use super::builder::{Built, StructureRegistration};
use super::data::{Pack, PackSet, TableSource, embedded_packs};
use super::plan::ColumnHeights;
use super::*;

const GENERAL_1: i64 = 4802433367836389877;

struct Flat<'a>(&'a Structures);

impl StartProbe for Flat<'_> {
    fn starts_at(&self, cell: (i32, i32)) -> Option<Arc<Starts>> {
        Some(Arc::new(self.0.build_starts(self, cell).expect("flat probe supplies everything")))
    }

    fn preliminary_surface(&self, _chunk: (i32, i32)) -> i32 {
        70
    }

    fn surface_at_quart(&self, _quart: (i32, i32)) -> i32 {
        70
    }

    fn biome(&self, _x: i32, _y: i32, _z: i32) -> i32 {
        1
    }

    fn lowest_column_top(&self, _chunk: (i32, i32)) -> Probed<i32> {
        Ok(70)
    }

    fn column_heights(&self, _chunk: (i32, i32)) -> Probed<Arc<ColumnHeights>> {
        Ok(Arc::new(ColumnHeights {
            solid: Box::new([70; 256]),
            top: Box::new([70; 256]),
            min_y: -64,
        }))
    }
}

fn structures(limit: usize) -> Structures {
    Structures::with_limit(GENERAL_1, Arc::new(StructureData::empty()), StructureBlocks::default(), limit)
}

#[test]
fn candidates_stay_inside_their_region() {
    let structures = structures(64);
    for kind in StructureKind::ALL {
        let Some(grid) = structures.registry().any_grid(kind) else { continue };
        for region in -3..3 {
            let (x, z) = structures.candidate(kind, (region * grid.spacing + 1, region * grid.spacing + 1)).unwrap();
            assert_eq!((x.div_euclid(grid.spacing), z.div_euclid(grid.spacing)), (region, region), "{kind:?}");
            assert!(x - region * grid.spacing < grid.spacing - grid.separation);
        }
    }
}

#[test]
fn a_tiny_cache_gives_the_same_starts() {
    let small = structures(16);
    let large = structures(1 << 20);
    for kind in [StructureKind::RuinedPortal, StructureKind::Village, StructureKind::Mineshaft, StructureKind::Shipwreck] {
        assert_eq!(small.starts_near(&Flat(&small), kind, (0, 0), 60), large.starts_near(&Flat(&large), kind, (0, 0), 60), "{kind:?}");
        assert_eq!(small.starts_near(&Flat(&small), kind, (0, 0), 60), large.starts_near(&Flat(&large), kind, (0, 0), 60), "{kind:?} again");
    }
}

#[test]
fn locate_answers_a_start() {
    let structures = structures(1024);
    let found = structures.locate(&Flat(&structures), StructureKind::RuinedPortal, (0, 0)).unwrap().expect("a ruined portal");
    assert!(structures.is_start(&Flat(&structures), StructureKind::RuinedPortal, found).unwrap());
}

fn distance(from: (i32, i32), (x, z): (i32, i32)) -> i64 {
    (x as i64 - from.0 as i64).pow(2) + (z as i64 - from.1 as i64).pow(2)
}

#[test]
fn strongholds_locate_the_nearest_start() {
    let structures = structures(1 << 20);
    for from in [(0, 0), (523, -611), (-1200, 900), (37, -4000)] {
        let found = structures.locate_stronghold(&Flat(&structures), from).unwrap().expect("a stronghold");
        let scanned = structures.starts_near(&Flat(&structures), StructureKind::Stronghold, from, 1200).unwrap();
        assert!(scanned.contains(&found), "{from:?}");
        assert_eq!(distance(from, found), scanned.iter().copied().map(|start| distance(from, start)).min().unwrap(), "{from:?}");
    }
}

#[test]
fn stronghold_locate_has_no_radius_cap() {
    let structures = structures(1 << 20);
    let mut proven = false;
    for step in 0..64 {
        let from = (step * 137 - 4000, step * -211 + 3000);
        if !structures.starts_near(&Flat(&structures), StructureKind::Stronghold, from, 100).unwrap().is_empty() {
            continue;
        }
        let found = structures.locate_stronghold(&Flat(&structures), from).unwrap().expect("a stronghold");
        assert!(distance(from, found) > 100 * 100, "{from:?} {found:?}");
        proven = true;
        break;
    }
    assert!(proven);
}

#[test]
fn grid_locate_returns_the_nearest_start() {
    let structures = structures(1 << 20);
    for kind in [StructureKind::RuinedPortal, StructureKind::Village] {
        for from in [(0, 0), (400, -250), (-900, 700)] {
            let found = structures.locate(&Flat(&structures), kind, from).unwrap().expect("a start");
            let scanned = structures.starts_near(&Flat(&structures), kind, from, 400).unwrap();
            assert!(scanned.contains(&found), "{kind:?} {from:?}");
            assert_eq!(distance(from, found), scanned.iter().copied().map(|start| distance(from, start)).min().unwrap(), "{kind:?} {from:?}");
        }
    }
}

#[test]
fn the_registry_defines_every_kind_and_its_placement() {
    let structures = structures(64);
    let registry = structures.registry();
    for kind in StructureKind::ALL {
        assert_eq!(registry.kind_named(registry.name(kind)), Some(kind), "{kind:?}");
    }
    let village = registry.grid(StructureKind::Village).expect("villages are placed on a grid");
    assert_eq!((village.spacing, village.separation, village.salt), (34, 8, 10_387_312));
    assert!(registry.grid(StructureKind::TrailRuins).is_none() && registry.java_grid(StructureKind::TrailRuins).is_some());
    assert!(registry.any_grid(StructureKind::Stronghold).is_none() && registry.any_grid(StructureKind::Mineshaft).is_none());
    assert!(registry.biomes("village").contains(&1) && registry.biomes("missing").is_empty());
}

struct Marker;

impl StructureBuilder for Marker {
    fn is_start(&self, _: &StructureContext, _: StructureKind, _: (i32, i32)) -> Probed<bool> {
        Ok(true)
    }

    fn build(&self, _: &StructureContext, _: StructureKind, _: (i32, i32)) -> Probed<Option<Built>> {
        Ok(Some(Arc::new(7_u32)))
    }

    fn place(&self, _: &StructureContext, _: &crate::shared::blocks::Blocks, _: &Reaching, _: &mut crate::shared::chunk::proto::ProtoChunk) -> Probed<()> {
        Ok(())
    }
}

const MARKER_SET: &[u8] =
    br#"{"identifier": "test:marker", "kind": "test:marker", "rule": "bedrock", "placement": {"random_spread": {"spacing": 8, "separation": 2, "salt": 4242, "spread": "linear"}}, "reach": 1}"#;
const UNKNOWN_KIND_SET: &[u8] = br#"{"identifier": "test:stray", "kind": "test:nobody", "rule": "bedrock", "placement": "per_chunk", "reach": 1}"#;

static MARKER_FILES: &[(&str, &[u8])] = &[("structure_sets/marker.json", MARKER_SET)];
static UNKNOWN_KIND_FILES: &[(&str, &[u8])] = &[("structure_sets/stray.json", UNKNOWN_KIND_SET)];

fn marker() -> StructureRegistration {
    StructureRegistration::new("test:marker", Arc::new(Marker))
}

fn sources_with(files: &'static [(&'static str, &'static [u8])]) -> PackSet {
    embedded_packs().with(Pack::new("test", Arc::new(TableSource::new(&[files])))).expect("the test files define new names")
}

fn load_error(sources: &PackSet, registrations: Vec<StructureRegistration>) -> String {
    match StructureRegistry::load_with(sources, registrations) {
        Ok(_) => panic!("the registry loaded"),
        Err(error) => error.to_string(),
    }
}

#[test]
fn a_structure_registered_at_runtime_is_placed_and_built_like_a_built_in() {
    let registry = StructureRegistry::load_with(&sources_with(MARKER_FILES), vec![marker()]).expect("loads");
    let kind = registry.registered_kind("test:marker").expect("the marker has a kind");
    assert_eq!(kind, StructureKind(StructureKind::BUILT_IN_COUNT));
    let data = Arc::new(StructureData {
        library: None,
        documents: Default::default(),
        registry,
        catalogues: Default::default(),
    });
    let structures = Structures::new(GENERAL_1, data, StructureBlocks::default());
    assert_eq!(structures.registry().kind_named("test:marker"), Some(kind));
    assert_eq!(structures.registry().name(kind), "test:marker");
    assert!(structures.kinds().contains(&kind));
    let cell = structures.candidate(kind, (0, 0)).expect("the registered kind has a grid");
    let starts = structures.build_starts(&Flat(&structures), cell).unwrap();
    assert!(starts.has(kind));
    assert_eq!(starts.get::<u32>(kind).map(|value| *value), Some(7));
    assert!(structures.starts_near(&Flat(&structures), kind, cell, 4).unwrap().contains(&cell));
}

#[test]
fn built_in_kinds_keep_their_numbers_beside_registered_ones() {
    let registry = StructureRegistry::load_with(&sources_with(MARKER_FILES), vec![marker()]).expect("loads");
    for kind in StructureKind::ALL {
        assert_eq!(registry.registered_kind(&format!("{kind:?}")), Some(kind));
    }
    assert_eq!(registry.registrations().count(), StructureKind::ALL.len() + 1);
}

#[test]
fn a_builder_without_a_structure_set_fails_to_load() {
    let error = load_error(&embedded_packs(), vec![marker()]);
    assert!(error.contains("test:marker") && error.contains("no structure set"), "{error}");
}

#[test]
fn a_structure_set_naming_an_unregistered_kind_fails_to_load() {
    let error = load_error(&sources_with(UNKNOWN_KIND_FILES), Vec::new());
    assert!(error.contains("test:nobody") && error.contains("no registered builder"), "{error}");
}

#[test]
fn a_builder_identifier_registered_twice_fails_to_load() {
    let error = load_error(&sources_with(MARKER_FILES), vec![marker(), marker()]);
    assert!(error.contains("test:marker") && error.contains("more than once"), "{error}");
    let error = load_error(&embedded_packs(), vec![StructureRegistration::new("village", Arc::new(Marker))]);
    assert!(error.contains("village") && error.contains("more than once"), "{error}");
}

#[test]
fn a_registered_builder_declares_its_blocks_while_the_table_is_built() {
    use crate::shared::blocks::BlockTable;
    use std::sync::atomic::{AtomicUsize, Ordering};
    let declared = Arc::new(AtomicUsize::new(0));
    let counter = declared.clone();
    let registration = marker().declaring_blocks(move |_| {
        counter.fetch_add(1, Ordering::SeqCst);
    });
    let registry = StructureRegistry::load_with(&sources_with(MARKER_FILES), vec![registration]).expect("loads");
    let data = StructureData {
        library: None,
        documents: Default::default(),
        registry,
        catalogues: Default::default(),
    };
    let mut blocks = chorus_block::block_registry::BlockRegistry::new();
    blocks.register_all(chorus_block::r#impl::DEFINITIONS.iter().copied());
    StructureBlocks::register(&mut BlockTable::new(&blocks), &data);
    assert_eq!(declared.load(Ordering::SeqCst), 1);
}

#[test]
fn a_structure_set_defined_twice_fails_to_load() {
    static TWICE: &[(&str, &[u8])] = &[("structure_sets/marker.json", MARKER_SET), ("structure_sets/marker_again.json", MARKER_SET)];
    let error = load_error(&sources_with(TWICE), vec![marker()]);
    assert!(error.contains("test:marker") && error.contains("more than once"), "{error}");
}

#[test]
fn the_fast_mineshaft_start_test_agrees_with_the_full_draw() {
    let seed = MineshaftSeed::new(GENERAL_1 as u32);
    for _ in 0..2 {
        for x in -90..90 {
            for z in -90..90 {
                assert_eq!(seed.is_start((x, z)), seed.random((x, z)).0, "{x} {z}");
            }
        }
    }
}

#[test]
fn the_vanilla_pack_loads_the_document_set_the_tables_did() {
    use super::layout::PoolDocuments;
    use crate::shared::hash::fnv1a_64;
    let packs = embedded_packs();
    let all_names = packs.packs()[0].source.names();
    let count_in = |folder: &str| all_names.iter().filter(|name| name.starts_with(folder)).count();
    assert_eq!((count_in("features/"), count_in("feature_rules/"), count_in("climate/")), (290, 267, 1));
    let mut names: Vec<String> = all_names
        .iter()
        .filter(|name| ["biomes/", "jigsaw/", "worldgen/"].iter().any(|folder| name.starts_with(folder)))
        .cloned()
        .collect();
    names.sort();
    let hash = |items: &[String]| fnv1a_64(items.iter().flat_map(|item| item.bytes().chain(std::iter::once(b'\n'))));
    let documents = PoolDocuments::load(&packs).unwrap();
    let mut identifiers: Vec<String> = documents
        .pools
        .iter()
        .chain(documents.structures.iter())
        .chain(documents.structure_sets.iter())
        .map(|document| super::layout::identifier(document).to_owned())
        .chain(documents.processors.keys().cloned())
        .collect();
    identifiers.sort();
    assert_eq!((names.len(), hash(&names)), (182, 0x5193eec0459ca2ce));
    assert_eq!(
        (
            documents.pools.len(),
            documents.families.len(),
            documents.processors.len(),
            documents.structures.len(),
            documents.structure_sets.len(),
            documents.biomes.ids_with_all(&[]).len()
        ),
        (64, 4, 3, 20, 2, 89)
    );
    assert_eq!(hash(&identifiers), 0x5094f1cacded6481);
}

#[test]
fn a_pool_defined_by_two_packs_fails_to_load_naming_both() {
    let known = super::data::PackWorldgen::load(&embedded_packs()).expect("vanilla documents").pools;
    let identifier = super::layout::identifier(&known[0]).to_owned();
    let document: &'static [u8] = Box::leak(
        format!(r#"{{"minecraft:template_pool":{{"description":{{"identifier":"{identifier}"}}}}}}"#)
            .into_bytes()
            .into_boxed_slice(),
    );
    let files: &'static [(&'static str, &'static [u8])] = Box::leak(Box::new([("worldgen/template_pools/clash.json", document)]));
    let error = match super::data::PackWorldgen::load(&sources_with(files)) {
        Ok(_) => panic!("the documents loaded"),
        Err(error) => error.to_string(),
    };
    assert!(error.contains(&identifier) && error.contains("pack vanilla") && error.contains("pack test"), "{error}");
}

#[test]
fn a_name_defined_by_vanilla_and_a_pack_cannot_be_added() {
    use chorus_structure_data::PackSource;
    let existing = embedded_packs().names().into_iter().find(|name| name.starts_with("worldgen/template_pools/")).expect("a vanilla pool");
    let leaked: &'static str = Box::leak(existing.clone().into_boxed_str());
    let clash: &'static [(&'static str, &'static [u8])] = Box::leak(Box::new([(leaked, &b"{}"[..])]));
    let error = embedded_packs().with(Pack::new("test", Arc::new(TableSource::new(&[clash])))).err().expect("a duplicate name");
    assert!(error.to_string().contains(&existing) && error.first == "vanilla" && error.second == "test");
}
