use super::super::super::world::read;
use super::super::multiface::SPREAD_ORDER;
use super::super::support::is_wet;
use super::{Behaviour, Cursor, Sculk};
use crate::shared::chunk::region::Region;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::survival::{Direction, multiface_bit};
use chorus_block::block_id::*;
use glam::IVec3;

const GROWTH_SPAWN_COST: i32 = 50;

const CHARGE_DECAY_RATE: i32 = 5;

const ADDITIONAL_DECAY_RATE: i32 = 10;

const SHRIEKER_ODDS: i32 = 11;

const GROWTH_BOX_RADIUS: i32 = 4;

const GROWTH_BOX_HEIGHT: i32 = 2;

const GROWTH_LIMIT: i32 = 3;

fn shuffled_faces(random: &mut MersenneTwister) -> [Direction; 6] {
    let mut faces = [1usize, 0, 2, 3, 4, 5];
    if random.next_u32() & 1 == 0 {
        faces[0] = 0;
        faces[1] = 1;
    }
    for index in 2..6 {
        let pick = (random.next_u32() % (index as u32 + 1)) as usize;
        if pick != index {
            faces[index] = faces[pick];
            faces[pick] = index;
        }
    }
    faces.map(|face| Direction::ALL[face])
}

impl Sculk {
    pub(super) fn use_charge(&self, region: &mut Region, behaviour: Behaviour, cursor: &Cursor, origin: IVec3, random: &mut MersenneTwister, spread_veins: bool) -> i32 {
        match behaviour {
            Behaviour::Default => {
                if cursor.decay_delay > 0 {
                    cursor.charge
                } else {
                    0
                }
            }
            Behaviour::Sculk => self.sculk_charge(region, cursor, origin, random),
            Behaviour::Vein => self.vein_charge(region, cursor, random, spread_veins),
        }
    }

    fn sculk_charge(&self, region: &mut Region, cursor: &Cursor, origin: IVec3, random: &mut MersenneTwister) -> i32 {
        let charge = cursor.charge;
        if random.next_int_bounded(CHARGE_DECAY_RATE) != 0 {
            return charge;
        }
        let distance = (cursor.pos - origin).length_squared() as f32;
        let far = 1.0 <= distance;
        if far && self.can_place_growth(region, cursor.pos) {
            if random.next_int_bounded(GROWTH_SPAWN_COST) < charge {
                let growth = if random.next_int_bounded(SHRIEKER_ODDS) == 0 { self.shrieker } else { self.sensor };
                region.set(cursor.pos + IVec3::Y, growth);
            }
            return (charge - GROWTH_SPAWN_COST).max(0);
        }
        if random.next_int_bounded(ADDITIONAL_DECAY_RATE) != 0 {
            return charge;
        }
        let penalty = if far {
            let fraction = ((distance.sqrt() - 4.0).powi(2) / 400.0).min(1.0);
            ((charge as f32 * fraction * 0.5) as i32).max(1)
        } else {
            1
        };
        charge - penalty
    }

    fn can_place_growth(&self, region: &Region, at: IVec3) -> bool {
        let blocks = region.blocks;
        let above = read(region, at + IVec3::Y);
        if !blocks.is_air(above) && !(is_wet(blocks, above) && blocks.name_of(above) == WATER) {
            return false;
        }
        let mut growths = 0;
        for dx in -GROWTH_BOX_RADIUS..=GROWTH_BOX_RADIUS {
            for dy in 0..=GROWTH_BOX_HEIGHT {
                for dz in -GROWTH_BOX_RADIUS..=GROWTH_BOX_RADIUS {
                    let block = read(region, at + IVec3::new(dx, dy, dz));
                    if self.sensor_kinds.iter().any(|&kind| blocks.is_kind(block, kind)) {
                        growths += 1;
                    }
                }
            }
        }
        growths < GROWTH_LIMIT
    }

    fn vein_charge(&self, region: &mut Region, cursor: &Cursor, random: &mut MersenneTwister, spread_veins: bool) -> i32 {
        if spread_veins {
            let state = read(region, cursor.pos);
            let target = shuffled_faces(random)
                .into_iter()
                .find(|&face| self.veins.holds_face(region, state, face) && self.replaceable.contains(region.blocks, read(region, cursor.pos + face.offset())));
            if let Some(face) = target {
                let sculk = cursor.pos + face.offset();
                region.set(sculk, self.sculk);
                for direction in SPREAD_ORDER.into_iter().filter(|&direction| direction != face.opposite()) {
                    self.discharge_vein(region, sculk + direction.offset());
                }
                self.veins.spread_all(region, self.sculk, sculk, false);
                return cursor.charge - 1;
            }
        }
        if random.next_int_bounded(CHARGE_DECAY_RATE) == 0 {
            (cursor.charge as f32 * 0.5).floor() as i32
        } else {
            cursor.charge
        }
    }

    pub(super) fn on_discharged(&self, region: &mut Region, behaviour: Behaviour, at: IVec3) {
        if behaviour == Behaviour::Vein {
            self.discharge_vein(region, at);
        }
    }

    fn discharge_vein(&self, region: &mut Region, at: IVec3) {
        let state = read(region, at);
        if !self.veins.is_vein(region, state) {
            return;
        }
        let wet = is_wet(region.blocks, state);
        let mut kept = self.veins.face_bits(region, state);
        for face in SPREAD_ORDER {
            let beside = read(region, at + face.offset());
            if self.veins.holds_face(region, state, face) && region.blocks.is_kind(beside, self.sculk_kind) {
                kept &= !multiface_bit(face);
                let block = match (kept, wet) {
                    (0, true) => region.water(),
                    (0, false) => 0,
                    (bits, true) => region.blocks.flooded(self.veins.with_faces(bits)),
                    (bits, false) => self.veins.with_faces(bits),
                };
                region.set(at, block);
            }
        }
    }
}
