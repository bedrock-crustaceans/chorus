use super::super::aquatic::is_solid_blocking;
use super::super::ore::Draw;
use super::kinds::Coral;
use super::{CORAL_COLORS, FAN_DIRECTIONS};
use crate::shared::chunk::region::Region;
use glam::IVec3;

const ANEMONE_DEPTH: i32 = 15;

impl Coral {
    pub fn anemone<D: Draw>(&self, region: &mut Region, random: &mut D, at: IVec3) -> bool {
        let ceiling = region.sea_level() - ANEMONE_DEPTH;
        let mut pos = at;
        while !is_solid_blocking(region, pos - IVec3::Y) && pos.y > ceiling {
            pos.y -= 1;
        }
        if !self.water(region, pos) || !is_solid_blocking(region, pos - IVec3::Y) || pos.y <= ceiling || !self.submerged(region, pos) {
            return false;
        }
        let block = if random.below(4) == 0 {
            let color = random.below(CORAL_COLORS as i32) as usize;
            let direction = random.below(FAN_DIRECTIONS as i32) as usize;
            self.fans[color][direction]
        } else {
            self.plants[random.below(CORAL_COLORS as i32) as usize]
        };
        region.set(pos, block);
        true
    }
}
