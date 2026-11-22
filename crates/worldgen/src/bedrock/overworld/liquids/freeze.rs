use super::Liquids;
use super::kinds::Kinds;
use crate::bedrock::overworld::surface::temperature_at;
use crate::shared::blocks::{AIR, BlockId};
use crate::shared::chunk::region::{Heightmap, Region};
use crate::shared::tags::Tag;
use chorus_block::state::block_state::BlockState;
use glam::IVec3;

const FREEZING_TEMPERATURE: f32 = 0.15;
const BARE_BLOCKS: [&str; 4] = ["ice", "frosted_ice", "powder_snow", "barrier"];

fn is_cold(region: &Region, pos: IVec3) -> bool {
    region.contains(pos) && temperature_at(&region.biome(pos), pos.x, pos.y, pos.z, region.sea_level()).is_some_and(|temperature| temperature <= FREEZING_TEMPERATURE)
}

fn cell(region: &Region, pos: IVec3) -> BlockId {
    region.get_contained(pos).unwrap_or(AIR)
}

fn holds_snow(region: &Region, block: BlockId) -> bool {
    let entry = region.blocks.entry(block);
    let name = entry.name.as_ref().strip_prefix("minecraft:").unwrap_or(entry.name.as_ref());
    if region.blocks.is(block, Tag::Leaves) {
        return true;
    }
    if name.ends_with("_stairs") {
        return false;
    }
    if name.ends_with("_slab") && !name.ends_with("_double_slab") {
        return entry
            .states
            .iter()
            .any(|(key, value)| key.as_ref() == "minecraft:vertical_half" && matches!(value, BlockState::Enum(half) if half.as_ref() == "top"));
    }
    entry.full && !BARE_BLOCKS.contains(&name) && !name.ends_with("fence_gate")
}

pub fn freeze_chunk(region: &mut Region, ids: &Liquids, kinds: &Kinds) {
    let (min_x, min_z) = (region.owner_x << 4, region.owner_z << 4);
    for x in min_x..min_x + 16 {
        for z in min_z..min_z + 16 {
            let top = region.height(Heightmap::MotionBlocking, x, z);
            let below = IVec3::new(x, top - 1, z);
            if kinds.is_water_source_block(cell(region, below)) && is_cold(region, below) {
                region.set(below, ids.ice);
            }
            let here = IVec3::new(x, top, z);
            let covered = cell(region, here);
            if (region.blocks.is_air(covered) || ids.is_snowable_plant(covered)) && holds_snow(region, cell(region, below)) && is_cold(region, here) {
                region.set(here, ids.snow_layer);
            }
        }
    }
}
