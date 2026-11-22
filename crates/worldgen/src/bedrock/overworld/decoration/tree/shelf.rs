use super::site::block;
use super::spec::MALFORMED;
use crate::bedrock::overworld::decoration::ore::Draw;
use crate::shared::blocks::{BlockId, BlockTable, int, state, text};
use crate::shared::chunk::region::Region;
use crate::shared::rng::mersenne::MersenneTwister;
use chorus_block::state::common::{GROWTH_8, MINECRAFT_CARDINAL_DIRECTION, PILLAR_AXIS};
use glam::IVec3;
use serde_json::Value;

const FACINGS: [IVec3; 4] = [IVec3::new(0, 0, -1), IVec3::new(0, 0, 1), IVec3::new(-1, 0, 0), IVec3::new(1, 0, 0)];
const DIRECTIONS: [&str; 4] = ["north", "south", "west", "east"];
const GROWTHS: [i32; 2] = [0, 1];
const AROUND: [IVec3; 6] = [IVec3::Y, IVec3::NEG_Y, IVec3::NEG_Z, IVec3::Z, IVec3::NEG_X, IVec3::X];
const BLOCK_KEY: &str = "places_block";
const BARK_KEY: &str = "bark_side_only";
const ADJACENT_KEY: &str = "allow_adjacent";
const AXIS_X: &str = "x";
const AXIS_Z: &str = "z";
const AXIS_Y: &str = "y";

#[derive(Debug)]
pub struct Shelf {
    blocks: [[BlockId; 2]; 4],
    bark_side_only: bool,
    allow_adjacent: bool,
}

impl Shelf {
    pub fn parse(table: &mut BlockTable, body: &'static Value) -> Result<Self, &'static str> {
        let name = body.get(BLOCK_KEY).and_then(Value::as_str).ok_or(MALFORMED)?;
        let blocks = DIRECTIONS.map(|direction| GROWTHS.map(|growth| table.get(name, &[state(&GROWTH_8, int(growth)), state(&MINECRAFT_CARDINAL_DIRECTION, text(direction))])));
        Ok(Self {
            blocks,
            bark_side_only: body.get(BARK_KEY).and_then(Value::as_bool).unwrap_or(false),
            allow_adjacent: body.get(ADJACENT_KEY).and_then(Value::as_bool).unwrap_or(false),
        })
    }

    fn holds(&self, region: &Region, pos: IVec3) -> bool {
        let found = block(region, pos);
        self.blocks.iter().flatten().any(|&variant| variant == found)
    }

    pub fn place(&self, region: &mut Region, random: &mut MersenneTwister, pos: IVec3) -> bool {
        let side = random.below(FACINGS.len() as i32) as usize;
        let offset = FACINGS[side];
        let target = pos + offset;
        if !region.blocks.is_air(block(region, target)) {
            return false;
        }
        if self.bark_side_only {
            let bark = match region.blocks.text(block(region, pos), &PILLAR_AXIS) {
                Some(AXIS_X) => offset.x == 0,
                Some(AXIS_Z) => offset.z == 0,
                Some(AXIS_Y) => true,
                _ => false,
            };
            if !bark {
                return false;
            }
        }
        if !self.allow_adjacent && (FACINGS.iter().any(|&side| self.holds(region, pos + side)) || AROUND.iter().any(|&side| self.holds(region, target + side))) {
            return false;
        }
        let growth = random.below(GROWTHS.len() as i32) as usize;
        region.set(target, self.blocks[side][growth])
    }
}
