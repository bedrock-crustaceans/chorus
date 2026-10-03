use glam::IVec3;

use super::super::blocks::BlockId;
pub use super::super::survival::*;
use super::super::tags::Tag;
use super::region::Region;

#[derive(Clone)]
pub enum BlockPredicate {
    True,
    MatchingBlocks(IVec3, Vec<u16>),
    MatchingTag(IVec3, Tag),
    MatchingFluids(IVec3, Vec<Fluid>),
    NoFluid(IVec3),
    Solid(IVec3),
    Replaceable(IVec3),
    WouldSurvive(IVec3, BlockId),
    SturdyFace(IVec3),
    InsideWorld(IVec3),
    Not(Box<BlockPredicate>),
    AllOf(Vec<BlockPredicate>),
    AnyOf(Vec<BlockPredicate>),
}

pub fn empty() -> BlockPredicate {
    BlockPredicate::MatchingTag(IVec3::ZERO, Tag::Air)
}

impl BlockPredicate {
    pub fn not(self) -> Self {
        Self::Not(Box::new(self))
    }

    pub fn test(&self, region: &Region, pos: IVec3) -> bool {
        let blocks = region.blocks;
        match self {
            Self::True => true,
            Self::MatchingBlocks(offset, kinds) => kinds.contains(&blocks.entry(region.get(pos + *offset)).kind),
            Self::MatchingTag(offset, tag) => blocks.is(region.get(pos + *offset), *tag),
            Self::MatchingFluids(offset, fluids) => fluid_at(blocks, region.get(pos + *offset)).is_some_and(|fluid| fluids.contains(&fluid)),
            Self::NoFluid(offset) => fluid_at(blocks, region.get(pos + *offset)).is_none(),
            Self::Solid(offset) => blocks.entry(region.get(pos + *offset)).solid,
            Self::Replaceable(offset) => blocks.is(region.get(pos + *offset), Tag::Replaceable),
            Self::WouldSurvive(offset, block) => can_survive(region, *block, pos + *offset),
            Self::SturdyFace(offset) => blocks.entry(region.get(pos + *offset)).full,
            Self::InsideWorld(offset) => !region.is_outside_build_height((pos + *offset).y),
            Self::Not(inner) => !inner.test(region, pos),
            Self::AllOf(predicates) => predicates.iter().all(|predicate| predicate.test(region, pos)),
            Self::AnyOf(predicates) => predicates.iter().any(|predicate| predicate.test(region, pos)),
        }
    }
}
