use super::ore::Draw;
use super::world::{is_air, read};
use crate::shared::blocks::{BlockId, BlockTable};
use crate::shared::chunk::region::Region;
use chorus_block::block_id::*;
use glam::IVec3;

const DESCENT_MARGIN: i32 = 2;
const PLUS_SIZE: i32 = 5;
const PLUS: [(i32, i32); 5] = [(0, 0), (-1, 0), (1, 0), (0, -1), (0, 1)];
const CORNERS: [(i32, i32); 4] = [(-1, -1), (-1, 1), (1, -1), (1, 1)];
const GROUND: [&str; 3] = [SAND, RED_SAND, SUSPICIOUS_SAND];
const WELL_BRUSHABLE_BLOCKS_ENABLED: bool = true;

#[derive(Debug)]
pub struct DesertWell {
    sand: BlockId,
    sandstone: BlockId,
    slab: BlockId,
    water: BlockId,
    suspicious: BlockId,
    ground: Vec<u16>,
}

impl DesertWell {
    pub fn new(table: &mut BlockTable) -> Self {
        Self {
            sand: table.name(SAND),
            sandstone: table.name(SANDSTONE),
            slab: table.name(SANDSTONE_SLAB),
            water: table.name(WATER),
            suspicious: table.name(SUSPICIOUS_SAND),
            ground: GROUND.iter().map(|name| table.kind(*name)).collect(),
        }
    }

    pub fn place<D: Draw>(&self, region: &mut Region, random: &mut D, origin: IVec3) -> bool {
        if !is_air(region, origin) {
            return false;
        }
        let mut base = origin;
        while base.y > region.min_y() + DESCENT_MARGIN && is_air(region, base) {
            base.y -= 1;
        }
        if !self.ground.contains(&region.blocks.entry(read(region, base)).kind) {
            return false;
        }
        for dx in -2..=2 {
            for dz in -2..=2 {
                let column = base + IVec3::new(dx, 0, dz);
                if is_air(region, column - IVec3::Y) && is_air(region, column - IVec3::Y * 2) {
                    return false;
                }
            }
        }
        let put = |region: &mut Region, dx: i32, dy: i32, dz: i32, block: BlockId| {
            region.set(base + IVec3::new(dx, dy, dz), block);
        };
        for dy in -2..=0 {
            for dx in -2..=2 {
                for dz in -2..=2 {
                    put(region, dx, dy, dz, self.sandstone);
                }
            }
        }
        for &(dx, dz) in &PLUS {
            put(region, dx, 0, dz, self.water);
            put(region, dx, -1, dz, self.sand);
        }
        for dx in -2..=2 {
            put(region, dx, 1, -2, self.sandstone);
            if dx == -2 || dx == 2 {
                for dz in -1..=1 {
                    put(region, dx, 1, dz, self.sandstone);
                }
            }
            put(region, dx, 1, 2, self.sandstone);
        }
        for (dx, dz) in [(2, 0), (-2, 0), (0, 2), (0, -2)] {
            put(region, dx, 1, dz, self.slab);
        }
        for dz in -1..=1 {
            put(region, -1, 4, dz, self.slab);
            put(region, 0, 4, dz, if dz == 0 { self.sandstone } else { self.slab });
            put(region, 1, 4, dz, self.slab);
        }
        for dy in 1..=3 {
            for &(dx, dz) in &CORNERS {
                put(region, dx, dy, dz, self.sandstone);
            }
        }
        if WELL_BRUSHABLE_BLOCKS_ENABLED {
            for dy in [-1, -2] {
                let (dx, dz) = PLUS[random.below(PLUS_SIZE) as usize];
                put(region, dx, dy, dz, self.suspicious);
            }
        }
        true
    }
}
