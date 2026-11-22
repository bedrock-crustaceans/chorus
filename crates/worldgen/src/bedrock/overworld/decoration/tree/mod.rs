mod acacia;
mod beehive;
mod canopy;
mod cherry;
mod decoration;
mod draw;
mod fallen;
mod fancy;
mod foliage;
mod legacy;
mod list;
mod litter;
mod load;
mod mangrove;
mod mega;
mod mushroom;
mod poplar;
mod rock;
mod roofed;
mod search;
mod shelf;
mod simple;
mod site;
mod spec;
#[cfg(test)]
mod tests;

use crate::bedrock::overworld::decoration::ore::Draw;
use crate::bedrock::overworld::decoration::rules::FeatureLoader;
use crate::shared::blocks::BlockId;
use crate::shared::chunk::region::Region;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::Probed;
use glam::IVec3;
use list::BlockList;
use serde_json::Value;
use site::Foliage;
use spec::{BaseCluster, Canopy, MangroveRoots, Trunk};

pub use beehive::OptionalBeehive;
pub use foliage::ForestFoliage;
pub use legacy::{LegacyTrees, SwampFoliage};
pub use litter::FlowerBed;
pub use load::register;
pub use mushroom::HugeMushroom;
pub use rock::ForestRock;
pub use search::{Search, Sequence};
pub use shelf::Shelf;

const BASE_BLOCK_KEY: &str = "base_block";
const MAY_GROW_ON_KEY: &str = "may_grow_on";
const MAY_REPLACE_KEY: &str = "may_replace";
const MAY_GROW_THROUGH_KEY: &str = "may_grow_through";
const ROOTS_KEY: &str = "mangrove_roots";
const BASE_CLUSTER_KEY: &str = "base_cluster";

#[derive(Debug)]
pub struct Tree {
    trunk: Trunk,
    canopy: Option<Canopy>,
    may_grow_on: BlockList,
    may_replace: BlockList,
    may_grow_through: BlockList,
    base_block: Vec<BlockId>,
    roots: Option<MangroveRoots>,
    base_cluster: Option<BaseCluster>,
    foliage: Foliage,
}

pub struct Anchor {
    pub at: IVec3,
    pub candidates: Vec<IVec3>,
}

impl Anchor {
    pub fn at(at: IVec3) -> Self {
        Self { at, candidates: Vec::new() }
    }
}

impl Tree {
    pub fn parse(loader: &mut FeatureLoader, depth: usize, body: &'static Value) -> Result<Self, &'static str> {
        let trunk = Trunk::parse(loader, depth, body)?;
        let table = loader.table();
        let base_block = match body.get(BASE_BLOCK_KEY) {
            Some(Value::Array(names)) => names.iter().map(|name| spec::block(table, name)).collect::<Result<Vec<_>, _>>()?,
            Some(single) => vec![spec::block(table, single)?],
            None => Vec::new(),
        };
        Ok(Self {
            trunk,
            canopy: Canopy::parse(table, body)?,
            may_grow_on: BlockList::parse(table, body.get(MAY_GROW_ON_KEY))?,
            may_replace: BlockList::parse(table, body.get(MAY_REPLACE_KEY))?,
            may_grow_through: BlockList::parse(table, body.get(MAY_GROW_THROUGH_KEY))?,
            base_block,
            roots: body.get(ROOTS_KEY).map(|roots| MangroveRoots::parse(table, roots)).transpose()?,
            base_cluster: body.get(BASE_CLUSTER_KEY).map(|cluster| BaseCluster::parse(table, cluster)).transpose()?,
            foliage: Foliage::new(table),
        })
    }

    pub fn place(&self, region: &mut Region, random: &mut MersenneTwister, at: IVec3) -> Probed<Option<IVec3>> {
        Ok(match &self.trunk {
            Trunk::Simple(trunk) => trunk.place(self, region, random, at),
            Trunk::Acacia(trunk) => trunk.place(self, region, random, at),
            Trunk::Fancy(trunk) => trunk.place(self, region, random, at),
            Trunk::Mega(trunk) => trunk.place(self, region, random, at),
            Trunk::Mangrove(trunk) => trunk.place(self, region, random, at),
            Trunk::Cherry(trunk) => trunk.place(self, region, random, at),
            Trunk::Fallen(trunk) => trunk.place(self, region, random, at)?,
            Trunk::Poplar(trunk) => trunk.place(self, region, random, at)?,
        })
    }

    fn roots_then<D: Draw>(&self, region: &mut Region, random: &mut D, origin: IVec3, height: i32) -> Option<IVec3> {
        match &self.roots {
            None => Some(origin),
            Some(roots) => roots.place(self, region, random, origin, height),
        }
    }

    fn place_canopies<D: Draw>(&self, region: &mut Region, random: &mut D, anchors: &[Anchor]) {
        if let Some(canopy) = &self.canopy {
            for anchor in anchors {
                canopy.place(self, region, random, anchor);
            }
        }
    }
}
