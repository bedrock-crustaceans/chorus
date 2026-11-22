use super::ore::Draw;
use super::world::{biome_at, read};
use crate::shared::biome::Biome;
use crate::shared::blocks::{BlockId, BlockTable};
use crate::shared::chunk::region::{Region, RuleKind};
use chorus_block::block_id::*;
use glam::IVec3;

const SPRING_TICK_DELAY: u64 = 1;
const COLUMN_SPAN: i32 = 16;
const WATER_TRIES: i32 = 25;
const LAVA_TRIES: i32 = 20;
const WATER_RANGE: i32 = 256;
const LAVA_RANGE: i32 = 240;
const LAVA_FLOOR_ABOVE_BOTTOM: i32 = 8;
const REQUIRED_ROCK_SIDES: usize = 4;
const REQUIRED_AIR_SIDES: usize = 1;
const NO_LAVA_BIOMES: [Biome; 4] = [Biome::DEEP_DARK, Biome::FROZEN_OCEAN, Biome::DEEP_FROZEN_OCEAN, Biome::ICE_SPIKES];
const ROCKS: [&str; 9] = [STONE, DEEPSLATE, TUFF, CALCITE, SNOW, POWDER_SNOW, PACKED_ICE, DIRT, COARSE_DIRT];
const SIDES: [IVec3; 4] = [IVec3::NEG_X, IVec3::X, IVec3::NEG_Z, IVec3::Z];

#[derive(Debug)]
pub struct Springs {
    water: BlockId,
    lava: BlockId,
    rocks: Vec<u16>,
}

impl Springs {
    pub fn new(table: &mut BlockTable) -> Self {
        Self {
            water: table.name(WATER),
            lava: table.name(LAVA),
            rocks: ROCKS.iter().map(|name| table.kind(*name)).collect(),
        }
    }

    pub fn place<D: Draw>(&self, region: &mut Region, random: &mut D, origin: IVec3) -> bool {
        region.set_rule(&self.rocks, RuleKind::Replaceable);
        for _ in 0..WATER_TRIES {
            let z = random.below(COLUMN_SPAN);
            let y = random.below(WATER_RANGE) + region.min_y();
            let x = random.below(COLUMN_SPAN);
            self.spring(region, origin + IVec3::new(x, y, z), self.water);
        }
        for _ in 0..LAVA_TRIES {
            let z = random.below(COLUMN_SPAN);
            let bound = random.below(LAVA_RANGE);
            let bound = random.below(bound);
            let y = random.below(bound) + region.min_y() + LAVA_FLOOR_ABOVE_BOTTOM;
            let x = random.below(COLUMN_SPAN);
            let pos = origin + IVec3::new(x, y, z);
            if !NO_LAVA_BIOMES.contains(&biome_at(region, pos)) {
                self.spring(region, pos, self.lava);
            }
        }
        true
    }

    fn is_rock(&self, region: &Region, pos: IVec3) -> bool {
        self.rocks.contains(&region.blocks.entry(read(region, pos)).kind)
    }

    pub(super) fn spring(&self, region: &mut Region, pos: IVec3, liquid: BlockId) {
        let here_is_air = region.blocks.is_air(read(region, pos));
        if !self.is_rock(region, pos + IVec3::Y) || !self.is_rock(region, pos - IVec3::Y) || !(here_is_air || self.is_rock(region, pos)) {
            return;
        }
        let around: Vec<IVec3> = SIDES.iter().map(|side| pos + *side).chain([pos - IVec3::Y]).collect();
        let rocks = around.iter().filter(|&&side| self.is_rock(region, side)).count();
        let air = around.iter().filter(|&&side| region.blocks.is_air(read(region, side))).count();
        if rocks == REQUIRED_ROCK_SIDES && air == REQUIRED_AIR_SIDES {
            region.set(pos, liquid);
            region.schedule_fluid(pos, liquid == self.lava, SPRING_TICK_DELAY);
        }
    }
}
