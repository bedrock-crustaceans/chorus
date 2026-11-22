use std::collections::BTreeSet;
use std::fmt::Write;

use sha2::{Digest, Sha256};

use super::{PlacementRule, StructureKind, StructureRegistry};
use crate::shared::structure::data::StructureData;
use crate::shared::structure::data::embedded_packs;
use crate::shared::structure::layout::{Capped, EntryKind, Grounding, PoolCatalogue, PoolDocuments, Processor};
use crate::shared::structure::template::{PaletteEntry, StateKind, TemplateLibrary};

pub(super) fn resolved(entry: &PaletteEntry) -> String {
    format!("{}[{}]{}", entry.name, entry.states, u8::from(entry.waterlogged))
}

fn state_token(library: &TemplateLibrary, state: u16) -> String {
    match library.kind(state) {
        StateKind::Block => resolved(library.entry(state)),
        StateKind::Jigsaw => "J".to_owned(),
        StateKind::StructureBlock => "S".to_owned(),
    }
}

pub(super) fn library_lines(library: &TemplateLibrary) -> Vec<String> {
    let mut lines = Vec::new();
    let mut referenced: BTreeSet<u16> = BTreeSet::new();
    let mut names: Vec<&str> = library.names().collect();
    names.sort_unstable();
    for name in &names {
        let template = library.get(name).expect("a listed template");
        let mut text = String::new();
        writeln!(text, "{:?}", template.size).unwrap();
        for block in &template.blocks {
            writeln!(text, "{} {} {} {}", block.x, block.y, block.z, state_token(library, block.state)).unwrap();
            referenced.insert(block.state);
        }
        writeln!(text, "{:?} {:?}", template.markers, template.loot_chests).unwrap();
        for jigsaw in &template.jigsaws {
            let final_state = resolved(library.entry(jigsaw.final_state));
            writeln!(
                text,
                "{} {} {} {} {} {} {} {} {}",
                jigsaw.block, jigsaw.name, jigsaw.target, jigsaw.pool, jigsaw.joint, jigsaw.orientation, final_state, jigsaw.selection_priority, jigsaw.placement_priority
            )
            .unwrap();
            referenced.insert(jigsaw.final_state);
        }
        lines.push(format!("T {name} blocks {} sha {}", template.blocks.len(), hex(&Sha256::digest(text.as_bytes()))));
        if !template.entities.is_empty() {
            lines.push(format!("E {name} {:?}", template.entities));
        }
    }
    for state in referenced {
        let kind = match library.kind(state) {
            StateKind::Block => 0,
            StateKind::Jigsaw => 1,
            StateKind::StructureBlock => 2,
        };
        let turned: Vec<String> = (0..4).map(|rotation| resolved(library.entry(library.rotated(state, rotation)))).collect();
        lines.push(format!("R {kind} {} -> {}", state_token(library, state), turned.join(" | ")));
    }
    lines.push(format!("N templates {}", names.len()));
    lines.sort();
    lines
}

fn dump_library(out: &mut String, library: &TemplateLibrary) {
    for line in library_lines(library) {
        writeln!(out, "{line}").unwrap();
    }
    let mut pool: Vec<String> = library.entries().iter().map(resolved).collect();
    pool.sort();
    writeln!(out, "pool {}", pool.len()).unwrap();
    for entry in pool {
        writeln!(out, "p {entry}").unwrap();
    }
}

pub(super) fn output_name(output: Option<u16>, library: &TemplateLibrary) -> String {
    output.map_or_else(|| "keep".to_owned(), |index| resolved(library.entry(index)))
}

pub(super) fn processor_line(processor: &Processor, library: &TemplateLibrary) -> String {
    format!(
        "input {:?} location {:?} position {:?} output {}",
        processor.input,
        processor.location,
        processor.position,
        output_name(processor.output, library)
    )
}

pub(super) fn processors_text(processors: &[Processor], library: &TemplateLibrary) -> String {
    processors.iter().map(|processor| processor_line(processor, library)).collect::<Vec<_>>().join("; ")
}

pub(super) fn capped_text(capped: &[Capped], library: &TemplateLibrary) -> String {
    capped
        .iter()
        .map(|cap| format!("capped {} [{}]", cap.limit, processors_text(&cap.rules, library)))
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn dump_catalogue(out: &mut String, tag: &str, catalogue: &PoolCatalogue, library: &TemplateLibrary) {
    writeln!(out, "catalogue {tag} entries {}", catalogue.entries.len()).unwrap();
    for (index, entry) in catalogue.entries.iter().enumerate() {
        match &entry.kind {
            EntryKind::Template {
                template,
                processors,
                grounding,
                foundation,
                aliases,
                capped,
                ..
            } => writeln!(
                out,
                "e {index} template {} follows {} foundation {foundation} aliases {aliases:?} processors [{}] capped [{}]",
                template.name,
                matches!(grounding, Grounding::FollowsTerrain),
                processors_text(processors, library),
                capped_text(capped, library)
            )
            .unwrap(),
            EntryKind::Feature(feature) => writeln!(out, "e {index} feature {feature}").unwrap(),
            EntryKind::Empty => writeln!(out, "e {index} empty").unwrap(),
            EntryKind::Missing => writeln!(out, "e {index} missing").unwrap(),
        }
    }
    let mut pools: Vec<_> = catalogue.pools.iter().collect();
    pools.sort_by(|a, b| a.0.cmp(b.0));
    for (name, pool) in pools {
        writeln!(out, "pool {name} fallback {:?} list {:?}", pool.fallback, pool.list).unwrap();
    }
    let mut bindings: Vec<_> = catalogue.bindings.iter().collect();
    bindings.sort_by(|a, b| a.0.cmp(b.0));
    for (name, groups) in bindings {
        writeln!(out, "bindings {name} {groups:?}").unwrap();
    }
    let mut variants: Vec<_> = catalogue.variants.iter().collect();
    variants.sort_by(|a, b| a.0.cmp(b.0));
    for (set, list) in variants {
        for variant in list {
            writeln!(
                out,
                "variant {set} {} {} {} {} {} {:?}",
                variant.pool, variant.weight, variant.max_depth, variant.horizontal, variant.padding, variant.biomes
            )
            .unwrap();
        }
    }
}

fn dump_documents(out: &mut String, documents: &PoolDocuments) {
    writeln!(
        out,
        "documents pools {} processors {} structures {} sets {}",
        documents.pools.len(),
        documents.processors.len(),
        documents.structures.len(),
        documents.structure_sets.len()
    )
    .unwrap();
    for pool in &documents.pools {
        writeln!(out, "dp {pool}").unwrap();
    }
    let mut processors: Vec<_> = documents.processors.iter().collect();
    processors.sort_by(|a, b| a.0.cmp(b.0));
    for (name, document) in processors {
        writeln!(out, "dr {name} {document}").unwrap();
    }
    let mut structures: Vec<String> = documents.structures.iter().map(|structure| structure.to_string()).collect();
    structures.sort();
    for structure in structures {
        writeln!(out, "ds {structure}").unwrap();
    }
    for set in &documents.structure_sets {
        writeln!(out, "dt {set}").unwrap();
    }
    let tags = documents.biomes.tags();
    writeln!(out, "biome tags {}", tags.len()).unwrap();
    for tag in tags {
        writeln!(out, "bt {tag} {:?}", documents.biomes.ids_with(tag)).unwrap();
    }
}

fn dump_registry(out: &mut String, registry: &StructureRegistry) {
    for def in registry.definitions() {
        let rule = match def.rule {
            PlacementRule::Bedrock => "bedrock",
            PlacementRule::Java => "java",
        };
        let filter = def.filter.as_ref().map(|filter| (filter.tag.clone(), filter.radius));
        let adaptation = def.adaptation.as_ref().map(|rule| (rule.mode.clone(), rule.order));
        let kind = StructureKind::ALL.iter().position(|kind| *kind == def.kind);
        writeln!(out, "def {} {kind:?} {rule} {} {:?} {filter:?} {adaptation:?}", def.identifier, def.reach, def.grid).unwrap();
    }
    let mut tags: Vec<_> = registry.tags.iter().collect();
    tags.sort_by(|a, b| a.0.cmp(b.0));
    for (name, ids) in tags {
        writeln!(out, "tag {name} {ids:?}").unwrap();
    }
}

const PINNED_SHA256: &str = "201a3d5124b3e2c9c90b81ff942f65ff270ec27ab76cb2bfe0d8b1776057e98d";

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
#[ignore = "run with: cargo test --release -p chorus_worldgen --lib -- --ignored embedded_data_keeps_its_pinned_dump"]
fn embedded_data_keeps_its_pinned_dump() {
    let sources = embedded_packs();
    let mut data = StructureData::load().expect("structure data");
    let library = data.library.take().expect("template library");
    let (documents, registry) = (PoolDocuments::load(&sources).expect("pool documents"), StructureRegistry::load(&sources).expect("structure registry"));
    let mut out = String::new();
    dump_library(&mut out, &library);
    dump_documents(&mut out, &documents);
    dump_registry(&mut out, &registry);
    for tag in ["v2", "village", "bastion"] {
        dump_catalogue(&mut out, tag, &PoolCatalogue::build(&documents, tag, &library).expect("pool catalogue"), &library);
    }
    if let Some(path) = std::env::var_os("CHORUS_GATE_OUT") {
        std::fs::write(path, &out).unwrap();
    }
    assert_eq!(hex(&Sha256::digest(out.as_bytes())), PINNED_SHA256);
}
