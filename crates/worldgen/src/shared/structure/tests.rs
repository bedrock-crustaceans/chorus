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
fn inconsistent_builders_and_structure_sets_fail_to_load() {
    static TWICE: &[(&str, &[u8])] = &[("structure_sets/marker.json", MARKER_SET), ("structure_sets/marker_again.json", MARKER_SET)];
    let cases = [
        (embedded_packs(), vec![marker()], ["test:marker", "no structure set"]),
        (sources_with(UNKNOWN_KIND_FILES), Vec::new(), ["test:nobody", "no registered builder"]),
        (sources_with(MARKER_FILES), vec![marker(), marker()], ["test:marker", "more than once"]),
        (embedded_packs(), vec![StructureRegistration::new("village", Arc::new(Marker))], ["village", "more than once"]),
        (sources_with(TWICE), vec![marker()], ["test:marker", "more than once"]),
    ];
    for (sources, registrations, expected) in cases {
        let error = load_error(&sources, registrations);
        assert!(expected.iter().all(|part| error.contains(part)), "{error}");
    }
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
