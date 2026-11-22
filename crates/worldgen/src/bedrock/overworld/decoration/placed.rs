use super::cave::Cave;
use super::coral::{Coral, CoralCrust};
use super::disk::Disk;
use super::expression::Variables;
use super::fossil::Fossil;
use super::geode::Geode;
use super::ice::{IcePatch, IceSpike};
use super::iceberg::IcebergFeature;
use super::ore::{Ore, RegionCells};
use super::pale::Pale;
use super::plants::{Bamboo, Cactus, Flower};
use super::pool::SulfurPool;
use super::scatter::Draws;
use super::scatter::Scatter;
use super::springs::Springs;
use super::structure_template::StructureTemplate;
use super::tree::{FlowerBed, ForestFoliage, ForestRock, HugeMushroom, LegacyTrees, OptionalBeehive, Search, Sequence, Shelf, SwampFoliage, Tree};
use super::vegetation::Vegetation;
use super::well::DesertWell;
use crate::shared::chunk::region::Region;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::Probed;
use glam::IVec3;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EarlyOut {
    None,
    FirstFailure,
    FirstSuccess,
}

#[derive(Debug)]
pub struct ScatterFeature {
    pub scatter: Scatter,
    pub project_to_floor: bool,
    pub inner: Arc<Placed>,
}

#[derive(Debug)]
pub struct Aggregate {
    pub early_out: EarlyOut,
    pub features: Vec<Arc<Placed>>,
}

pub trait PlacedFeature: std::fmt::Debug + Send + Sync {
    fn run(&self, region: &mut Region, random: &mut MersenneTwister, pos: IVec3) -> Probed<bool>;

    fn run_at(&self, region: &mut Region, random: &mut MersenneTwister, pos: IVec3) -> Probed<Option<IVec3>> {
        Ok(self.run(region, random, pos)?.then_some(pos))
    }

    fn as_any(&self) -> &dyn std::any::Any;
}

pub type Placed = dyn PlacedFeature;

impl dyn PlacedFeature {
    pub fn place_at(&self, region: &mut Region, random: &mut MersenneTwister, pos: IVec3) -> Probed<Option<IVec3>> {
        region.begin_feature();
        self.run_at(region, random, pos)
    }

    pub fn place(&self, region: &mut Region, random: &mut MersenneTwister, pos: IVec3) -> Probed<bool> {
        region.begin_feature();
        self.run(region, random, pos)
    }
}

macro_rules! plain {
    ($($t:ty),* $(,)?) => {$(
        impl PlacedFeature for $t {
            fn run(&self, region: &mut Region, random: &mut MersenneTwister, pos: IVec3) -> Probed<bool> {
                Ok(self.place(region, random, pos))
            }

            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
        }
    )*};
}

macro_rules! probed {
    ($($t:ty),* $(,)?) => {$(
        impl PlacedFeature for $t {
            fn run(&self, region: &mut Region, random: &mut MersenneTwister, pos: IVec3) -> Probed<bool> {
                self.place(region, random, pos)
            }

            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
        }
    )*};
}

macro_rules! located {
    ($($t:ty),* $(,)?) => {$(
        impl PlacedFeature for $t {
            fn run(&self, region: &mut Region, random: &mut MersenneTwister, pos: IVec3) -> Probed<bool> {
                Ok(self.place(region, random, pos)?.is_some())
            }

            fn run_at(&self, region: &mut Region, random: &mut MersenneTwister, pos: IVec3) -> Probed<Option<IVec3>> {
                self.place(region, random, pos)
            }

            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
        }
    )*};
}

plain!(
    StructureTemplate,
    IcebergFeature,
    IcePatch,
    IceSpike,
    Disk,
    Springs,
    SulfurPool,
    Cactus,
    Bamboo,
    Flower,
    DesertWell,
    Coral,
    CoralCrust,
    Vegetation,
    FlowerBed,
    ForestFoliage,
    HugeMushroom,
    ForestRock,
    Shelf,
    Pale,
);
probed!(Fossil, ScatterFeature, Aggregate, Geode, Cave, LegacyTrees, SwampFoliage);
located!(Tree, Search, Sequence, OptionalBeehive);

impl PlacedFeature for Ore {
    fn run(&self, region: &mut Region, random: &mut MersenneTwister, pos: IVec3) -> Probed<bool> {
        Ok(self.place(&mut RegionCells(region), random, pos))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Debug)]
pub struct CoralHang(pub Coral);

impl PlacedFeature for CoralHang {
    fn run(&self, region: &mut Region, random: &mut MersenneTwister, pos: IVec3) -> Probed<bool> {
        Ok(self.0.hang(region, random, pos))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Debug)]
pub struct SeaAnemone(pub Coral);

impl PlacedFeature for SeaAnemone {
    fn run(&self, region: &mut Region, random: &mut MersenneTwister, pos: IVec3) -> Probed<bool> {
        Ok(self.0.anemone(region, random, pos))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl ScatterFeature {
    fn place(&self, region: &mut Region, random: &mut MersenneTwister, pos: IVec3) -> Probed<bool> {
        super::scatter::bind_world();
        let mut origin = pos;
        if self.project_to_floor {
            while origin.y > region.min_y() && super::world::is_air(region, origin - IVec3::Y) {
                origin.y -= 1;
            }
        }
        let variables = Variables {
            origin: origin.as_vec3().to_array(),
            world: [0.0; 3],
        };
        let mut any = false;
        let mut draws = Draws { positions: random, molang: None };
        self.scatter.try_run(&mut draws, variables, origin, region, |region, random, target| {
            any |= self.inner.place(region, random, target)?;
            Ok(())
        })?;
        super::scatter::leave_origin(origin);
        Ok(any)
    }
}

impl Aggregate {
    fn place(&self, region: &mut Region, random: &mut MersenneTwister, pos: IVec3) -> Probed<bool> {
        let mut any = false;
        for feature in &self.features {
            let placed = feature.place(region, random, pos)?;
            any |= placed;
            match self.early_out {
                EarlyOut::FirstFailure if !any => break,
                EarlyOut::FirstSuccess if placed => break,
                _ => {}
            }
        }
        Ok(any)
    }
}
