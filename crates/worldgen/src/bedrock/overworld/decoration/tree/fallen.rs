use super::Tree;
use super::decoration::Decoration;
use super::site::{block, put};
use super::spec::FallenTrunk;
use crate::bedrock::overworld::decoration::ore::Draw;
use crate::shared::chunk::region::Region;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::Probed;
use crate::shared::survival::fluid_at;
use chorus_block::block_id::*;
use glam::IVec3;

const FACINGS: [IVec3; 4] = [IVec3::new(0, 0, -1), IVec3::new(0, 0, 1), IVec3::new(-1, 0, 0), IVec3::new(1, 0, 0)];
const PASSABLE_PLANTS: [&str; 7] = [SHORT_GRASS, TALL_GRASS, FERN, LARGE_FERN, DEADBUSH, SNOW_LAYER, LEAF_LITTER];
const MAX_UNSUPPORTED: i32 = 2;

fn passable(region: &Region, pos: IVec3) -> bool {
    let found = block(region, pos);
    region.blocks.is_air(found) || fluid_at(region.blocks, found).is_some() || PASSABLE_PLANTS.contains(&region.blocks.name_of(found))
}

fn fits(region: &Region, start: IVec3, facing: IVec3, logs: i32) -> bool {
    let mut ok = true;
    let mut unsupported = 0;
    for index in 0..logs {
        let pos = start + facing * index;
        if !passable(region, pos) {
            ok = false;
        }
        if region.blocks.entry(block(region, pos - IVec3::Y)).solid {
            unsupported = 0;
        } else {
            if unsupported >= MAX_UNSUPPORTED {
                ok = false;
            }
            unsupported += 1;
        }
    }
    ok
}

impl FallenTrunk {
    pub fn place(&self, tree: &Tree, region: &mut Region, random: &mut MersenneTwister, origin: IVec3) -> Probed<Option<IVec3>> {
        let length = self.log_length.draw(random);
        let Some(origin) = tree.roots_then(region, random, origin, length) else { return Ok(None) };
        if !tree.prepare(region, origin, length, 1) {
            return Ok(None);
        }
        let modifier = self.height_modifier.draw(random);
        let facing = FACINGS[random.below(4) as usize];
        let offset = random.below(2) + 2;
        let logs = length + modifier - 2;
        let stump = self.stump_height.draw(random);
        let start_column = origin + facing * offset;
        let mut start = IVec3::new(start_column.x, origin.y + stump, start_column.z);
        while start.y > region.min_y() && passable(region, start - IVec3::Y) {
            start.y -= 1;
        }
        if logs >= 1 && fits(region, start, facing, logs) {
            let value = if facing.x != 0 { self.log.along_x } else { self.log.along_z };
            for index in 0..logs {
                let pos = start + facing * index;
                put(region, pos, value);
                if let Some(feature) = &self.log_decoration {
                    feature.place(region, random, pos)?;
                }
            }
        }
        let mut last = None;
        for index in 0..stump {
            let pos = origin + IVec3::Y * index;
            if tree.may_replace.allows(region.blocks, block(region, pos)) {
                let placed = Decoration::place_decorated(self.decoration.as_ref(), region, &tree.foliage, random, pos, self.log.upright);
                last = placed.then_some(pos);
            }
        }
        tree.place_base_block(region, origin - IVec3::Y);
        Ok(last.map(|pos| pos + IVec3::Y))
    }
}
