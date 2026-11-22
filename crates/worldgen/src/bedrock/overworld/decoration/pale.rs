use super::registry::FeatureRegistry;
use super::rules::arc;
use super::world::{is_air, read};
use crate::shared::blocks::{BlockId, BlockTable, flag, state, text};
use crate::shared::chunk::region::Region;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::survival::can_survive;
use crate::shared::tags::Tag;
use chorus_block::block_id::{CLOSED_EYEBLOSSOM, PALE_MOSS_CARPET, SHORT_GRASS, TALL_GRASS};
use chorus_block::state::common::{PALE_MOSS_CARPET_SIDE_EAST, PALE_MOSS_CARPET_SIDE_NORTH, PALE_MOSS_CARPET_SIDE_SOUTH, PALE_MOSS_CARPET_SIDE_WEST, UPPER_BLOCK_BIT};
use glam::IVec3;

const MOSS_VEGETATION: &str = "minecraft:pale_moss_vegetation_feature";
const EYEBLOSSOM: &str = "minecraft:eyeblossom_feature";
const CARPET_WEIGHT: i32 = 25;
const SHORT_GRASS_WEIGHT: i32 = 25;
const TALL_GRASS_WEIGHT: i32 = 10;
const SIDE_VALUES: [&str; 3] = ["none", "short", "tall"];
const TOP_VALUES: [&str; 2] = ["none", "short"];
const CARPET_SIDES: i32 = 3;
const CARPET_TOP_SIDES: i32 = 2;
const NORTH_STRIDE: usize = 27;
const EAST_STRIDE: usize = 9;
const SOUTH_STRIDE: usize = 3;
const TOP_NORTH_STRIDE: usize = 8;
const TOP_EAST_STRIDE: usize = 4;
const TOP_SOUTH_STRIDE: usize = 2;
const BASE_VARIANTS: usize = 81;
const TOP_VARIANTS: usize = 16;
const SIDES: [IVec3; 4] = [IVec3::new(0, 0, -1), IVec3::new(0, 0, 1), IVec3::new(-1, 0, 0), IVec3::new(1, 0, 0)];
const TALL_CHANCE: f32 = 0.5;

#[derive(Clone, Copy, Debug)]
enum Choice {
    Carpet,
    ShortGrass,
    TallGrass,
}

#[derive(Debug)]
pub struct MossVegetation {
    choices: [(Choice, i32); 3],
    short: BlockId,
    tall_lower: BlockId,
    tall_upper: BlockId,
    base: Vec<BlockId>,
    top: Vec<BlockId>,
}

#[derive(Debug)]
pub enum Pale {
    Moss(MossVegetation),
    Eyeblossom { block: BlockId },
}

fn carpet_state(table: &mut BlockTable, sides: [&'static str; 4], upper: bool) -> BlockId {
    table.get(
        PALE_MOSS_CARPET,
        &[
            state(&PALE_MOSS_CARPET_SIDE_NORTH, text(sides[0])),
            state(&PALE_MOSS_CARPET_SIDE_SOUTH, text(sides[1])),
            state(&PALE_MOSS_CARPET_SIDE_WEST, text(sides[2])),
            state(&PALE_MOSS_CARPET_SIDE_EAST, text(sides[3])),
            state(&UPPER_BLOCK_BIT, flag(upper)),
        ],
    )
}

fn carpet_variants(table: &mut BlockTable) -> (Vec<BlockId>, Vec<BlockId>) {
    let mut base = vec![0; BASE_VARIANTS];
    let mut top = vec![0; TOP_VARIANTS];
    for north in 0..CARPET_SIDES {
        for east in 0..CARPET_SIDES {
            for south in 0..CARPET_SIDES {
                for west in 0..CARPET_SIDES {
                    let index = north as usize * NORTH_STRIDE + east as usize * EAST_STRIDE + south as usize * SOUTH_STRIDE + west as usize;
                    let sides = [SIDE_VALUES[north as usize], SIDE_VALUES[south as usize], SIDE_VALUES[west as usize], SIDE_VALUES[east as usize]];
                    base[index] = carpet_state(table, sides, false);
                }
            }
        }
    }
    for north in 0..CARPET_TOP_SIDES {
        for east in 0..CARPET_TOP_SIDES {
            for south in 0..CARPET_TOP_SIDES {
                for west in 0..CARPET_TOP_SIDES {
                    let index = north as usize * TOP_NORTH_STRIDE + east as usize * TOP_EAST_STRIDE + south as usize * TOP_SOUTH_STRIDE + west as usize;
                    let sides = [TOP_VALUES[north as usize], TOP_VALUES[south as usize], TOP_VALUES[west as usize], TOP_VALUES[east as usize]];
                    top[index] = carpet_state(table, sides, true);
                }
            }
        }
    }
    (base, top)
}

fn supports(region: &Region, pos: IVec3) -> bool {
    let found = read(region, pos);
    region.blocks.entry(found).solid && !region.blocks.is(found, Tag::Leaves)
}

fn pick(choices: &[(Choice, i32)], random: &mut MersenneTwister) -> Choice {
    let total: i32 = choices.iter().map(|&(_, weight)| weight).sum();
    let mut left = random.next_int_bounded(total);
    for &(choice, weight) in choices {
        left -= weight;
        if left < 0 {
            return choice;
        }
    }
    choices[choices.len() - 1].0
}

impl Pale {
    pub fn register(registry: &mut FeatureRegistry) {
        registry.register_id(MOSS_VEGETATION, |loader, _| {
            let table = loader.table();
            let (base, top) = carpet_variants(table);
            Ok(arc(Self::Moss(MossVegetation {
                choices: [(Choice::Carpet, CARPET_WEIGHT), (Choice::ShortGrass, SHORT_GRASS_WEIGHT), (Choice::TallGrass, TALL_GRASS_WEIGHT)],
                short: table.name(SHORT_GRASS),
                tall_lower: table.get(TALL_GRASS, &[state(&UPPER_BLOCK_BIT, flag(false))]),
                tall_upper: table.get(TALL_GRASS, &[state(&UPPER_BLOCK_BIT, flag(true))]),
                base,
                top,
            })))
        });
        registry.register_id(EYEBLOSSOM, |loader, _| {
            Ok(arc(Self::Eyeblossom {
                block: loader.table().name(CLOSED_EYEBLOSSOM),
            }))
        });
    }

    fn carpet(&self, region: &mut Region, random: &mut MersenneTwister, pos: IVec3, base: &[BlockId], top: &[BlockId]) -> bool {
        let open_above = is_air(region, pos + IVec3::Y);
        let mut low = [0usize; 4];
        let mut high = [0usize; 4];
        for (index, &side) in SIDES.iter().enumerate() {
            let wall = supports(region, pos + side);
            if !open_above {
                low[index] = usize::from(wall);
                continue;
            }
            let tall = random.next_float() > TALL_CHANCE && supports(region, pos + IVec3::Y + side);
            low[index] = if wall { 1 + usize::from(tall) } else { 0 };
            high[index] = usize::from(wall && tall);
        }
        let index = low[0] * NORTH_STRIDE + low[3] * EAST_STRIDE + low[1] * SOUTH_STRIDE + low[2];
        region.set(pos, base[index]);
        if high.iter().any(|&side| side != 0) {
            let index = high[0] * TOP_NORTH_STRIDE + high[3] * TOP_EAST_STRIDE + high[1] * TOP_SOUTH_STRIDE + high[2];
            region.set(pos + IVec3::Y, top[index]);
        }
        true
    }

    fn plant(region: &mut Region, pos: IVec3, lower: BlockId, upper: Option<BlockId>) -> bool {
        if !is_air(region, pos) || !can_survive(region, lower, pos) {
            return false;
        }
        if let Some(upper) = upper {
            if !is_air(region, pos + IVec3::Y) {
                return false;
            }
            region.set(pos + IVec3::Y, upper);
        }
        region.set(pos, lower)
    }

    pub fn place(&self, region: &mut Region, random: &mut MersenneTwister, pos: IVec3) -> bool {
        match self {
            Self::Eyeblossom { block } => is_air(region, pos) && can_survive(region, *block, pos) && region.set(pos, *block),
            Self::Moss(moss) => {
                let choice = pick(&moss.choices, random);
                if !is_air(region, pos) || !supports(region, pos - IVec3::Y) {
                    return false;
                }
                match choice {
                    Choice::Carpet => self.carpet(region, random, pos, &moss.base, &moss.top),
                    Choice::ShortGrass => Self::plant(region, pos, moss.short, None),
                    Choice::TallGrass => Self::plant(region, pos, moss.tall_lower, Some(moss.tall_upper)),
                }
            }
        }
    }
}
