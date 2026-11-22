use crate::shared::blocks::{BlockId, BlockTable, Blocks};
use crate::shared::tags::Tag;
use chorus_block::block_id::*;
use serde_json::Value;

const TAGS_KEY: &str = "tags";
const NAME_KEY: &str = "name";
const TAG_QUERY_OPEN: &str = "query.any_tag('";
const TAG_QUERY_CLOSE: &str = "')";
const DIRT_TAG: &str = "dirt";
const LEAVES_TAG: &str = "minecraft:leaves";
pub const UNSUPPORTED_TAG: &str = "unsupported block tag";
const DIRT_MEMBERS: [&str; 11] = [
    DIRT,
    GRASS_BLOCK,
    PODZOL,
    COARSE_DIRT,
    MYCELIUM,
    MUD,
    MUDDY_MANGROVE_ROOTS,
    DIRT_WITH_ROOTS,
    MOSS_BLOCK,
    FARMLAND,
    PALE_MOSS_BLOCK,
];

#[derive(Debug, Default)]
pub struct BlockList {
    kinds: Vec<u16>,
    leaves: bool,
}

impl BlockList {
    pub fn parse(table: &mut BlockTable, value: Option<&'static Value>) -> Result<Self, &'static str> {
        let mut list = Self::default();
        let entries: &[Value] = match value {
            Some(Value::Array(entries)) => entries,
            Some(single) => std::slice::from_ref(single),
            None => &[],
        };
        for entry in entries {
            match entry {
                Value::String(name) => list.kinds.push(table.kind(name)),
                Value::Object(map) if map.contains_key(TAGS_KEY) => {
                    let query = map[TAGS_KEY].as_str().unwrap_or_default();
                    let tag = query.strip_prefix(TAG_QUERY_OPEN).and_then(|rest| rest.strip_suffix(TAG_QUERY_CLOSE)).ok_or(UNSUPPORTED_TAG)?;
                    match tag {
                        DIRT_TAG => list.kinds.extend(DIRT_MEMBERS.iter().map(|name| table.kind(*name))),
                        LEAVES_TAG => list.leaves = true,
                        _ => return Err(UNSUPPORTED_TAG),
                    }
                }
                Value::Object(map) => list.kinds.push(table.kind(map.get(NAME_KEY).and_then(Value::as_str).ok_or(UNSUPPORTED_TAG)?)),
                _ => return Err(UNSUPPORTED_TAG),
            }
        }
        list.kinds.sort_unstable();
        list.kinds.dedup();
        Ok(list)
    }

    pub fn from_names(table: &mut BlockTable, names: &[&'static str]) -> Self {
        let mut kinds: Vec<u16> = names.iter().map(|name| table.kind(*name)).collect();
        kinds.sort_unstable();
        kinds.dedup();
        Self { kinds, leaves: false }
    }

    pub fn is_empty(&self) -> bool {
        self.kinds.is_empty() && !self.leaves
    }

    pub fn contains(&self, blocks: &Blocks, block: BlockId) -> bool {
        self.kinds.binary_search(&blocks.entry(block).kind).is_ok() || (self.leaves && blocks.is(block, Tag::Leaves))
    }

    pub fn allows(&self, blocks: &Blocks, block: BlockId) -> bool {
        self.is_empty() || self.contains(blocks, block)
    }
}
