use super::draw::{Odds, Range};
use super::site::{Foliage, SIDES, block, put};
use crate::bedrock::overworld::decoration::ore::Draw;
use crate::shared::blocks::{BlockId, BlockTable};
use crate::shared::chunk::region::Region;
use chorus_block::block_id::VINE;
use glam::IVec3;
use serde_json::Value;

const BLOCK_KEY: &str = "decoration_block";
const SEQUENCE_KEY: &str = "decoration_blocks_sequence";
const CHANCE_KEY: &str = "decoration_chance";
const STEPS_KEY: &str = "num_steps";
const DIRECTION_KEY: &str = "step_direction";
const COUNT_KEY: &str = "count";
const SEQUENCE_BLOCK_KEY: &str = "block";
const NAME_KEY: &str = "name";
pub const UNSUPPORTED_DECORATION: &str = "unsupported tree decoration";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decorative {
    Vine,
    Block(BlockId),
}

impl Decorative {
    pub fn parse(table: &mut BlockTable, value: &'static Value) -> Result<Self, &'static str> {
        let name = value.as_str().or_else(|| value.get(NAME_KEY).and_then(Value::as_str)).ok_or(UNSUPPORTED_DECORATION)?;
        Ok(if name == VINE { Self::Vine } else { Self::Block(table.name(name)) })
    }

    fn block(self, foliage: &Foliage, side: usize) -> BlockId {
        match self {
            Self::Vine => foliage.vine_on(side),
            Self::Block(id) => id,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Down,
    Up,
    Outward,
}

#[derive(Debug)]
pub struct Decoration {
    entries: Vec<(Decorative, Range)>,
    chance: Odds,
    step: Step,
}

impl Decoration {
    pub fn parse(table: &mut BlockTable, value: Option<&'static Value>) -> Result<Option<Self>, &'static str> {
        let Some(value) = value else { return Ok(None) };
        let entries = match value.get(SEQUENCE_KEY).and_then(Value::as_array) {
            Some(sequence) => sequence
                .iter()
                .map(|entry| Ok((Decorative::parse(table, &entry[SEQUENCE_BLOCK_KEY])?, Range::parse(entry.get(COUNT_KEY), Range::constant(1)))))
                .collect::<Result<Vec<_>, &'static str>>()?,
            None => {
                let steps = value.get(STEPS_KEY).and_then(Value::as_i64).unwrap_or_default() as i32 + 1;
                vec![(Decorative::parse(table, &value[BLOCK_KEY])?, Range::constant(steps))]
            }
        };
        let step = match value.get(DIRECTION_KEY).and_then(Value::as_str) {
            Some("up") => Step::Up,
            Some("outward") => Step::Outward,
            _ => Step::Down,
        };
        Ok(Some(Self {
            entries,
            chance: Odds::parse(value.get(CHANCE_KEY)),
            step,
        }))
    }

    pub fn place<D: Draw>(&self, region: &mut Region, foliage: &Foliage, random: &mut D, pos: IVec3, enabled: [bool; 4]) {
        for (index, &side) in SIDES.iter().enumerate() {
            if !enabled[index] || !self.chance.roll(random) {
                continue;
            }
            let mut at = pos + side;
            if !region.blocks.is_air(block(region, at)) {
                continue;
            }
            let step = match self.step {
                Step::Outward => side,
                Step::Up => IVec3::Y,
                Step::Down => IVec3::NEG_Y,
            };
            for &(decorative, count) in &self.entries {
                let count = count.draw_inclusive(random);
                if !region.blocks.is_air(block(region, at)) || count < 1 {
                    continue;
                }
                for done in 0..count {
                    put(region, at, decorative.block(foliage, index));
                    at += step;
                    if done + 1 < count && !region.blocks.is_air(block(region, at)) {
                        break;
                    }
                }
            }
        }
    }

    pub fn place_decorated<D: Draw>(decoration: Option<&Self>, region: &mut Region, foliage: &Foliage, random: &mut D, pos: IVec3, value: BlockId) -> bool {
        let placed = put(region, pos, value);
        if let (true, Some(decoration)) = (placed, decoration) {
            decoration.place(region, foliage, random, pos, [true; 4]);
        }
        placed
    }
}

#[derive(Debug)]
pub struct CanopyDecoration {
    block: Decorative,
    chance: Odds,
    steps: Range,
}

impl CanopyDecoration {
    pub fn parse(table: &mut BlockTable, value: Option<&'static Value>) -> Result<Option<Self>, &'static str> {
        let Some(value) = value else { return Ok(None) };
        Ok(Some(Self {
            block: Decorative::parse(table, &value[BLOCK_KEY])?,
            chance: Odds::parse(value.get(CHANCE_KEY)),
            steps: Range::parse(value.get(STEPS_KEY), Range::constant(0)),
        }))
    }

    pub fn place<D: Draw>(&self, region: &mut Region, foliage: &Foliage, random: &mut D, pos: IVec3) {
        for (index, &side) in SIDES.iter().enumerate() {
            if !self.chance.roll(random) {
                continue;
            }
            let mut at = pos + side;
            if !region.blocks.is_air(block(region, at)) {
                continue;
            }
            let steps = self.steps.draw_inclusive(random) + 1;
            for _ in 0..steps {
                if !region.blocks.is_air(block(region, at)) {
                    break;
                }
                put(region, at, self.block.block(foliage, index));
                at += IVec3::NEG_Y;
            }
        }
    }
}
