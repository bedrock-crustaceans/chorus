use super::super::decoration::Decoration;
use super::super::draw::{Odds, Range};
use super::super::site::Log;
use super::UNSUPPORTED_SHAPE;
use super::canopy::Canopy;
use super::parse::{block, float, intervals, log, number, required};
use crate::bedrock::overworld::decoration::placed::Placed;
use crate::bedrock::overworld::decoration::rules::FeatureLoader;
use crate::shared::blocks::{BlockId, BlockTable};
use serde_json::Value;
use std::sync::Arc;

const TRUNK_KEYS: [&str; 8] = ["trunk", "acacia_trunk", "fancy_trunk", "mega_trunk", "mangrove_trunk", "cherry_trunk", "fallen_trunk", "poplar_trunk"];

const DEFAULT_POPLAR_TRUNK_HEIGHT: Range = Range::new(7, 11);

const DEFAULT_POPLAR_ABOVE_BRANCHES: Range = Range::new(4, 4);

const DEFAULT_POPLAR_BRANCHES: Range = Range::new(1, 4);

const DEFAULT_ACACIA_BRANCH_LENGTH: Range = Range::new(1, 5);

const DEFAULT_ACACIA_BRANCH_POSITION: Range = Range::new(1, 3);

const DEFAULT_MIN_HEIGHT_FOR_CANOPY: i32 = 3;

const UNBOUNDED_DEPTH: i32 = 255;

#[derive(Debug)]
pub struct SimpleTrunk {
    pub block: BlockId,
    pub height: Range,
    pub height_modifier: Range,
    pub max_depth: i32,
    pub decoration: Option<Decoration>,
}

#[derive(Debug)]
pub struct AcaciaBranches {
    pub length: Range,
    pub position: Range,
    pub chance: Odds,
    pub canopy: Option<Canopy>,
}

#[derive(Debug)]
pub struct AcaciaTrunk {
    pub block: BlockId,
    pub width: i32,
    pub height_base: i32,
    pub height_intervals: Vec<i32>,
    pub min_height_for_canopy: i32,
    pub lean_height: Range,
    pub lean_steps: Range,
    pub lean_length: Range,
    pub allow_diagonal: bool,
    pub decoration: Option<Decoration>,
    pub branches: AcaciaBranches,
}

#[derive(Debug)]
pub struct FancyTrunk {
    pub log: Log,
    pub width: i32,
    pub height_base: i32,
    pub height_variance: i32,
    pub height_scale: f32,
    pub branch_slope: f32,
    pub branch_density: f32,
    pub min_altitude_factor: f32,
    pub foliage_altitude_factor: f32,
    pub width_scale: f32,
}

#[derive(Debug)]
pub struct MegaBranches {
    pub length: i32,
    pub slope: f32,
    pub interval: Range,
    pub altitude_factor: (f32, f32),
    pub canopy: Option<Canopy>,
}

#[derive(Debug)]
pub struct MegaTrunk {
    pub block: BlockId,
    pub width: i32,
    pub height_base: i32,
    pub height_intervals: Vec<i32>,
    pub decoration: Option<Decoration>,
    pub branches: Option<MegaBranches>,
}

#[derive(Debug)]
pub struct MangroveTrunk {
    pub block: BlockId,
    pub height_base: i32,
    pub height_rand_a: i32,
    pub height_rand_b: i32,
    pub branch_length: Range,
    pub branch_steps: Range,
    pub decoration: Option<Decoration>,
}

#[derive(Debug)]
pub struct CherryTrunk {
    pub log: Log,
    pub height_base: i32,
    pub height_intervals: Vec<i32>,
    pub weights: [i32; 3],
    pub horizontal_length: Range,
    pub start_offset: Range,
    pub end_offset: Range,
}

#[derive(Debug)]
pub struct FallenTrunk {
    pub log: Log,
    pub log_length: Range,
    pub height_modifier: Range,
    pub stump_height: Range,
    pub log_decoration: Option<Arc<Placed>>,
    pub decoration: Option<Decoration>,
}

#[derive(Debug)]
pub struct PoplarTrunk {
    pub log: Log,
    pub height: Range,
    pub above_branches: Range,
    pub branches: Range,
    pub log_decoration: Option<Arc<Placed>>,
}

#[derive(Debug)]
pub enum Trunk {
    Simple(SimpleTrunk),
    Acacia(AcaciaTrunk),
    Fancy(FancyTrunk),
    Mega(MegaTrunk),
    Mangrove(MangroveTrunk),
    Cherry(CherryTrunk),
    Fallen(FallenTrunk),
    Poplar(PoplarTrunk),
}

impl Trunk {
    pub fn parse(loader: &mut FeatureLoader, depth: usize, body: &'static Value) -> Result<Self, &'static str> {
        let key = TRUNK_KEYS.iter().find(|key| body.get(**key).is_some()).ok_or(UNSUPPORTED_SHAPE)?;
        let trunk = &body[*key];
        let decoration = Decoration::parse(loader.table(), trunk.get("trunk_decoration"))?;
        let log_decoration = match trunk.get("log_decoration_feature").and_then(Value::as_str) {
            Some(name) => Some(loader.feature(name, depth + 1).map_err(|blocker| blocker.kind)?),
            None => None,
        };
        let table = loader.table();
        match *key {
            "trunk" => Self::parse_simple(table, trunk, decoration),
            "acacia_trunk" => Self::parse_acacia(table, trunk, decoration),
            "fancy_trunk" => Self::parse_fancy(table, trunk),
            "mega_trunk" => Self::parse_mega(table, trunk, decoration),
            "mangrove_trunk" => Self::parse_mangrove(table, trunk, decoration),
            "cherry_trunk" => Self::parse_cherry(table, trunk),
            "fallen_trunk" => Ok(Self::Fallen(FallenTrunk {
                log: log(table, &trunk["trunk_block"])?,
                log_length: Range::parse(trunk.get("log_length"), Range::constant(1)),
                height_modifier: Range::parse(trunk.get("height_modifier"), Range::constant(0)),
                stump_height: Range::parse(trunk.get("stump_height"), Range::constant(1)),
                log_decoration,
                decoration,
            })),
            "poplar_trunk" => Ok(Self::Poplar(PoplarTrunk {
                log: log(table, &trunk["trunk_block"])?,
                height: Range::parse(trunk.get("trunk_height"), DEFAULT_POPLAR_TRUNK_HEIGHT),
                above_branches: Range::parse(trunk.get("remaining_trunk_height_above_branches"), DEFAULT_POPLAR_ABOVE_BRANCHES),
                branches: Range::parse(trunk.get("amount_of_foliage_support_branches"), DEFAULT_POPLAR_BRANCHES),
                log_decoration,
            })),
            _ => Err(UNSUPPORTED_SHAPE),
        }
    }

    fn parse_simple(table: &mut BlockTable, trunk: &'static Value, decoration: Option<Decoration>) -> Result<Self, &'static str> {
        let max_depth = match trunk.get("can_be_submerged") {
            Some(Value::Bool(true)) => UNBOUNDED_DEPTH,
            Some(found @ Value::Object(_)) => number(found, "max_depth", 0),
            _ => 0,
        };
        Ok(Self::Simple(SimpleTrunk {
            block: block(table, &trunk["trunk_block"])?,
            height: Range::parse(trunk.get("trunk_height"), Range::constant(1)),
            height_modifier: Range::parse(trunk.get("height_modifier"), Range::constant(0)),
            max_depth,
            decoration,
        }))
    }

    fn parse_acacia(table: &mut BlockTable, trunk: &'static Value, decoration: Option<Decoration>) -> Result<Self, &'static str> {
        let lean = &trunk["trunk_lean"];
        let height = &trunk["trunk_height"];
        let branches = match trunk.get("branches") {
            None => AcaciaBranches {
                length: DEFAULT_ACACIA_BRANCH_LENGTH,
                position: DEFAULT_ACACIA_BRANCH_POSITION,
                chance: Odds::NEVER,
                canopy: None,
            },
            Some(branches) => AcaciaBranches {
                length: Range::parse(branches.get("branch_length"), DEFAULT_ACACIA_BRANCH_LENGTH),
                position: Range::parse(branches.get("branch_position"), DEFAULT_ACACIA_BRANCH_POSITION),
                chance: Odds::parse(branches.get("branch_chance")),
                canopy: Canopy::parse(table, &branches["branch_canopy"])?,
            },
        };
        Ok(Self::Acacia(AcaciaTrunk {
            block: block(table, &trunk["trunk_block"])?,
            width: required(trunk, "trunk_width")?,
            height_base: required(height, "base")?,
            height_intervals: intervals(height),
            min_height_for_canopy: number(height, "min_height_for_canopy", DEFAULT_MIN_HEIGHT_FOR_CANOPY),
            lean_height: Range::parse(lean.get("lean_height"), Range::constant(0)),
            lean_steps: Range::parse(lean.get("lean_steps"), Range::constant(0)),
            lean_length: Range::parse(lean.get("lean_length"), Range::constant(0)),
            allow_diagonal: lean.get("allow_diagonal_growth").and_then(Value::as_bool).unwrap_or(true),
            decoration,
            branches,
        }))
    }

    fn parse_fancy(table: &mut BlockTable, trunk: &'static Value) -> Result<Self, &'static str> {
        let height = &trunk["trunk_height"];
        let branches = &trunk["branches"];
        Ok(Self::Fancy(FancyTrunk {
            log: log(table, &trunk["trunk_block"])?,
            width: required(trunk, "trunk_width")?,
            height_base: required(height, "base")?,
            height_variance: required(height, "variance")?,
            height_scale: float(height, "scale", 0.0),
            branch_slope: float(branches, "slope", 0.0),
            branch_density: float(branches, "density", 0.0),
            min_altitude_factor: float(branches, "min_altitude_factor", 0.0),
            foliage_altitude_factor: float(trunk, "foliage_altitude_factor", 0.0),
            width_scale: float(trunk, "width_scale", 0.0),
        }))
    }

    fn parse_mega(table: &mut BlockTable, trunk: &'static Value, decoration: Option<Decoration>) -> Result<Self, &'static str> {
        let height = &trunk["trunk_height"];
        let branches = match trunk.get("branches") {
            None => None,
            Some(branches) => Some(MegaBranches {
                length: required(branches, "branch_length")?,
                slope: float(branches, "branch_slope", 0.0),
                interval: Range::parse(branches.get("branch_interval"), Range::constant(1)),
                altitude_factor: (float(&branches["branch_altitude_factor"], "min", 0.0), float(&branches["branch_altitude_factor"], "max", 0.0)),
                canopy: Canopy::parse(table, &branches["branch_canopy"])?,
            }),
        };
        Ok(Self::Mega(MegaTrunk {
            block: block(table, &trunk["trunk_block"])?,
            width: required(trunk, "trunk_width")?,
            height_base: required(height, "base")?,
            height_intervals: intervals(height),
            decoration,
            branches,
        }))
    }

    fn parse_mangrove(table: &mut BlockTable, trunk: &'static Value, decoration: Option<Decoration>) -> Result<Self, &'static str> {
        let height = &trunk["trunk_height"];
        let branches = trunk.get("branches").unwrap_or(&Value::Null);
        Ok(Self::Mangrove(MangroveTrunk {
            block: block(table, &trunk["trunk_block"])?,
            height_base: required(height, "base")?,
            height_rand_a: required(height, "height_rand_a")?,
            height_rand_b: required(height, "height_rand_b")?,
            branch_length: Range::parse(branches.get("branch_length"), Range::constant(0)),
            branch_steps: Range::parse(branches.get("branch_steps"), Range::constant(0)),
            decoration,
        }))
    }

    fn parse_cherry(table: &mut BlockTable, trunk: &'static Value) -> Result<Self, &'static str> {
        let height = &trunk["trunk_height"];
        let branches = &trunk["branches"];
        let weights = &branches["tree_type_weights"];
        Ok(Self::Cherry(CherryTrunk {
            log: log(table, &trunk["trunk_block"])?,
            height_base: required(height, "base")?,
            height_intervals: intervals(height),
            weights: [required(weights, "one_branch")?, required(weights, "two_branches")?, required(weights, "two_branches_and_trunk")?],
            horizontal_length: Range::parse(branches.get("branch_horizontal_length"), Range::constant(0)),
            start_offset: Range::parse(branches.get("branch_start_offset_from_top"), Range::constant(0)),
            end_offset: Range::parse(branches.get("branch_end_offset_from_top"), Range::constant(0)),
        }))
    }
}
