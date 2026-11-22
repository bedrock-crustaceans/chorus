use super::world::read;
use crate::shared::blocks::{BlockId, BlockTable};
use crate::shared::chunk::region::Region;
use crate::shared::rng::mersenne::MersenneTwister;
use chorus_block::block_id::{AIR, COARSE_DIRT, DIRT, GRASS_BLOCK, ICE, PACKED_ICE, SNOW};
use glam::IVec3;

const PATCH_RADIUS: i32 = 4;
const PATCH_MIN_RADIUS: i32 = 2;
const DESCENT_SENTINEL: i32 = 50;
const SPIKE_DEPTH: i32 = 13;
const FILL_CHANCE: f32 = 0.75;

#[derive(Debug)]
pub struct IcePatch {
    block: BlockId,
    air: BlockId,
    snow: BlockId,
    ice: BlockId,
    dirt: BlockId,
    coarse_dirt: BlockId,
}

impl IcePatch {
    pub fn new(table: &mut BlockTable) -> Self {
        Self {
            block: table.name(PACKED_ICE),
            air: table.name(AIR),
            snow: table.name(SNOW),
            ice: table.name(ICE),
            dirt: table.name(DIRT),
            coarse_dirt: table.name(COARSE_DIRT),
        }
    }

    fn replaceable(&self, region: &Region, pos: IVec3) -> bool {
        let block = read(region, pos);
        block == self.snow || block == self.ice || block == self.dirt || block == self.coarse_dirt
    }

    pub fn place(&self, region: &mut Region, random: &mut MersenneTwister, pos: IVec3) -> bool {
        let mut origin = pos;
        while read(region, origin) == self.air && origin.y > region.min_y() + 2 {
            origin.y -= 1;
        }
        if read(region, origin) != self.snow {
            return false;
        }
        let radius = random.next_int_bounded(PATCH_RADIUS - PATCH_MIN_RADIUS) + PATCH_MIN_RADIUS;
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                if dx * dx + dz * dz > radius * radius {
                    continue;
                }
                for dy in [-1, 0, 1] {
                    let cell = origin + IVec3::new(dx, dy, dz);
                    if self.replaceable(region, cell) {
                        region.set(cell, self.block);
                    }
                }
            }
        }
        true
    }
}

#[derive(Debug)]
pub struct IceSpike {
    block: BlockId,
    air: BlockId,
    grass_block: BlockId,
    dirt: BlockId,
    coarse_dirt: BlockId,
    snow: BlockId,
    ice: BlockId,
}

impl IceSpike {
    pub fn new(table: &mut BlockTable) -> Self {
        Self {
            block: table.name(PACKED_ICE),
            air: table.name(AIR),
            grass_block: table.name(GRASS_BLOCK),
            dirt: table.name(DIRT),
            coarse_dirt: table.name(COARSE_DIRT),
            snow: table.name(SNOW),
            ice: table.name(ICE),
        }
    }

    fn replaceable(&self, region: &Region, pos: IVec3) -> bool {
        let block = read(region, pos);
        block == self.air || block == self.grass_block || block == self.dirt || block == self.coarse_dirt || block == self.snow || block == self.ice || block == self.block
    }

    fn place_cell(&self, region: &mut Region, origin: IVec3, dx: i32, layer: i32, dz: i32, lower: bool) {
        let cell = origin + IVec3::new(dx, layer, dz);
        if self.replaceable(region, cell) {
            region.set(cell, self.block);
        }
        if lower {
            let cell = origin + IVec3::new(dx, -layer, dz);
            if self.replaceable(region, cell) {
                region.set(cell, self.block);
            }
        }
    }

    fn descend(&self, region: &mut Region, random: &mut MersenneTwister, origin: IVec3, dx: i32, dz: i32) {
        let floor = region.sea_level() - SPIKE_DEPTH;
        let mut y = origin.y - 1;
        let mut length = if dx.abs() == 1 && dz.abs() == 1 { random.next_int_bounded(5) } else { DESCENT_SENTINEL };
        loop {
            let mut blocked = false;
            loop {
                if y <= floor {
                    blocked = true;
                    break;
                }
                let cell = IVec3::new(origin.x + dx, y, origin.z + dz);
                if !self.replaceable(region, cell) {
                    blocked = true;
                    break;
                }
                region.set(cell, self.block);
                y -= 1;
                length -= 1;
                if length <= 0 {
                    break;
                }
            }
            if blocked {
                break;
            }
            let gap = random.next_int_bounded(5);
            y -= gap + 1;
            length = random.next_int_bounded(5);
        }
    }

    pub fn place(&self, region: &mut Region, random: &mut MersenneTwister, pos: IVec3) -> bool {
        let mut origin = pos;
        while read(region, origin) == self.air && origin.y > region.min_y() + 2 {
            origin.y -= 1;
        }
        if read(region, origin) != self.snow {
            return false;
        }
        origin.y += random.next_int_bounded(4);
        let height = random.next_int_bounded(4) + 7;
        let radius = height / 4 + random.next_int_bounded(2);
        if radius > 1 && random.next_int_bounded(60) == 0 {
            origin.y += random.next_int_bounded(30) + 10;
        }
        for layer in 0..height {
            let layer_radius = (1.0 - layer as f32 / height as f32) * radius as f32;
            let bounds = layer_radius.ceil() as i32;
            let squared = layer_radius * layer_radius;
            let lower = layer != 0 && bounds > 1;
            let edge = (bounds as f32 - 0.25) * (bounds as f32 - 0.25);
            for dx in -bounds..=bounds {
                let x = (dx.abs() as f32 - 0.25) * (dx.abs() as f32 - 0.25);
                if x + edge <= squared && random.next_float() <= FILL_CHANCE {
                    self.place_cell(region, origin, dx, layer, -bounds, lower);
                }
                for dz in 1 - bounds..=bounds {
                    let z = (dz.abs() as f32 - 0.25) * (dz.abs() as f32 - 0.25);
                    if (dx == 0 && dz == 0) || x + z <= squared {
                        let boundary = dx == bounds || dz == bounds || dx == -bounds || dz == -bounds;
                        if !boundary || random.next_float() <= FILL_CHANCE {
                            self.place_cell(region, origin, dx, layer, dz, lower);
                        }
                    }
                }
            }
        }
        let span = (radius.min(2) - 1).max(0);
        for dx in -span..=span {
            for dz in -span..=span {
                self.descend(region, random, origin, dx, dz);
            }
        }
        true
    }
}
