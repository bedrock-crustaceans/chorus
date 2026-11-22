use super::super::ore::Draw;
use super::super::world::{is_protected, read};
use super::Geode;
use super::noise::{GeodeNoise, inverse_sqrt};
use crate::shared::blocks::{AIR, BlockId};
use crate::shared::chunk::region::Region;
use crate::shared::survival::Direction;
use chorus_block::block_id::*;
use glam::IVec3;

const FILLING_BITS: u32 = 0x3f44_57d1;

const PLACEMENT_ORDER: [usize; 6] = [1, 0, 2, 3, 4, 5];

pub(super) fn sample<D: Draw>(random: &mut D, min: i32, max: i32) -> i32 {
    if max > min { min + random.below(max - min) } else { min }
}

pub(super) fn set_changed(region: &mut Region, pos: IVec3, block: BlockId) -> bool {
    if is_protected(region, pos) || read(region, pos) == block {
        return false;
    }
    region.set(pos, block)
}

pub(super) fn place_bud(region: &mut Region, pos: IVec3, faces: &[BlockId; 6]) -> bool {
    for index in PLACEMENT_ORDER {
        let target = pos + Direction::ALL[index].offset();
        let there = read(region, target);
        if region.blocks.is_air(there) || region.blocks.entry(there).name.as_ref() == WATER {
            return region.blocks.entry(read(region, pos)).solid && set_changed(region, target, faces[index]);
        }
    }
    false
}

fn squared_distance(a: IVec3, b: IVec3) -> i32 {
    let delta = a - b;
    delta.z * delta.z + delta.y * delta.y + delta.x * delta.x
}

pub(super) struct Scan<'g, 'r, 'w, D> {
    pub(super) geode: &'g Geode,
    pub(super) region: &'r mut Region<'w>,
    pub(super) random: &'r mut D,
    pub(super) noise: &'g GeodeNoise,
    pub(super) origin: IVec3,
    pub(super) points: Vec<(IVec3, i32)>,
    pub(super) cracks: Vec<IVec3>,
    pub(super) crack: bool,
    pub(super) crack_threshold: f32,
    pub(super) layers: [f32; 3],
    pub(super) buffered: Vec<(IVec3, BlockId)>,
    pub(super) potential: Vec<IVec3>,
    pub(super) placed: bool,
}

impl<D: Draw> Scan<'_, '_, '_, D> {
    fn row(&mut self, x: i32, y: i32, found: &mut bool, flag: &mut bool) -> bool {
        let radius = self.geode.max_radius;
        let mut placed_row = false;
        for z in self.origin.z - radius..=self.origin.z + radius {
            if self.cell(IVec3::new(x, y, z)) && self.placed {
                *flag = true;
                placed_row = true;
            }
        }
        if placed_row || *flag {
            *found = true;
        }
        !placed_row && *flag
    }

    fn cell(&mut self, pos: IVec3) -> bool {
        let geode = self.geode;
        let noise = self.noise.sample(pos.x as f32, pos.y as f32, pos.z as f32) * geode.noise_multiplier;
        let sum = self
            .points
            .iter()
            .fold(0.0f32, |sum, &(point, offset)| sum + inverse_sqrt((offset + squared_distance(pos, point)) as f32));
        let field = self.points.len() as f32 * noise + sum;
        let [inner, middle, outer] = self.layers;
        if field < outer {
            return false;
        }
        let crack_sum = self
            .cracks
            .iter()
            .fold(0.0f32, |sum, &crack| sum + inverse_sqrt(squared_distance(pos, crack) as f32 + geode.crack_point_offset));
        let filling = f32::from_bits(FILLING_BITS);
        if self.crack && crack_sum + noise * self.cracks.len() as f32 >= self.crack_threshold && field < filling {
            if set_changed(self.region, pos, AIR) {
                self.placed = true;
            }
            return true;
        }
        if field >= filling {
            self.buffered.push((pos, geode.filler));
        } else if field >= inner {
            if geode.alternate_chance <= self.random.float() {
                self.buffered.push((pos, geode.inner));
                if geode.require_alternate {
                    return true;
                }
            } else {
                self.buffered.push((pos, geode.alternate));
            }
            if self.random.float() < geode.placement_chance {
                self.potential.push(pos);
            }
        } else if field >= middle {
            self.buffered.push((pos, geode.middle));
        } else {
            self.buffered.push((pos, geode.outer));
        }
        true
    }

    fn slice(&mut self, x: i32) -> (bool, bool) {
        let (mut found, mut flag) = (false, false);
        let (origin_y, radius) = (self.origin.y, self.geode.max_radius);
        let mut y = origin_y;
        while y <= origin_y + radius && !self.row(x, y, &mut found, &mut flag) {
            y += 1;
        }
        let upward = flag;
        flag = false;
        let mut y = origin_y;
        while y > origin_y - radius {
            y -= 1;
            if self.row(x, y, &mut found, &mut flag) {
                break;
            }
        }
        (found, upward || flag)
    }

    pub(super) fn run(&mut self) {
        let (origin_x, radius) = (self.origin.x, self.geode.max_radius);
        let mut any = false;
        for x in origin_x..=origin_x + radius {
            let (found, some) = self.slice(x);
            any |= some;
            if !found && any {
                break;
            }
        }
        for x in (origin_x - radius..origin_x).rev() {
            let (found, some) = self.slice(x);
            if !found && (any || some) {
                break;
            }
            any |= some;
        }
    }
}
