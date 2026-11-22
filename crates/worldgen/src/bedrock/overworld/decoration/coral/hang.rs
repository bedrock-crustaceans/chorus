use super::super::aquatic::is_solid_blocking;
use super::super::ore::Draw;
use super::super::world::read;
use super::kinds::Coral;
use super::{EAST, FACES, NORTH, SOUTH, WALL_DIRECTION, WEST};
use crate::shared::chunk::region::Region;
use glam::IVec3;

const HANG_DEPTH: i32 = 13;

const TRIED_FACES: [usize; 4] = [EAST, SOUTH, WEST, NORTH];

const HANG_DIRECTIONS: i32 = 4;

impl Coral {
    pub fn hang<D: Draw>(&self, region: &mut Region, random: &mut D, at: IVec3) -> bool {
        let ceiling = region.sea_level() - HANG_DEPTH;
        let mut pos = at;
        while !is_solid_blocking(region, pos) && pos.y > ceiling {
            pos.y -= 1;
        }
        let on = read(region, pos);
        if !is_solid_blocking(region, pos) || (!self.is_coral(region, pos) && region.blocks.entry(on).kind != self.stone_kind) {
            return false;
        }
        let mut faces = TRIED_FACES;
        let mut chosen = NORTH;
        for count in (1..=HANG_DIRECTIONS).rev() {
            let pick = random.below(count) as usize;
            if self.water(region, pos + FACES[faces[pick]]) {
                chosen = faces[pick];
                break;
            }
            faces.swap(pick, count as usize - 1);
        }
        if !self.water(region, pos + FACES[chosen]) || !self.submerged(region, at + FACES[chosen]) {
            return false;
        }
        let color = usize::from(random.below(2) > 0);
        region.set(pos + FACES[chosen], self.wall_fans[color][WALL_DIRECTION[chosen - NORTH] as usize]);
        true
    }
}
