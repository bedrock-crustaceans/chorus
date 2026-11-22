use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::pack::PackSet;
use super::packs::GRID_EXTENSION;
use super::pool::StatePoolBuilder;
use super::read::{LocalState, Parsed, load_grid, load_listed, load_packed};
use super::source::PackSource;
use super::template::{Jigsaw, StateKind, Template, TemplateBlock};

const MAX_HORIZONTAL_SIZE: u16 = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateTooLarge {
    pub name: String,
    pub size: [u16; 3],
}

impl fmt::Display for TemplateTooLarge {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}: size {:?} is past the {MAX_HORIZONTAL_SIZE} block horizontal limit of the template format",
            self.name, self.size
        )
    }
}

impl Error for TemplateTooLarge {}

pub struct TemplateStore {
    templates: HashMap<String, Arc<Template>>,
    names: Vec<String>,
}

fn parse(packs: &PackSet, entry: &str) -> Result<Parsed, String> {
    if let Some(packed) = packs.packed(entry) {
        return match packed.map_err(|error| format!("{entry}: {error}"))? {
            bedrock_vanilla::Entry::Listed(listed) => load_packed(&listed).map_err(|error| format!("{entry}: {error}")),
            bedrock_vanilla::Entry::Raw(bytes) => parse_bytes(entry, bytes),
        };
    }
    let bytes = packs.read(entry).ok_or_else(|| format!("{entry}: missing entry"))?;
    parse_bytes(entry, &bytes)
}

fn parse_bytes(entry: &str, bytes: &[u8]) -> Result<Parsed, String> {
    let parsed = if entry.ends_with(GRID_EXTENSION) { load_grid(bytes) } else { load_listed(bytes) };
    parsed.map_err(|error| format!("{entry}: {error}"))
}

fn parse_all(packs: &PackSet, entries: &[(String, String)]) -> Vec<Result<Parsed, String>> {
    let threads = std::thread::available_parallelism().map_or(1, |count| count.get()).clamp(1, entries.len().max(1));
    let next = AtomicUsize::new(0);
    let mut results: Vec<Option<Result<Parsed, String>>> = (0..entries.len()).map(|_| None).collect();
    std::thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                scope.spawn(|| {
                    let mut done = Vec::new();
                    loop {
                        let at = next.fetch_add(1, Ordering::Relaxed);
                        let Some((_, entry)) = entries.get(at) else { break };
                        done.push((at, parse(packs, entry)));
                    }
                    done
                })
            })
            .collect();
        for worker in workers {
            for (at, result) in worker.join().expect("template reader thread") {
                results[at] = Some(result);
            }
        }
    });
    results.into_iter().map(|result| result.expect("every template is parsed")).collect()
}

impl TemplateStore {
    pub fn load(packs: &PackSet, pool: &mut StatePoolBuilder) -> Result<Self, Box<dyn Error>> {
        let entries = packs.template_entries()?;
        if entries.is_empty() {
            return Err("no structure templates in the loaded packs".into());
        }
        let mut templates = HashMap::with_capacity(entries.len());
        let mut names = Vec::with_capacity(entries.len());
        let parsed = parse_all(packs, &entries);
        for ((name, _), parsed) in entries.into_iter().zip(parsed) {
            let parsed = parsed?;
            let [size_x, size_y, size_z] = parsed.size;
            if size_x.max(size_z) >= MAX_HORIZONTAL_SIZE {
                return Err(TemplateTooLarge { name, size: parsed.size }.into());
            }
            let global: Vec<u16> = parsed
                .palette
                .iter()
                .map(|local| match local {
                    LocalState::Block(state) => pool.intern(&state.name, &state.states, state.waterlogged),
                    LocalState::Jigsaw => pool.intern_kind(StateKind::Jigsaw),
                    LocalState::StructureBlock => pool.intern_kind(StateKind::StructureBlock),
                })
                .collect();
            let blocks = parsed
                .blocks
                .iter()
                .map(|block| TemplateBlock {
                    state: global[block.state as usize],
                    ..*block
                })
                .collect();
            let jigsaws = parsed
                .jigsaws
                .into_iter()
                .map(|jigsaw| Jigsaw {
                    final_state: global[jigsaw.final_state as usize],
                    ..jigsaw
                })
                .collect();
            names.push(name.clone());
            templates.insert(
                name.clone(),
                Arc::new(Template {
                    name,
                    size: (i32::from(size_x), i32::from(size_y), i32::from(size_z)),
                    blocks,
                    markers: parsed.markers,
                    loot_chests: parsed.loot_chests,
                    jigsaws,
                    entities: parsed.entities,
                }),
            );
        }
        Ok(Self { templates, names })
    }

    pub fn get(&self, name: &str) -> Option<Arc<Template>> {
        self.templates.get(name).cloned()
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.names.iter().map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}
