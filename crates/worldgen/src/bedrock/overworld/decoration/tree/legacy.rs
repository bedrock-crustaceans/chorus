use super::site::ground;
use crate::bedrock::overworld::decoration::placed::Placed;
use crate::bedrock::overworld::decoration::scatter::bind_world;
use crate::shared::chunk::region::Region;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::Probed;
use glam::IVec3;
use std::sync::Arc;

const COUNT_ROLL_BOUND: i32 = 10;
const COUNT_FLOOR: i32 = 7;
const COLUMN_SPAN: i32 = 14;
const COLUMN_INSET: i32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Picker {
    First,
    OneInThree,
    FourInFive,
    BirchMutated,
    ExtremeHills,
    Forest,
    Meadow,
    Jungle,
    JungleEdge,
    BambooJungle,
    MegaTaiga,
    RedwoodMutated,
}

#[derive(Debug)]
pub struct LegacyTrees {
    pub count: f32,
    pub picker: Picker,
    pub features: Vec<Arc<Placed>>,
}

fn one_in(random: &mut MersenneTwister, bound: i32) -> bool {
    random.next_int_bounded(bound) == 0
}

impl Picker {
    fn pick(self, random: &mut MersenneTwister) -> usize {
        match self {
            Self::First => 0,
            Self::OneInThree => usize::from(!one_in(random, 3)),
            Self::FourInFive => usize::from(random.next_int_bounded(5) <= 0),
            Self::BirchMutated => {
                if one_in(random, 5) {
                    0
                } else {
                    usize::from(random.next_boolean())
                }
            }
            Self::ExtremeHills => {
                if random.next_int_bounded(3) > 0 {
                    0
                } else if one_in(random, 10) {
                    1
                } else {
                    2
                }
            }
            Self::Forest => {
                if one_in(random, 5) {
                    0
                } else if one_in(random, 10) {
                    1
                } else {
                    2
                }
            }
            Self::Meadow => usize::from(!random.next_boolean()),
            Self::Jungle => {
                if one_in(random, 10) {
                    0
                } else if one_in(random, 2) {
                    1
                } else if one_in(random, 3) {
                    2
                } else {
                    3
                }
            }
            Self::JungleEdge => {
                if one_in(random, 10) {
                    0
                } else if one_in(random, 2) {
                    1
                } else {
                    2
                }
            }
            Self::BambooJungle => {
                if random.next_int_bounded(20) == 0 {
                    0
                } else if random.next_int_bounded(20) < 3 {
                    1
                } else if random.next_int_bounded(10) < 7 {
                    2
                } else {
                    3
                }
            }
            Self::MegaTaiga => {
                if one_in(random, 3) {
                    usize::from(!one_in(random, 13))
                } else if one_in(random, 3) {
                    2
                } else {
                    3
                }
            }
            Self::RedwoodMutated => {
                if one_in(random, 3) {
                    0
                } else if one_in(random, 3) {
                    1
                } else {
                    2
                }
            }
        }
    }
}

impl LegacyTrees {
    pub(super) fn tree_count(&self, random: &mut MersenneTwister) -> i32 {
        if one_in(random, COUNT_ROLL_BOUND) {
            (self.count * random.next_float()) as i32
        } else if self.count >= 1.0 {
            (self.count + random.next_int_bounded(COUNT_ROLL_BOUND).max(COUNT_FLOOR) as f32) as i32 - COUNT_FLOOR
        } else {
            i32::from(random.next_float() < self.count)
        }
    }

    pub fn place(&self, region: &mut Region, random: &mut MersenneTwister, at: IVec3) -> Probed<bool> {
        let count = self.tree_count(random);
        place_trees(region, random, at, count, |random| &self.features[self.picker.pick(random)])
    }
}

pub fn place_trees<'a>(region: &mut Region, random: &mut MersenneTwister, at: IVec3, count: i32, mut pick: impl FnMut(&mut MersenneTwister) -> &'a Arc<Placed>) -> Probed<bool> {
    for _ in 0..count {
        let feature = pick(random);
        let z = at.z + random.next_int_bounded(COLUMN_SPAN) + COLUMN_INSET;
        let x = at.x + random.next_int_bounded(COLUMN_SPAN) + COLUMN_INSET;
        let pos = IVec3::new(x, ground(region, x, z), z);
        feature.place(region, random, pos)?;
        bind_world();
    }
    Ok(true)
}

const SWAMP_SEAGRASS_TRIES: i32 = 10;
const SWAMP_SEAGRASS_LIFT: i32 = 54;
const SWAMP_CHUNK_SPAN: i32 = 16;
const SWAMP_MUSHROOM_BOUND: i32 = 20;
const SWAMP_COUNT_SHIFT: i32 = 5;

#[derive(Debug)]
pub struct SwampFoliage {
    pub seagrass: Arc<Placed>,
    pub tree: Arc<Placed>,
    pub mushroom: Arc<Placed>,
}

impl SwampFoliage {
    pub fn place(&self, region: &mut Region, random: &mut MersenneTwister, at: IVec3) -> Probed<bool> {
        let count = if one_in(random, COUNT_ROLL_BOUND) {
            let value = random.next_float();
            (value + value) as i32
        } else {
            random.next_int_bounded(COUNT_ROLL_BOUND).max(COUNT_FLOOR) - SWAMP_COUNT_SHIFT
        };
        for _ in 0..SWAMP_SEAGRASS_TRIES {
            let dx = random.next_int_bounded(SWAMP_CHUNK_SPAN);
            let dz = random.next_int_bounded(SWAMP_CHUNK_SPAN);
            self.seagrass.place(region, random, at + IVec3::new(dx, SWAMP_SEAGRASS_LIFT, dz))?;
        }
        place_trees(region, random, at, count, |random| if one_in(random, SWAMP_MUSHROOM_BOUND) { &self.mushroom } else { &self.tree })
    }
}
