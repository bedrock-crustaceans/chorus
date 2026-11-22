use super::super::super::world::read;
use super::super::multiface::SPREAD_ORDER;
use super::super::support::holds_face;
use super::tables::NEIGHBOURS;
use super::{Behaviour, FACINGS_EMPTY, FACINGS_UNSET, Sculk};
use crate::shared::blocks::BlockId;
use crate::shared::chunk::region::Region;
use crate::shared::rng::mersenne::MersenneTwister;
use glam::IVec3;

impl Sculk {
    pub(super) fn movement_target(&self, region: &Region, pos: IVec3, random: &mut MersenneTwister) -> Option<IVec3> {
        let mut offsets = NEIGHBOURS;
        for index in 1..offsets.len() {
            offsets.swap(index, (random.next_u32() % (index as u32 + 1)) as usize);
        }
        let mut found = None;
        for offset in offsets {
            let next = pos + offset;
            let block = read(region, next);
            if self.behaviour(region, block) == Behaviour::Default || !self.unobstructed(region, pos, offset) {
                continue;
            }
            found = Some(next);
            if self.veins.is_vein(region, block) && self.substrate_access(region, block, next) {
                break;
            }
        }
        found
    }

    fn unobstructed(&self, region: &Region, pos: IVec3, offset: IVec3) -> bool {
        if offset.x.abs() + offset.y.abs() + offset.z.abs() == 1 {
            return true;
        }
        let walled = |axis: usize, sign: i32| {
            let mut step = IVec3::ZERO;
            step[axis] = sign;
            holds_face(region.blocks, read(region, pos + step))
        };
        let (first, second) = if offset.x == 0 {
            (walled(1, offset.y), walled(2, offset.z))
        } else if offset.y == 0 {
            (walled(0, offset.x), walled(2, offset.z))
        } else {
            (walled(0, offset.x), walled(1, offset.y))
        };
        !(first && second)
    }

    fn substrate_access(&self, region: &Region, vein: BlockId, at: IVec3) -> bool {
        SPREAD_ORDER
            .iter()
            .any(|&face| self.veins.holds_face(region, vein, face) && self.replaceable.contains(region.blocks, read(region, at + face.offset())))
    }

    pub(super) fn attempt_spread_vein(&self, region: &mut Region, behaviour: Behaviour, at: IVec3, state: BlockId, facings: i32) -> bool {
        if behaviour != Behaviour::Default || facings == FACINGS_EMPTY {
            return self.veins.spread_all(region, state, at, false) > 0;
        }
        if facings == FACINGS_UNSET {
            return self.veins.spread_all(region, state, at, true) > 0;
        }
        if !self.veins.replaceable(region, state) || !(1..64).contains(&facings) {
            return false;
        }
        region.set(at, self.veins.with_faces(facings));
        true
    }
}
