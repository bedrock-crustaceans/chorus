use super::super::placed::Placed;
use super::super::world::{is_air, read};
use super::support::{holds_face_at, holds_vine, is_wet};
use super::{BlockSet, pick_weighted};
use crate::shared::blocks::{BlockId, BlockTable, State, flag, int, state, text};
use crate::shared::chunk::region::{Heightmap, Region};
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::Probed;
use crate::shared::survival::can_survive;
use chorus_block::block_id::*;
use chorus_block::state::common::{MINECRAFT_CARDINAL_DIRECTION, UPPER_BLOCK_BIT, VINE_DIRECTION_BITS};
use glam::IVec3;
use std::sync::Arc;

const MOSS_VEGETATION_WEIGHTS: [f32; 5] = [4.0, 7.0, 25.0, 50.0, 10.0];
const SMALL_DRIPLEAF_CHANCE: f32 = 0.2;
const DRIPLEAF_FACINGS: [&str; 4] = chorus_block::state::common::CARDINAL_DIRECTIONS;
const VINE_SIDES: [(IVec3, i32); 4] = [(IVec3::NEG_Z, 4), (IVec3::X, 8), (IVec3::Z, 1), (IVec3::NEG_X, 2)];
const ROOT_COLUMN_LIMIT: i32 = 184;
const ROOT_ATTEMPTS: usize = 20;
const ROOT_SPREAD: i32 = 3;
const HANGING_SPREAD: i32 = 2;
const ROOT_TREE_HEIGHT: i32 = 3;
const ROOT_REPLACEABLE: [&str; 17] = [
    CLAY,
    MOSS_BLOCK,
    SAND,
    GRAVEL,
    DIRT,
    PODZOL,
    DIRT_WITH_ROOTS,
    GRASS_BLOCK,
    MYCELIUM,
    STONE,
    DEEPSLATE,
    TUFF,
    CAVE_VINES,
    CAVE_VINES_HEAD_WITH_BERRIES,
    MOSS_CARPET,
    SHORT_GRASS,
    FERN,
];

#[derive(Debug)]
pub struct Plantlet {
    lower: BlockId,
    upper: Option<BlockId>,
}

#[derive(Debug)]
pub enum Lush {
    MossVegetation(Vec<(Plantlet, f32)>),
    Dripleaf { small: Vec<Plantlet>, big: Vec<Arc<Placed>> },
    VinesSingleFace([BlockId; 4]),
    SporeBlossom(BlockId),
    AzaleaRoots(Roots),
}

#[derive(Debug)]
pub struct Roots {
    tree: Option<Arc<Placed>>,
    replaceable: BlockSet,
    rooted_dirt: BlockId,
    hanging: BlockId,
}

impl Plantlet {
    fn single(block: BlockId) -> Self {
        Self { lower: block, upper: None }
    }

    fn double(table: &mut BlockTable, name: &'static str, extra: &[State], flooded: bool) -> Self {
        let mut lower_states = vec![state(&UPPER_BLOCK_BIT, flag(false))];
        let mut upper_states = vec![state(&UPPER_BLOCK_BIT, flag(true))];
        for (key, value) in extra {
            lower_states.push((key.clone(), value.clone()));
            upper_states.push((key.clone(), value.clone()));
        }
        let (lower, upper) = (table.get(name, &lower_states), table.get(name, &upper_states));
        if flooded {
            table.flooded(lower);
            table.flooded(upper);
        }
        Self { lower, upper: Some(upper) }
    }
}

pub fn moss_vegetation(table: &mut BlockTable) -> Lush {
    let blocks = [
        Plantlet::single(table.name(FLOWERING_AZALEA)),
        Plantlet::single(table.name(AZALEA)),
        Plantlet::single(table.name(MOSS_CARPET)),
        Plantlet::single(table.name(SHORT_GRASS)),
        Plantlet::double(table, TALL_GRASS, &[], false),
    ];
    Lush::MossVegetation(blocks.into_iter().zip(MOSS_VEGETATION_WEIGHTS).collect())
}

pub fn dripleaf(table: &mut BlockTable, big: Vec<Arc<Placed>>) -> Lush {
    let small = DRIPLEAF_FACINGS
        .iter()
        .map(|facing| Plantlet::double(table, SMALL_DRIPLEAF_BLOCK, &[state(&MINECRAFT_CARDINAL_DIRECTION, text(facing))], true))
        .collect();
    Lush::Dripleaf { small, big }
}

pub fn vines_single_face(table: &mut BlockTable) -> Lush {
    Lush::VinesSingleFace(VINE_SIDES.map(|(_, bits)| table.get(VINE, &[state(&VINE_DIRECTION_BITS, int(bits))])))
}

pub fn spore_blossom(table: &mut BlockTable) -> Lush {
    Lush::SporeBlossom(table.name(SPORE_BLOSSOM))
}

pub fn azalea_roots(table: &mut BlockTable, tree: Option<Arc<Placed>>) -> Lush {
    Lush::AzaleaRoots(Roots {
        tree,
        replaceable: BlockSet::from_names(table, &ROOT_REPLACEABLE),
        rooted_dirt: table.name(DIRT_WITH_ROOTS),
        hanging: table.name(HANGING_ROOTS),
    })
}

fn put_plantlet(region: &mut Region, pos: IVec3, plantlet: &Plantlet) -> bool {
    if !can_survive(region, plantlet.lower, pos) {
        return false;
    }
    let Some(upper) = plantlet.upper else { return region.set(pos, plantlet.lower) };
    let above = pos + IVec3::Y;
    let above_block = read(region, above);
    let open_above = region.blocks.is_air(above_block) || super::support::is_water_source(region.blocks, above_block);
    if !open_above {
        return false;
    }
    let wet = |region: &Region, at: IVec3| is_wet(region.blocks, read(region, at)) && super::support::is_water_source(region.blocks, read(region, at));
    let (upper_wet, lower_wet) = (wet(region, above), wet(region, pos));
    let upper_block = if upper_wet { region.blocks.flooded(upper) } else { upper };
    let lower_block = if lower_wet { region.blocks.flooded(plantlet.lower) } else { plantlet.lower };
    region.set(pos, lower_block);
    region.set(above, upper_block);
    true
}

fn jitter(random: &mut MersenneTwister, bound: i32) -> i32 {
    let first = random.next_int_bounded(bound);
    first - random.next_int_bounded(bound)
}

impl Lush {
    pub fn place(&self, region: &mut Region, random: &mut MersenneTwister, pos: IVec3) -> Probed<bool> {
        Ok(match self {
            Self::MossVegetation(choices) => {
                let Some(plantlet) = pick_weighted(choices, random) else { return Ok(false) };
                is_air(region, pos) && holds_face_at(region, pos - IVec3::Y) && put_plantlet(region, pos, plantlet)
            }
            Self::Dripleaf { small, big } => {
                let is_small = random.next_float() < SMALL_DRIPLEAF_CHANCE;
                let facing = random.next_int_bounded(DRIPLEAF_FACINGS.len() as i32) as usize;
                if is_small {
                    put_plantlet(region, pos, &small[facing])
                } else {
                    big[facing].place(region, random, pos)?
                }
            }
            Self::VinesSingleFace(vines) => {
                if !is_air(region, pos) {
                    return Ok(false);
                }
                for (&(offset, _), &vine) in VINE_SIDES.iter().zip(vines) {
                    if holds_vine(region.blocks, read(region, pos + offset)) {
                        return Ok(region.set(pos, vine));
                    }
                }
                false
            }
            Self::SporeBlossom(block) => {
                random.next_int_bounded(1);
                can_survive(region, *block, pos) && region.set(pos, *block)
            }
            Self::AzaleaRoots(roots) => roots.place(region, random, pos)?,
        })
    }
}

impl Roots {
    fn open_column(&self, region: &Region, pos: IVec3) -> bool {
        let blocks = region.blocks;
        let open = |y: i32| {
            let block = read(region, IVec3::new(pos.x, y, pos.z));
            blocks.is_air(block) || super::support::is_water_source(blocks, block)
        };
        let wet = |y: i32| super::support::is_water_source(blocks, read(region, IVec3::new(pos.x, y, pos.z)));
        (0..ROOT_TREE_HEIGHT).all(|offset| open(pos.y + offset)) && !(0..ROOT_TREE_HEIGHT).all(|offset| wet(pos.y + offset))
    }

    fn place(&self, region: &mut Region, random: &mut MersenneTwister, pos: IVec3) -> Probed<bool> {
        let top = region.height(Heightmap::MotionBlocking, pos.x, pos.z);
        let mut tree_y = None;
        if let Some(tree) = &self.tree {
            for y in pos.y..pos.y + ROOT_COLUMN_LIMIT {
                let at = IVec3::new(pos.x, y, pos.z);
                let grounded = region.blocks.entry(read(region, at - IVec3::Y)).solid;
                if grounded && self.open_column(region, at) && tree.place(region, random, at)? {
                    tree_y = Some(y);
                    break;
                }
                if y >= top {
                    break;
                }
            }
        }
        let Some(tree_y) = tree_y else { return Ok(false) };
        for y in pos.y + 1..=tree_y {
            for _ in 0..ROOT_ATTEMPTS {
                let dx = jitter(random, ROOT_SPREAD);
                let dz = jitter(random, ROOT_SPREAD);
                let at = IVec3::new(pos.x + dx, y, pos.z + dz);
                if self.replaceable.contains(region.blocks, read(region, at)) {
                    region.set(at, self.rooted_dirt);
                }
            }
        }
        let mut walker = pos;
        for _ in 0..ROOT_ATTEMPTS {
            walker.x += jitter(random, ROOT_SPREAD);
            walker.z += jitter(random, ROOT_SPREAD);
            walker.y += jitter(random, HANGING_SPREAD);
            if is_air(region, walker) && region.blocks.entry(read(region, walker + IVec3::Y)).solid {
                region.set(walker, self.hanging);
            }
        }
        Ok(true)
    }
}
