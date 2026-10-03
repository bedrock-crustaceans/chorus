use glam::IVec3;

use super::super::blocks::{BlockId, Blocks, flag, state, text};
use super::super::random::RandomSource;
use super::super::tags::Tag;
use super::placement::Placed;
use super::predicate::{Fluid, can_survive, fluid_at};
use super::provider::{StateProvider, weighted};

use super::{Context, Feature};

use chorus_block::state::block_state::BlockState;
use chorus_block::state::common::{SEA_GRASS_TYPE, UPPER_BLOCK_BIT};

pub struct SimpleBlock {
    pub state: StateProvider,
}

impl SimpleBlock {
    pub fn place(&self, ctx: &mut Context, random: &mut dyn RandomSource, pos: IVec3) -> bool {
        let Some(state) = self.state.optional_state(ctx.region, random, pos) else { return false };
        if !can_survive(ctx.region, state, pos) {
            return false;
        }
        let blocks = ctx.region.blocks;
        if blocks.is_kind(state, ctx.catalog.moss_carpet.kind) {
            ctx.catalog.moss_carpet.place(ctx.region, ctx.region_random, pos);
            return true;
        }
        if let Some(upper) = upper_half(blocks, state) {
            let above = ctx.region.get(pos + IVec3::Y);
            let same_fluid = fluid_at(blocks, above) == fluid_at(blocks, state);
            if !blocks.is_air(above) && (!same_fluid || !blocks.is(above, Tag::Replaceable)) {
                return false;
            }
            let wet = |block: BlockId, at: BlockId| if fluid_at(blocks, at) == Some(Fluid::Water) { blocks.flooded(block) } else { block };
            ctx.region.set(pos, wet(state, ctx.region.get(pos)));
            ctx.region.set(pos + IVec3::Y, wet(upper, above));
        } else {
            ctx.region.set(pos, state);
        }
        true
    }
}

fn upper_half(blocks: &Blocks, lower: BlockId) -> Option<BlockId> {
    match (blocks.state(lower, UPPER_BLOCK_BIT), blocks.text(lower, SEA_GRASS_TYPE)) {
        (Some(BlockState::Bool(false)), _) => Some(blocks.with(lower, state(UPPER_BLOCK_BIT, flag(true)))),
        (_, Some("double_bot")) => Some(blocks.with(lower, state(SEA_GRASS_TYPE, text("double_top")))),
        _ => None,
    }
}

pub fn sequence(ctx: &mut Context, random: &mut dyn RandomSource, features: &[Placed], pos: IVec3) -> bool {
    for feature in features {
        if !feature.place(ctx, random, pos, None) {
            return false;
        }
    }
    true
}

pub fn random_selector(ctx: &mut Context, random: &mut dyn RandomSource, features: &[(Placed, f32)], default: &Placed, pos: IVec3) -> bool {
    for (feature, chance) in features {
        if random.next_float() < *chance {
            return feature.place(ctx, random, pos, None);
        }
    }
    default.place(ctx, random, pos, None)
}

pub fn simple_random_selector(ctx: &mut Context, random: &mut dyn RandomSource, features: &[Placed], pos: IVec3) -> bool {
    let index = random.next_int_bounded(features.len() as i32) as usize;
    features[index].place(ctx, random, pos, None)
}

pub fn weighted_selector(ctx: &mut Context, random: &mut dyn RandomSource, features: &[(Placed, i32)], pos: IVec3) -> bool {
    weighted(random, features).place(ctx, random, pos, None)
}

pub fn simple(state: impl Into<StateProvider>) -> Feature {
    Feature::SimpleBlock(SimpleBlock { state: state.into() })
}
