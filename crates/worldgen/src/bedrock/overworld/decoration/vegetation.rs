use super::aquatic::{Kelp, Seagrass};
use super::jungle::{PodzolArea, SeaPickle, Vines};
use super::land::{Firefly, Mushroom, RandomWalkPlant, SpreadPlant, WaterEdgeColumn, Waterlily};
use super::pile::BlockPile;
use super::plants::Meadow;
use super::registry::FeatureRegistry;
use super::rules::arc;
use super::single_block::SingleBlock;
use crate::shared::chunk::region::Region;
use crate::shared::rng::mersenne::MersenneTwister;
use glam::IVec3;

const SEAGRASS_FEATURE: &str = "chorus:seagrass";
const KELP_FEATURE: &str = "chorus:kelp";
const SPREAD_PLANT_FEATURE: &str = "chorus:spread_plant";
const RANDOM_WALK_PLANT_FEATURE: &str = "chorus:random_walk_plant";
const WATER_EDGE_COLUMN_FEATURE: &str = "chorus:water_edge_column";
const MUSHROOM_FEATURE: &str = "chorus:mushroom";

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Vegetation {
    Kelp(Kelp),
    Seagrass(Seagrass),
    SpreadPlant(SpreadPlant),
    WaterEdgeColumn(WaterEdgeColumn),
    Waterlily(Waterlily),
    RandomWalkPlant(RandomWalkPlant),
    Mushroom(Mushroom),
    Firefly(Firefly),
    Meadow(Meadow),
    SingleBlock(SingleBlock),
    Vines(Vines),
    PodzolArea(PodzolArea),
    SeaPickle(SeaPickle),
    BlockPile(BlockPile),
}

impl Vegetation {
    pub fn register(registry: &mut FeatureRegistry) {
        registry.register(SEAGRASS_FEATURE, |loader, doc| {
            Seagrass::parse(loader.table(), doc.body).map(|seagrass| arc(Self::Seagrass(seagrass))).ok_or(doc.blocked())
        });
        registry.register(KELP_FEATURE, |loader, doc| Ok(arc(Self::Kelp(Kelp::parse(loader.table(), doc.body)))));
        registry.register_id("minecraft:waterlily_feature", |loader, _| Ok(arc(Self::Waterlily(Waterlily::new(loader.table())))));
        registry.register(SPREAD_PLANT_FEATURE, |loader, doc| {
            SpreadPlant::parse(loader.table(), doc.body).map(|plant| arc(Self::SpreadPlant(plant))).ok_or(doc.blocked())
        });
        registry.register(RANDOM_WALK_PLANT_FEATURE, |loader, doc| {
            RandomWalkPlant::parse(loader.table(), doc.body).map(|plant| arc(Self::RandomWalkPlant(plant))).ok_or(doc.blocked())
        });
        registry.register(WATER_EDGE_COLUMN_FEATURE, |loader, doc| {
            WaterEdgeColumn::parse(loader.table(), doc.body).map(|column| arc(Self::WaterEdgeColumn(column))).ok_or(doc.blocked())
        });
        registry.register(MUSHROOM_FEATURE, |loader, doc| {
            Mushroom::parse(loader.table(), doc.body).map(|mushroom| arc(Self::Mushroom(mushroom))).ok_or(doc.blocked())
        });
        registry.register_id("minecraft:firefly_bush_cluster_feature", |loader, _| Ok(arc(Self::Firefly(Firefly::new(loader.table(), false)))));
        registry.register_id("minecraft:firefly_bush_water_cluster_feature", |loader, _| Ok(arc(Self::Firefly(Firefly::new(loader.table(), true)))));
        registry.register_id("minecraft:meadow_flower_feature", |loader, _| Ok(arc(Self::Meadow(Meadow::new(loader.table())))));
        registry.register_id("minecraft:vines_feature", |loader, _| Ok(arc(Self::Vines(Vines::new(loader.table())))));
        registry.register_id("minecraft:podzol_area_feature", |loader, _| Ok(arc(Self::PodzolArea(PodzolArea::new(loader.table())))));
        registry.register_id("minecraft:pickle_feature", |loader, _| Ok(arc(Self::SeaPickle(SeaPickle::new(loader.table())))));
    }

    pub fn place(&self, region: &mut Region, random: &mut MersenneTwister, pos: IVec3) -> bool {
        match self {
            Self::Kelp(kelp) => kelp.place(region, random, pos),
            Self::Seagrass(seagrass) => seagrass.place(region, random, pos),
            Self::SpreadPlant(plant) => plant.place(region, random, pos),
            Self::WaterEdgeColumn(column) => column.place(region, random, pos),
            Self::Waterlily(lily) => lily.place(region, pos),
            Self::RandomWalkPlant(plant) => plant.place(region, random, pos),
            Self::Mushroom(mushroom) => mushroom.place(region, random, pos),
            Self::Firefly(firefly) => firefly.place(region, random, pos),
            Self::Meadow(meadow) => meadow.place(region, pos),
            Self::SingleBlock(single) => single.place(region, random, pos),
            Self::Vines(vines) => vines.place(region, random, pos),
            Self::PodzolArea(area) => area.place(region, random, pos),
            Self::SeaPickle(pickle) => pickle.place(region, random, pos),
            Self::BlockPile(pile) => pile.place(region, random, pos),
        }
    }
}
