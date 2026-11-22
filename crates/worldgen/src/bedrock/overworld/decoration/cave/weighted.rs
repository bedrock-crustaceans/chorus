use super::super::placed::Placed;
use crate::shared::chunk::region::Region;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::Probed;
use glam::IVec3;
use std::sync::Arc;

pub fn place(entries: &[(Arc<Placed>, f32)], region: &mut Region, random: &mut MersenneTwister, pos: IVec3) -> Probed<bool> {
    let total = entries.iter().fold(0i32, |total, (_, weight)| (total as f32 + weight) as i32);
    let mut left = if total == 0 { 0.0 } else { random.next_int_bounded(total) as f32 };
    for (feature, weight) in entries {
        left -= weight;
        if left < 0.0 {
            return feature.place(region, random, pos);
        }
    }
    Ok(false)
}
