use super::super::decoration::Decoration;
use super::super::draw::{Odds, Range};
use super::super::list::BlockList;
use super::parse::{block, required};
use crate::shared::blocks::{BlockId, BlockTable};
use serde_json::Value;

#[derive(Debug)]
pub struct MangroveRoots {
    pub max_width: i32,
    pub max_length: i32,
    pub y_offset: Range,
    pub root: BlockId,
    pub muddy_root: BlockId,
    pub mud: BlockId,
    pub above_root: Option<(BlockId, Odds)>,
    pub may_grow_through: BlockList,
    pub decoration: Option<Decoration>,
}

#[derive(Debug)]
pub struct BaseCluster {
    pub may_replace: BlockList,
    pub count: i32,
    pub radius: i32,
}

impl MangroveRoots {
    pub fn parse(table: &mut BlockTable, roots: &'static Value) -> Result<Self, &'static str> {
        let above_root = match roots.get("above_root") {
            Some(above) => Some((block(table, &above["above_root_block"])?, Odds::parse(above.get("above_root_chance")))),
            None => None,
        };
        Ok(Self {
            max_width: required(roots, "max_root_width")?,
            max_length: required(roots, "max_root_length")?,
            y_offset: Range::parse(roots.get("y_offset"), Range::constant(0)),
            root: block(table, &roots["root_block"])?,
            muddy_root: block(table, &roots["muddy_root_block"])?,
            mud: block(table, &roots["mud_block"])?,
            above_root,
            may_grow_through: BlockList::parse(table, roots.get("roots_may_grow_through"))?,
            decoration: Decoration::parse(table, roots.get("root_decoration"))?,
        })
    }
}

impl BaseCluster {
    pub fn parse(table: &mut BlockTable, cluster: &'static Value) -> Result<Self, &'static str> {
        Ok(Self {
            may_replace: BlockList::parse(table, cluster.get("may_replace"))?,
            count: required(cluster, "num_clusters")?,
            radius: required(cluster, "cluster_radius")?,
        })
    }
}
