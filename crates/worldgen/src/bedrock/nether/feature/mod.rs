use super::decorate::View;
use crate::bedrock::overworld::expression::Variables;
use crate::bedrock::overworld::ore::{Ore, resolve_block};
use crate::bedrock::overworld::scatter::{Draws, Scatter};
use crate::shared::blocks::{AIR, BlockId, BlockTable};
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::{PaletteBlocks, TemplateLibrary};
use glam::IVec3;
use serde_json::Value;

mod basalt;
mod fossil;
mod fungus;
mod vines;

use basalt::{columns, delta, pillar};
use fossil::fossil;
use fungus::fungus;
use vines::{twisting_cluster, weeping_cluster};

pub struct Materials<'a> {
    pub templates: &'a TemplateLibrary,
    pub palette: &'a PaletteBlocks,
    pub air: BlockId,
    pub ground: BlockId,
    pub ghast_ground: BlockId,
}

#[derive(Debug)]
pub struct Plant {
    pub block: BlockId,
    pub may_replace: Vec<u16>,
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Feature {
    Scatter { scatter: Scatter, inner: Box<Feature> },
    Ore(Ore),
    Plant(Plant),
    Columns,
    Pillar,
    Delta,
    Fungus { crimson: bool },
    WeepingVines,
    TwistingVines,
    Fossil,
    Nothing,
}

pub fn plant(table: &mut BlockTable, body: &'static Value) -> Option<Plant> {
    let block = resolve_block(table, &body["places_block"])?;
    let may_replace = body["may_replace"].as_array()?.iter().map(|entry| Some(table.kind(plant_name(entry)?))).collect::<Option<Vec<_>>>()?;
    Some(Plant { block, may_replace })
}

fn plant_name(value: &Value) -> Option<&str> {
    match value {
        Value::String(name) => Some(name.as_str()),
        other => other.get("name")?.as_str(),
    }
}

fn nether_soil(view: &View, block: BlockId) -> bool {
    let ids = view.ids();
    block == ids.crimson_nylium
        || block == ids.warped_nylium
        || block == ids.soul_soil
        || block == ids.dirt
        || block == ids.coarse_dirt
        || block == ids.grass_block
        || block == ids.mycelium
        || block == ids.podzol
}

fn open(view: &View, block: BlockId) -> bool {
    view.blocks().is_air(block) || view.blocks().entry(block).liquid
}

fn placeable(view: &View, [x, y, z]: [i32; 3]) -> bool {
    open(view, view.get(x, y, z)) && nether_soil(view, view.get(x, y - 1, z))
}

fn buildable(view: &View, block: BlockId) -> bool {
    let ids = view.ids();
    block == AIR || block == ids.crimson_roots || block == ids.warped_roots || block == ids.nether_sprouts || block == ids.fire || block == ids.soul_fire || view.blocks().entry(block).liquid
}

pub fn place(feature: &Feature, view: &mut View, random: &mut MersenneTwister, at: IVec3, materials: Option<&Materials>) -> bool {
    let at = [at.x, at.y, at.z];
    match feature {
        Feature::Scatter { scatter, inner } => place_scatter(scatter, inner, view, random, at, materials),
        Feature::Ore(ore) => ore.place(view, random, IVec3::from_array(at)),
        Feature::Plant(plant) => place_plant(plant, view, random, at),
        Feature::Columns => columns(view, random, at),
        Feature::Pillar => pillar(view, random, at),
        Feature::Delta => delta(view, random, at),
        Feature::Fungus { crimson } => fungus(view, random, at, *crimson),
        Feature::WeepingVines => weeping_cluster(view, random, at),
        Feature::TwistingVines => twisting_cluster(view, random, at),
        Feature::Fossil => materials.is_some_and(|materials| fossil(view, random, at, materials)),
        Feature::Nothing => false,
    }
}

fn place_scatter(scatter: &Scatter, inner: &Feature, view: &mut View, random: &mut MersenneTwister, at: [i32; 3], materials: Option<&Materials>) -> bool {
    let mut placed = false;
    let mut draws = Draws { positions: random, molang: None };
    scatter.run(&mut draws, Variables::default(), IVec3::from_array(at), view, |view, random, position| {
        placed |= place(inner, view, random, position, materials);
    });
    placed
}

fn place_plant(plant: &Plant, view: &mut View, random: &mut MersenneTwister, at: [i32; 3]) -> bool {
    random.next_int_bounded(1);
    if !placeable(view, at) {
        return false;
    }
    let kind = view.blocks().entry(view.get(at[0], at[1], at[2])).kind;
    if !plant.may_replace.is_empty() && !plant.may_replace.contains(&kind) {
        return false;
    }
    view.set(at[0], at[1], at[2], plant.block)
}
