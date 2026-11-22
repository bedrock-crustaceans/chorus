use super::site::{is_air, put};
use crate::shared::blocks::{BlockId, BlockTable, int, state, text};
use crate::shared::chunk::region::Region;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::survival::can_survive;
use crate::shared::tags::Tag;
use chorus_block::block_id::{LEAF_LITTER, PINK_PETALS, WILDFLOWERS};
use chorus_block::state::common::{GROWTH_8, MINECRAFT_CARDINAL_DIRECTION};
use glam::IVec3;

pub const FEATURES: [(&str, &str); 3] = [
    ("minecraft:leaf_litter_feature", LEAF_LITTER),
    ("minecraft:pink_petals_feature", PINK_PETALS),
    ("minecraft:wildflowers_feature", WILDFLOWERS),
];
const GROWTH_STAGES: i32 = 4;
const FACINGS: [&str; 4] = chorus_block::state::common::CARDINAL_DIRECTIONS;

#[derive(Debug)]
pub struct FlowerBed {
    variants: Vec<BlockId>,
    rejects_leaves_below: bool,
}

impl FlowerBed {
    pub fn new(table: &mut BlockTable, name: &'static str) -> Self {
        let variants = FACINGS
            .iter()
            .flat_map(|&facing| (0..GROWTH_STAGES).map(move |growth| (facing, growth)))
            .map(|(facing, growth)| table.get(name, &[state(&GROWTH_8, int(growth)), state(&MINECRAFT_CARDINAL_DIRECTION, text(facing))]))
            .collect();
        Self {
            variants,
            rejects_leaves_below: name == LEAF_LITTER,
        }
    }

    pub fn place(&self, region: &mut Region, random: &mut MersenneTwister, pos: IVec3) -> bool {
        let growth = random.next_int_bounded(GROWTH_STAGES);
        let facing = random.next_int_bounded(FACINGS.len() as i32);
        let value = self.variants[(facing * GROWTH_STAGES + growth) as usize];
        is_air(region, pos) && can_survive(region, value, pos) && !(self.rejects_leaves_below && region.blocks.is(region.get(pos - IVec3::Y), Tag::Leaves)) && put(region, pos, value)
    }
}
