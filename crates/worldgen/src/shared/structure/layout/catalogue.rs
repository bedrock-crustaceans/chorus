use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use crate::shared::structure::data::jigsaw::{Element, Input, Location, Pool as FamilyPool, Projection, RuleSet as FamilyRuleSet};
use serde_json::Value;

use super::blocks::BlockResolver;
use super::documents::{PoolDocuments, bare, identifier};
use super::entry::{Capped, EntryKind, Grounding, PoolEntry, PreparedJigsaw};
use super::fault::{Fault, PoolError, items, member, signed, text, unsigned};
use super::processors::{LinearPos, Matcher, Processor, is_default_state};
use super::rule_documents::{Rule, Step, parse_processor_list};
use crate::shared::structure::template::TemplateLibrary;

const NAMESPACE: &str = "minecraft:";
const TERRAIN_MATCHING: &str = "terrain_matching";
const EMPTY_POOL: &str = "empty";
const PACK_CATALOGUE: &str = "v2";
const VILLAGE_CATALOGUE: &str = "village";
const BASTION_CATALOGUE: &str = "bastion";
const POST_PROCESS_MODES: [&str; 4] = ["none", "support_block", "beard", "all"];
const POOL_TARGET_KEY: &str = "target_pool";
const RIGID_FOUNDATION: u8 = 0;
const BIOME_TAG_TEST: &str = "has_biome_tag";
const EQUALS: &str = "==";
pub const CATALOGUES: [&str; 3] = [PACK_CATALOGUE, VILLAGE_CATALOGUE, BASTION_CATALOGUE];

pub struct Pool {
    pub fallback: Option<String>,
    pub list: Vec<usize>,
}

pub type AliasChoice = Vec<(String, String)>;

pub struct VariantSet {
    pub pool: String,
    pub weight: i32,
    pub max_depth: i32,
    pub horizontal: i32,
    pub padding: i32,
    pub biomes: Vec<i32>,
}

pub struct PoolCatalogue {
    pub entries: Vec<PoolEntry>,
    pub bindings: HashMap<String, Vec<Vec<AliasChoice>>>,
    pub pools: HashMap<String, Pool>,
    pub variants: HashMap<String, Vec<VariantSet>>,
    pub default_states: Vec<bool>,
}

impl PoolCatalogue {
    pub fn pool(&self, name: &str) -> Option<&Pool> {
        self.pools.get(name.strip_prefix(NAMESPACE).unwrap_or(name))
    }

    pub fn build(documents: &PoolDocuments, catalogue: &str, library: &TemplateLibrary) -> Result<Self, PoolError> {
        let default_states = library.entries().iter().map(|entry| is_default_state(&entry.states)).collect();
        let in_catalogue = |document: &&Value| catalogue_of(identifier(document)) == catalogue;
        let resolver = BlockResolver::new(library.pool());
        let mut cache: ProcessorCache = HashMap::new();
        let mut rule_cache: HashMap<String, Arc<RuleSet>> = HashMap::new();
        let mut entries = Vec::new();
        let mut staged: Vec<StagedPool> = Vec::new();
        for document in documents.pools.iter().filter(in_catalogue) {
            let name = identifier(document);
            let in_pool = |fault: Fault| fault.in_document(name);
            let mut list = Vec::new();
            for item in items(document, "elements").map_err(in_pool)? {
                let weight = unsigned(item, "weight").map_err(in_pool)? as usize;
                list.push((entries.len(), weight));
                let element = member(item, "element").map_err(in_pool)?;
                entries.push(json_entry(element, name, documents, library, &resolver, &mut cache)?);
            }
            let fallback = bare(document["fallback"].as_str().unwrap_or(EMPTY_POOL));
            staged.push((name, (fallback != EMPTY_POOL).then(|| fallback.to_string()), list));
        }
        for family in &documents.families {
            for (name, pool) in &family.pools {
                if catalogue_of(name) != catalogue {
                    continue;
                }
                let mut list = Vec::new();
                for element in &pool.elements {
                    list.push((entries.len(), element.weight() as usize));
                    entries.push(family_entry(element, name, pool, &family.rule_sets, library, &resolver, &mut rule_cache)?);
                }
                let fallback = pool.fallback.as_deref().map(|name| bare(name).to_string());
                staged.push((name, fallback, list));
            }
        }
        let empty = entries.len();
        entries.push(PoolEntry { kind: EntryKind::Empty });
        let pools = staged
            .into_iter()
            .map(|(name, fallback, list)| {
                let list = list
                    .into_iter()
                    .flat_map(|(index, weight)| {
                        let index = if matches!(entries[index].kind, EntryKind::Empty) { empty } else { index };
                        std::iter::repeat_n(index, weight)
                    })
                    .collect();
                (bare(name).to_string(), Pool { fallback, list })
            })
            .collect();
        let mut bindings: HashMap<String, Vec<Vec<AliasChoice>>> = HashMap::new();
        let mut ordered: HashMap<String, Vec<(i64, VariantSet)>> = HashMap::new();
        for document in documents.structures.iter().filter(|_| catalogue == PACK_CATALOGUE) {
            if let Some(aliases) = document["pool_aliases"].as_array() {
                let name = identifier(document);
                let groups = aliases.iter().map(alias_groups).collect::<Result<Vec<_>, _>>().map_err(|fault| fault.in_document(name))?;
                bindings.entry(bare(name).to_string()).or_default().extend(groups);
            }
        }
        for (set, order, variant) in set_variants(documents)? {
            ordered.entry(set).or_default().push((order, variant));
        }
        let variants = ordered
            .into_iter()
            .map(|(set, mut list)| {
                list.sort_by_key(|(order, _)| *order);
                (set, list.into_iter().map(|(_, variant)| variant).collect())
            })
            .collect();
        Ok(Self {
            entries,
            bindings,
            pools,
            variants,
            default_states,
        })
    }
}

type StagedPool<'a> = (&'a str, Option<String>, Vec<(usize, usize)>);

type ProcessorCache = HashMap<String, (Arc<[Processor]>, Arc<[Capped]>)>;

fn horizontal_reach(value: &Value) -> Result<i32, Fault> {
    value
        .as_i64()
        .or_else(|| value.get("horizontal").and_then(Value::as_i64))
        .map(|reach| reach as i32)
        .ok_or(Fault::wrong_type("max_distance_from_center", "an integer or a horizontal reach"))
}

fn filter_tags(structure: &Value) -> Result<Vec<&str>, Fault> {
    let Some(filters) = structure["biome_filters"].as_array() else { return Ok(Vec::new()) };
    filters
        .iter()
        .map(|filter| {
            if filter["test"] != BIOME_TAG_TEST || filter["operator"] != EQUALS {
                return Err(Fault::unknown("biome_filters", filter.to_string()));
            }
            text(filter, "value")
        })
        .collect()
}

fn set_variants(documents: &PoolDocuments) -> Result<Vec<(String, i64, VariantSet)>, PoolError> {
    let mut found = Vec::new();
    for set in &documents.structure_sets {
        let set_name = identifier(set);
        let in_set = |fault: Fault| fault.in_document(set_name);
        for (order, member_document) in items(set, "structures").map_err(in_set)?.iter().enumerate() {
            let wanted = bare(text(member_document, "structure").map_err(in_set)?);
            let Some(structure) = documents.structures.iter().find(|document| bare(identifier(document)) == wanted) else {
                continue;
            };
            let in_structure = |fault: Fault| fault.in_document(identifier(structure));
            let variant = VariantSet {
                pool: bare(text(structure, "start_pool").map_err(in_structure)?).to_string(),
                weight: signed(member_document, "weight").map_err(in_set)? as i32,
                max_depth: signed(structure, "max_depth").map_err(in_structure)? as i32,
                horizontal: horizontal_reach(&structure["max_distance_from_center"]).map_err(in_structure)?,
                padding: structure["dimension_padding"].as_i64().unwrap_or(0) as i32,
                biomes: documents.biomes.ids_with_all(&filter_tags(structure).map_err(in_structure)?),
            };
            found.push((bare(set_name).to_string(), order as i64, variant));
        }
    }
    Ok(found)
}

fn catalogue_of(pool_name: &str) -> &'static str {
    match pool_name.strip_prefix(NAMESPACE).unwrap_or(pool_name).split('/').next() {
        Some("village" | "ancient_city") => VILLAGE_CATALOGUE,
        Some("bastion") => BASTION_CATALOGUE,
        _ => PACK_CATALOGUE,
    }
}

fn alias_groups(alias: &Value) -> Result<Vec<AliasChoice>, Fault> {
    items(alias, "groups")?.iter().map(|group| alias_pairs(member(group, "data")?)).collect()
}

fn alias_pairs(list: &Value) -> Result<AliasChoice, Fault> {
    list.as_array()
        .ok_or(Fault::wrong_type("data", "a list"))?
        .iter()
        .map(|pair| Ok((bare(text(pair, "alias")?).to_string(), bare(text(pair, "target")?).to_string())))
        .collect()
}

fn processor_document<'a>(documents: &'a PoolDocuments, name: &str) -> Result<&'a Value, Fault> {
    documents.processors.get(bare(name)).ok_or_else(|| Fault::undeclared("processors", name))
}

fn json_entry(element: &Value, pool: &str, documents: &PoolDocuments, library: &TemplateLibrary, resolver: &BlockResolver, cache: &mut ProcessorCache) -> Result<PoolEntry, PoolError> {
    let in_pool = |fault: Fault| fault.in_document(pool);
    let kind = match text(element, "element_type").map_err(in_pool)? {
        "minecraft:feature_pool_element" => EntryKind::Feature(bare(text(element, "feature").map_err(in_pool)?).to_string()),
        "minecraft:empty_pool_element" => EntryKind::Empty,
        _ => match library.get(text(element, "location").map_err(in_pool)?) {
            None => EntryKind::Missing,
            Some(template) => {
                let processor_name = element["processors"].as_str();
                let document = processor_name.map(|name| processor_document(documents, name)).transpose().map_err(in_pool)?;
                let (processors, capped) = match (processor_name, document) {
                    (Some(name), Some(document)) => cached_processors(bare(name), document, resolver, cache)?,
                    _ => (Arc::from([]), Arc::from([])),
                };
                let aliases = match (processor_name, document) {
                    (Some(name), Some(document)) => pool_aliases(document).map_err(|fault| fault.in_document(bare(name)))?,
                    _ => None,
                };
                let prepared = PreparedJigsaw::all(&template, aliases.as_deref());
                EntryKind::Template {
                    template,
                    processors,
                    grounding: if element["projection"].as_str().is_some_and(|projection| bare(projection) == TERRAIN_MATCHING) {
                        Grounding::FollowsTerrain
                    } else {
                        Grounding::Fixed
                    },
                    foundation: element["post_process"].as_str().map_or(Ok(RIGID_FOUNDATION), post_process_flags).map_err(in_pool)?,
                    #[cfg(test)]
                    aliases,
                    capped,
                    prepared,
                }
            }
        },
    };
    Ok(PoolEntry { kind })
}

fn cached_processors(name: &str, document: &Value, resolver: &BlockResolver, cache: &mut ProcessorCache) -> Result<(Arc<[Processor]>, Arc<[Capped]>), PoolError> {
    if let Some(found) = cache.get(name) {
        return Ok(found.clone());
    }
    let built = processor_list(document, resolver).map_err(|fault| fault.in_document(name))?;
    cache.insert(name.to_owned(), built.clone());
    Ok(built)
}

fn post_process_flags(mode: &str) -> Result<u8, Fault> {
    POST_PROCESS_MODES
        .iter()
        .position(|known| *known == mode)
        .map(|flags| flags as u8)
        .ok_or_else(|| Fault::unknown("post_process", mode))
}

fn pool_aliases(document: &Value) -> Result<Option<Arc<[(String, String)]>>, Fault> {
    let Some(rules) = document["bedrock:tag_rules"].as_array() else { return Ok(None) };
    let mut pairs: Vec<(String, String)> = Vec::new();
    for rule in rules.iter().filter(|rule| rule["input_predicate"]["key"] == POOL_TARGET_KEY && rule["output_key"] == POOL_TARGET_KEY) {
        let tag = text(member(rule, "input_predicate")?, "value")?;
        pairs.push((bare(tag).to_string(), bare(text(rule, "output_value")?).to_string()));
    }
    Ok((!pairs.is_empty()).then(|| Arc::from(pairs)))
}

fn resolve_rules(rules: Vec<Rule>, resolver: &BlockResolver) -> Result<Vec<Processor>, Fault> {
    rules
        .into_iter()
        .map(|rule| {
            Ok(Processor {
                input: rule.input,
                location: rule.location,
                position: rule.position,
                output: resolver.resolve(rule.output)?,
            })
        })
        .collect()
}

fn processor_list(document: &Value, resolver: &BlockResolver) -> Result<(Arc<[Processor]>, Arc<[Capped]>), Fault> {
    let mut rules = Vec::new();
    let mut capped = Vec::new();
    for step in parse_processor_list(document)? {
        match step {
            Step::Rules(listed) => rules.extend(resolve_rules(listed, resolver)?),
            Step::Capped { limit, rules } => capped.push(Capped {
                limit,
                rules: resolve_rules(rules, resolver)?.into(),
            }),
        }
    }
    Ok((rules.into(), capped.into()))
}

struct RuleSet {
    processors: Arc<[Processor]>,
    aliases: Option<Arc<[(String, String)]>>,
}

fn family_entry(
    element: &Element,
    pool_name: &str,
    pool: &FamilyPool,
    rule_sets: &BTreeMap<String, FamilyRuleSet>,
    library: &TemplateLibrary,
    resolver: &BlockResolver,
    cache: &mut HashMap<String, Arc<RuleSet>>,
) -> Result<PoolEntry, PoolError> {
    let (location, rules, projection, post_process) = match element {
        Element::Empty { .. } => return Ok(PoolEntry { kind: EntryKind::Empty }),
        Element::Feature { feature, .. } => {
            return Ok(PoolEntry {
                kind: EntryKind::Feature(bare(feature).to_string()),
            });
        }
        Element::Template {
            location,
            rules,
            projection,
            post_process,
            ..
        } => (location, rules, projection, post_process),
    };
    let Some(template) = library.get(location) else {
        return Ok(PoolEntry { kind: EntryKind::Missing });
    };
    let rules = rules.as_ref().map(|name| cached_rules(name, pool_name, rule_sets, resolver, cache)).transpose()?;
    let aliases = rules.as_ref().and_then(|rules| rules.aliases.clone());
    let prepared = PreparedJigsaw::all(&template, aliases.as_deref());
    Ok(PoolEntry {
        kind: EntryKind::Template {
            template,
            processors: rules.map_or_else(|| Arc::from([]), |rules| rules.processors.clone()),
            grounding: if projection.unwrap_or(pool.projection) == Projection::TerrainMatching {
                Grounding::FollowsTerrain
            } else {
                Grounding::Fixed
            },
            foundation: post_process.unwrap_or(pool.post_process).flags(),
            #[cfg(test)]
            aliases,
            capped: Arc::from([]),
            prepared,
        },
    })
}

fn cached_rules(name: &str, pool_name: &str, rule_sets: &BTreeMap<String, FamilyRuleSet>, resolver: &BlockResolver, cache: &mut HashMap<String, Arc<RuleSet>>) -> Result<Arc<RuleSet>, PoolError> {
    if let Some(found) = cache.get(name) {
        return Ok(found.clone());
    }
    let set = rule_sets.get(name).ok_or_else(|| Fault::undeclared("rules", name).in_document(pool_name))?;
    let built = Arc::new(family_rules(set, resolver).map_err(|fault| fault.in_document(name))?);
    cache.insert(name.to_owned(), built.clone());
    Ok(built)
}

fn family_rules(set: &FamilyRuleSet, resolver: &BlockResolver) -> Result<RuleSet, Fault> {
    let processors = set
        .rules
        .iter()
        .map(|rule| {
            let input = match &rule.from {
                Input::Block(block) => Matcher::Block(bare(block).to_string()),
                Input::Chance { block, chance } => Matcher::RandomBlock(bare(block).to_string(), *chance),
                Input::AnyOf { blocks, chance } => Matcher::RandomInList(blocks.iter().map(|block| bare(block).to_string()).collect(), *chance),
            };
            let location = match (&rule.at, set.not_in.is_empty()) {
                (Some(Location::NotIn(list)), _) => Matcher::NotInList(list.iter().map(|block| bare(block).to_string()).collect()),
                (Some(Location::Block(block)), _) => Matcher::Block(bare(block).to_string()),
                (None, false) => Matcher::NotInList(set.not_in.iter().map(|block| bare(block).to_string()).collect()),
                (None, true) => Matcher::Always,
            };
            let position = rule.fade.as_ref().map(|fade| LinearPos {
                min: fade.chance.0,
                max: fade.chance.1,
                min_d: fade.distance.0,
                max_d: fade.distance.1,
                axis: fade.axis.index(),
            });
            Ok(Processor {
                input,
                location,
                position,
                output: resolver.resolve_states(&rule.to, &rule.to_states)?,
            })
        })
        .collect::<Result<Vec<_>, Fault>>()?;
    let swaps: Vec<(String, String)> = set.swap_pools.iter().map(|(from, to)| (bare(from).to_string(), bare(to).to_string())).collect();
    Ok(RuleSet {
        processors: processors.into(),
        aliases: (!swaps.is_empty()).then(|| Arc::from(swaps)),
    })
}

#[cfg(test)]
mod malformed {
    use super::*;
    use crate::shared::structure::data::StructureData;
    use serde_json::json;

    fn build(pools: Vec<Value>, processors: Vec<(&str, Value)>) -> Result<PoolCatalogue, PoolError> {
        let data = StructureData::shared().expect("the vanilla structure data loads");
        let library = data.library.as_ref().expect("the vanilla library exists");
        let documents = PoolDocuments {
            pools,
            processors: processors.into_iter().map(|(name, document)| (name.to_owned(), document)).collect(),
            ..PoolDocuments::default()
        };
        PoolCatalogue::build(&documents, PACK_CATALOGUE, library)
    }

    fn pool(elements: Value) -> Value {
        json!({"description": {"identifier": "test:pool"}, "elements": elements})
    }

    fn template_element(extra: Value) -> Value {
        let data = StructureData::shared().expect("the vanilla structure data loads");
        let location = data.library.as_ref().and_then(|library| library.names().next().map(str::to_owned)).expect("a template exists");
        let mut element = json!({"element_type": "minecraft:single_pool_element", "location": location});
        element.as_object_mut().expect("an object").extend(extra.as_object().expect("an object").clone());
        element
    }

    fn element(element: Value) -> Value {
        pool(json!([{"weight": 1, "element": element}]))
    }

    fn error_of(result: Result<PoolCatalogue, PoolError>) -> PoolError {
        match result {
            Ok(_) => panic!("the documents were accepted"),
            Err(error) => error,
        }
    }

    #[test]
    fn malformed_pools_are_errors_naming_the_field() {
        let cases = [
            (vec![json!({"description": {"identifier": "test:pool"}})], "elements", "test:pool"),
            (vec![pool(json!([{"weight": "heavy", "element": {}}]))], "weight", "test:pool"),
            (vec![element(json!({}))], "element_type", "test:pool"),
            (vec![element(template_element(json!({"post_process": "everything"})))], "post_process", "test:pool"),
            (vec![element(template_element(json!({"processors": "test:nowhere"})))], "processors", "test:pool"),
        ];
        for (pools, field, document) in cases {
            let error = error_of(build(pools, Vec::new()));
            assert_eq!((error.fault.field, error.document.as_str()), (field, document));
        }
    }

    #[test]
    fn malformed_processors_are_errors_naming_the_field_and_document() {
        let rule = |input: &str, output: &str| json!({"input_predicate": {"predicate_type": input}, "output_state": {"name": output}});
        let list = |processor: Value| json!({"processors": [processor]});
        let rules = |rule: Value| list(json!({"processor_type": "minecraft:rule", "rules": [rule]}));
        let cases = [
            (list(json!({"processor_type": "minecraft:mystery"})), "processor_type"),
            (rules(rule("minecraft:mystery", "minecraft:air")), "predicate_type"),
            (rules(rule("minecraft:always_true", "minecraft:chorus_test_block")), "output_state"),
            (json!({}), "processors"),
        ];
        for (document, field) in cases {
            let pools = vec![element(template_element(json!({"processors": "minecraft:lists"})))];
            let error = error_of(build(pools, vec![("lists", document)]));
            assert_eq!((error.fault.field, error.document.as_str()), (field, "lists"));
        }
    }

    #[test]
    fn well_formed_documents_build() {
        assert!(build(vec![element(json!({"element_type": "minecraft:empty_pool_element"}))], Vec::new()).is_ok());
    }
}
