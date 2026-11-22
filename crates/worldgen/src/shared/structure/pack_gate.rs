use super::data::{PackSet, PackWorldgen, embedded_packs};
use super::data_driven::JigsawStructureBuilder;
use super::layout::{Capped, EntryKind, Grounding, PoolCatalogue, PoolDocuments, Processor, StartHeight};
use super::registry::StructureRegistry;
use super::template::TemplateLibrary;
use super::{Grid, Spread, StructureBlocks, StructureData, StructureKind, Structures};
use crate::shared::hash::fnv1a_64;

const CATALOGUE: &str = "v2";
const POOL_PREFIXES: [&str; 2] = ["trail_ruins/", "abandoned_camp/"];
const VARIANT_SET: &str = "abandoned_camp";
const PINNED_DUMP: u64 = 0xbd6a86da8a4cb7be;

struct Dump {
    pools: usize,
    entries: usize,
    processors: usize,
    variants: usize,
    lines: Vec<String>,
}

fn embedded() -> (PackSet, TemplateLibrary) {
    let sources = embedded_packs();
    let mut data = StructureData::load().expect("");
    let library = data.library.take().expect("");
    PackWorldgen::load(&sources).expect("");
    (sources, library)
}

fn output_name(output: Option<u16>, library: &TemplateLibrary) -> String {
    output.map_or_else(|| "keep".to_owned(), |index| format!("{}[{}]", library.entry(index).name, library.entry(index).states))
}

fn processor_line(processor: &Processor, library: &TemplateLibrary) -> String {
    format!(
        "input {:?} location {:?} position {:?} output {}",
        processor.input,
        processor.location,
        processor.position,
        output_name(processor.output, library)
    )
}

fn capped_line(capped: &Capped, library: &TemplateLibrary) -> String {
    let rules: Vec<String> = capped.rules.iter().map(|rule| processor_line(rule, library)).collect();
    format!("capped {} [{}]", capped.limit, rules.join("; "))
}

fn dump(catalogue: &PoolCatalogue, library: &TemplateLibrary) -> Dump {
    let mut names: Vec<&String> = catalogue.pools.keys().filter(|name| POOL_PREFIXES.iter().any(|prefix| name.starts_with(prefix))).collect();
    names.sort();
    let mut result = Dump {
        pools: names.len(),
        entries: 0,
        processors: 0,
        variants: 0,
        lines: Vec::new(),
    };
    for name in names {
        let pool = &catalogue.pools[name];
        result.lines.push(format!("pool {name} fallback {:?} size {}", pool.fallback, pool.list.len()));
        for &index in &pool.list {
            result.entries += 1;
            let line = match &catalogue.entries[index].kind {
                EntryKind::Template {
                    template,
                    processors,
                    grounding,
                    foundation,
                    aliases,
                    capped,
                    ..
                } => {
                    result.processors += processors.len() + capped.iter().map(|cap| cap.rules.len()).sum::<usize>();
                    let rules: Vec<String> = processors.iter().map(|rule| processor_line(rule, library)).collect();
                    let caps: Vec<String> = capped.iter().map(|cap| capped_line(cap, library)).collect();
                    format!(
                        "  template {} grounding {} foundation {foundation} aliases {aliases:?} rules [{}] {}",
                        template.name,
                        matches!(grounding, Grounding::FollowsTerrain),
                        rules.join("; "),
                        caps.join(" ")
                    )
                }
                EntryKind::Feature(feature) => format!("  feature {feature}"),
                EntryKind::Empty => "  empty".to_owned(),
                EntryKind::Missing => "  missing".to_owned(),
            };
            result.lines.push(line);
        }
    }
    let mut bindings: Vec<&String> = catalogue.bindings.keys().filter(|name| POOL_PREFIXES.iter().any(|prefix| name.starts_with(prefix))).collect();
    bindings.sort();
    for name in bindings {
        result.lines.push(format!("bindings {name} {:?}", catalogue.bindings[name]));
    }
    for variant in catalogue.variants.get(VARIANT_SET).into_iter().flatten() {
        result.variants += 1;
        result.lines.push(format!(
            "variant {} weight {} depth {} horizontal {} padding {} biomes {:?}",
            variant.pool, variant.weight, variant.max_depth, variant.horizontal, variant.padding, variant.biomes
        ));
    }
    result
}

fn runtime_catalogue(sources: &PackSet, library: &TemplateLibrary) -> PoolCatalogue {
    PoolCatalogue::build(&PoolDocuments::load(sources).expect("runtime documents"), CATALOGUE, library).expect("runtime catalogue")
}

fn checksum(lines: &[String]) -> u64 {
    fnv1a_64(lines.iter().flat_map(|line| line.bytes().chain(std::iter::once(b'\n'))))
}

#[test]
fn pack_sourced_pools_keep_their_pinned_dump() {
    let (sources, library) = embedded();
    let dumped = dump(&runtime_catalogue(&sources, &library), &library);
    assert_eq!((dumped.pools, dumped.entries, dumped.processors, dumped.variants, dumped.lines.len()), (64, 1167, 393, 18, 1249));
    assert_eq!(checksum(&dumped.lines), PINNED_DUMP);
    assert!(
        dumped
            .lines
            .iter()
            .any(|line| line.ends_with("abandoned_camp/tent/dappled_forest weight 1 depth 3 horizontal 116 padding 10 biomes [195]"))
    );
}

#[test]
fn pack_placement_and_biomes_match_the_known_numbers() {
    let (sources, _) = embedded();
    let registry = StructureRegistry::load(&sources).expect("registry");
    let expected = [
        (StructureKind::TrailRuins, Grid::new(34, 8, 83469867, Spread::Linear)),
        (StructureKind::AbandonedCamp, Grid::new(37, 8, 91231127, Spread::Linear)),
    ];
    for (kind, grid) in expected {
        assert_eq!(format!("{:?}", registry.any_grid(kind)), format!("{:?}", Some(grid)), "{kind:?}");
    }
    let mut biomes = registry.biomes("trail_ruins").to_vec();
    biomes.sort_unstable();
    assert_eq!(biomes, [5, 21, 30, 32, 155, 160]);
}

#[test]
fn v2_start_specs_come_from_the_structure_documents() {
    let data = StructureData::shared().expect("");
    let structures = Structures::new(1, data, StructureBlocks::default());
    let ruins_builder = JigsawStructureBuilder::trail_ruins();
    let ruins = ruins_builder.spec(&structures).expect("trail ruins spec");
    assert_eq!(
        (ruins.pool, ruins.max_depth, ruins.horizontal, ruins.vertical, ruins.project, ruins.padding, ruins.aliases),
        ("trail_ruins/tower", 7, 80, 80, true, 0, None)
    );
    assert!(matches!(ruins.height, StartHeight::Constant(-15)));
    let chambers_builder = JigsawStructureBuilder::trial_chambers();
    let chambers = chambers_builder.spec(&structures).expect("trial chambers spec");
    assert_eq!(
        (
            chambers.pool,
            chambers.max_depth,
            chambers.horizontal,
            chambers.vertical,
            chambers.project,
            chambers.padding,
            chambers.aliases
        ),
        ("trial_chambers/chamber/end", 20, 116, 116, false, 10, Some("trial_chambers"))
    );
    assert!(matches!(chambers.height, StartHeight::Uniform(-40, -20)));
}
