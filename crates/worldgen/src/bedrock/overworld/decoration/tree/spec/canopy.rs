use super::super::decoration::{CanopyDecoration, Decoration};
use super::super::draw::{Odds, Range};
use super::super::site::Log;
use super::UNSUPPORTED_SHAPE;
use super::parse::{block, float, log, number, required, weighted_blocks, weighted_values};
use crate::shared::blocks::{BlockId, BlockTable};
use serde_json::Value;

const DEFAULT_POPLAR_CANOPY_HEIGHT: Range = Range::new(5, 6);

const DEFAULT_SIDE_HOLE_CHANCE: f64 = 0.15;

const DEFAULT_POPLAR_RADIUS: i32 = 5;

const CANOPY_SUFFIX: &str = "canopy";

const DEFAULT_MEGA_RADIUS: i32 = 2;

const DEFAULT_CORE_WIDTH: i32 = 1;

const DEFAULT_RADIUS_STEP: f64 = 3.5;

const DEFAULT_RISE: i32 = 2;

const DEFAULT_RUN: i32 = 1;

const DEFAULT_MIN_WIDTH: i32 = 1;

#[derive(Debug)]
pub struct SimpleCanopy {
    pub leaf: BlockId,
    pub offset: (i32, i32),
    pub min_width: i32,
    pub rise: i32,
    pub run: i32,
    pub variation_chance: Vec<Odds>,
    pub decoration: Option<CanopyDecoration>,
}

#[derive(Debug)]
pub struct MangroveCanopy {
    pub height: Range,
    pub radius: Range,
    pub attempts: i32,
    pub leaves: Vec<(BlockId, i32)>,
    pub hanging: BlockId,
    pub hanging_chance: Odds,
    pub decoration: Option<Decoration>,
}

#[derive(Debug)]
pub struct CherryCanopy {
    pub leaf: BlockId,
    pub height: Range,
    pub radius: Range,
    pub trunk_width: i32,
    pub wide_bottom_hole: Odds,
    pub corner_hole: Odds,
    pub hanging_leaves: Odds,
    pub hanging_leaves_extension: Odds,
}

#[derive(Debug)]
pub struct PoplarCanopy {
    pub leaf: BlockId,
    pub branch: Log,
    pub radius: Vec<(i32, i32)>,
    pub height: Range,
    pub side_hole_chance: f32,
}

#[derive(Debug)]
pub enum Canopy {
    Simple(SimpleCanopy),
    Acacia {
        leaf: BlockId,
        size: i32,
        simplify: bool,
    },
    Pine {
        leaf: BlockId,
        height: Range,
        base_radius: i32,
    },
    Spruce {
        leaf: BlockId,
        lower_offset: Range,
        upper_offset: Range,
        max_radius: Range,
    },
    Fancy {
        leaf: BlockId,
        height: i32,
        radius: i32,
    },
    Mega {
        leaf: BlockId,
        height: Range,
        base_radius: i32,
        core_width: i32,
        simplify: bool,
    },
    MegaPine {
        leaf: BlockId,
        height: Range,
        base_radius: i32,
        radius_step_modifier: f32,
        core_width: i32,
    },
    Roofed {
        leaf: BlockId,
        height: i32,
        core_width: i32,
        outer_radius: i32,
        inner_radius: i32,
    },
    Mangrove(MangroveCanopy),
    Cherry(CherryCanopy),
    RandomSpread {
        leaves: Vec<(BlockId, i32)>,
        height: i32,
        radius: i32,
        attempts: i32,
    },
    Poplar(PoplarCanopy),
}

impl Canopy {
    pub fn parse(table: &mut BlockTable, body: &'static Value) -> Result<Option<Self>, &'static str> {
        let Some((key, canopy)) = body.as_object().and_then(|map| map.iter().find(|(key, _)| key.ends_with(CANOPY_SUFFIX))) else {
            return Ok(None);
        };
        let parsed = match key.as_str() {
            "canopy" => Self::parse_simple(table, canopy)?,
            "acacia_canopy" => Self::Acacia {
                leaf: leaf(table, canopy)?,
                size: required(canopy, "canopy_size")?,
                simplify: canopy.get("simplify_canopy").and_then(Value::as_bool).unwrap_or(false),
            },
            "pine_canopy" => Self::Pine {
                leaf: leaf(table, canopy)?,
                height: Range::parse(canopy.get("canopy_height"), Range::constant(0)),
                base_radius: required(canopy, "base_radius")?,
            },
            "spruce_canopy" => Self::Spruce {
                leaf: leaf(table, canopy)?,
                lower_offset: Range::parse(canopy.get("lower_offset"), Range::constant(0)),
                upper_offset: Range::parse(canopy.get("upper_offset"), Range::constant(0)),
                max_radius: Range::parse(canopy.get("max_radius"), Range::constant(0)),
            },
            "fancy_canopy" => Self::Fancy {
                leaf: leaf(table, canopy)?,
                height: required(canopy, "height")?,
                radius: required(canopy, "radius")?,
            },
            "mega_canopy" => Self::parse_mega(table, canopy)?,
            "mega_pine_canopy" => Self::parse_mega_pine(table, canopy)?,
            "roofed_canopy" => Self::Roofed {
                leaf: leaf(table, canopy)?,
                height: required(canopy, "canopy_height")?,
                core_width: required(canopy, "core_width")?,
                outer_radius: required(canopy, "outer_radius")?,
                inner_radius: required(canopy, "inner_radius")?,
            },
            "mangrove_canopy" => Self::parse_mangrove(table, canopy)?,
            "cherry_canopy" => Self::parse_cherry(table, canopy)?,
            "random_spread_canopy" => Self::RandomSpread {
                leaves: weighted_blocks(table, &canopy["leaf_blocks"])?,
                height: required(canopy, "canopy_height")?,
                radius: required(canopy, "canopy_radius")?,
                attempts: required(canopy, "leaf_placement_attempts")?,
            },
            "poplar_canopy" => Self::parse_poplar(table, canopy)?,
            _ => return Err(UNSUPPORTED_SHAPE),
        };
        Ok(Some(parsed))
    }

    fn parse_simple(table: &mut BlockTable, canopy: &'static Value) -> Result<Self, &'static str> {
        let slope = canopy.get("canopy_slope");
        let variation = match canopy.get("variation_chance") {
            Some(Value::Array(list)) => list.iter().map(|chance| Odds::parse(Some(chance))).collect(),
            Some(single) => vec![Odds::parse(Some(single))],
            None => Vec::new(),
        };
        let offset = &canopy["canopy_offset"];
        Ok(Self::Simple(SimpleCanopy {
            leaf: leaf(table, canopy)?,
            offset: (number(offset, "min", 0), number(offset, "max", 0)),
            min_width: number(canopy, "min_width", DEFAULT_MIN_WIDTH),
            rise: slope.map_or(DEFAULT_RISE, |slope| number(slope, "rise", DEFAULT_RISE)),
            run: slope.map_or(DEFAULT_RUN, |slope| number(slope, "run", DEFAULT_RUN)),
            variation_chance: variation,
            decoration: CanopyDecoration::parse(table, canopy.get("canopy_decoration"))?,
        }))
    }

    fn parse_mega(table: &mut BlockTable, canopy: &'static Value) -> Result<Self, &'static str> {
        Ok(Self::Mega {
            leaf: leaf(table, canopy)?,
            height: Range::parse(canopy.get("canopy_height"), Range::constant(0)),
            base_radius: number(canopy, "base_radius", DEFAULT_MEGA_RADIUS),
            core_width: number(canopy, "core_width", DEFAULT_CORE_WIDTH),
            simplify: canopy.get("simplify_canopy").and_then(Value::as_bool).unwrap_or(false),
        })
    }

    fn parse_mega_pine(table: &mut BlockTable, canopy: &'static Value) -> Result<Self, &'static str> {
        Ok(Self::MegaPine {
            leaf: leaf(table, canopy)?,
            height: Range::parse(canopy.get("canopy_height"), Range::constant(0)),
            base_radius: number(canopy, "base_radius", DEFAULT_MEGA_RADIUS),
            radius_step_modifier: float(canopy, "radius_step_modifier", DEFAULT_RADIUS_STEP),
            core_width: number(canopy, "core_width", DEFAULT_CORE_WIDTH),
        })
    }

    fn parse_mangrove(table: &mut BlockTable, canopy: &'static Value) -> Result<Self, &'static str> {
        Ok(Self::Mangrove(MangroveCanopy {
            height: Range::parse(canopy.get("canopy_height"), Range::constant(0)),
            radius: Range::parse(canopy.get("canopy_radius"), Range::constant(0)),
            attempts: required(canopy, "leaf_placement_attempts")?,
            leaves: weighted_blocks(table, &canopy["leaf_blocks"])?,
            hanging: block(table, &canopy["hanging_block"])?,
            hanging_chance: Odds::parse(canopy.get("hanging_block_placement_chance")),
            decoration: Decoration::parse(table, canopy.get("canopy_decoration"))?,
        }))
    }

    fn parse_cherry(table: &mut BlockTable, canopy: &'static Value) -> Result<Self, &'static str> {
        Ok(Self::Cherry(CherryCanopy {
            leaf: leaf(table, canopy)?,
            height: Range::parse(canopy.get("height"), Range::constant(0)),
            radius: Range::parse(canopy.get("radius"), Range::constant(0)),
            trunk_width: number(canopy, "trunk_width", 1),
            wide_bottom_hole: Odds::parse(canopy.get("wide_bottom_layer_hole_chance")),
            corner_hole: Odds::parse(canopy.get("corner_hole_chance")),
            hanging_leaves: Odds::parse(canopy.get("hanging_leaves_chance")),
            hanging_leaves_extension: Odds::parse(canopy.get("hanging_leaves_extension_chance")),
        }))
    }

    fn parse_poplar(table: &mut BlockTable, canopy: &'static Value) -> Result<Self, &'static str> {
        Ok(Self::Poplar(PoplarCanopy {
            leaf: leaf(table, canopy)?,
            branch: log(table, &canopy["branch_block"])?,
            radius: weighted_values(canopy.get("radius"), DEFAULT_POPLAR_RADIUS),
            height: Range::parse(canopy.get("height"), DEFAULT_POPLAR_CANOPY_HEIGHT),
            side_hole_chance: float(canopy, "side_hole_chance", DEFAULT_SIDE_HOLE_CHANCE),
        }))
    }
}

fn leaf(table: &mut BlockTable, canopy: &'static Value) -> Result<BlockId, &'static str> {
    block(table, &canopy["leaf_block"])
}
