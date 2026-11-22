use super::expression::Host;
use crate::shared::biome::Biome;
use crate::shared::blocks::{AIR, BlockId};
use crate::shared::chunk::region::{Heightmap, Region};
use crate::shared::tags::Tag;
use glam::IVec3;
use std::sync::LazyLock;

const DEFAULT_BIOME: Biome = Biome::OCEAN;

pub fn biome_at(region: &Region, pos: IVec3) -> Biome {
    if region.contains(pos) { region.biome(pos) } else { DEFAULT_BIOME }
}

pub fn biome_id_at(region: &Region, pos: IVec3) -> i32 {
    static DEFAULT_ID: LazyLock<i32> = LazyLock::new(|| DEFAULT_BIOME.bedrock_id());
    if region.contains(pos) { region.biome_id(pos) } else { *DEFAULT_ID }
}

pub fn read(region: &Region, pos: IVec3) -> BlockId {
    region.get_contained(pos).unwrap_or(AIR)
}

pub fn is_air(region: &Region, pos: IVec3) -> bool {
    region.blocks.is_air(read(region, pos))
}

pub fn is_solid(region: &Region, pos: IVec3) -> bool {
    region.blocks.entry(read(region, pos)).solid
}

pub fn is_protected(region: &Region, pos: IVec3) -> bool {
    region.blocks.is(read(region, pos), Tag::FeaturesCannotReplace)
}

impl Host for Region<'_> {
    fn heightmap(&mut self, x: i32, z: i32) -> i32 {
        self.height(Heightmap::WorldSurfaceWg, x, z)
    }

    fn above_top_solid(&mut self, x: i32, z: i32) -> i32 {
        if !self.contains(IVec3::new(x, self.min_y(), z)) {
            return self.max_y() + 1;
        }
        self.height(Heightmap::AboveTopSolid, x, z)
    }
}
