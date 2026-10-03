use glam::IVec3;

use super::super::blocks::{AIR, BlockId};
use super::super::random::RandomSource;
use super::super::tags::Tag;
use super::Context;
use super::multiface::{DEFAULT_ORDER, MultifaceStates, SAME_POSITION, SculkBlocks, Spreader};
use super::predicate::{Direction, fluid_at};
use super::region::Region;
use super::tree::shuffle;
use chorus_block::block_id::WATER;

const GROWTH_SPAWN_COST: i32 = 50;
const NO_GROWTH_RADIUS: i32 = 1;
const CHARGE_DECAY_RATE: i32 = 5;
const ADDITIONAL_DECAY_RATE: i32 = 10;
const MAX_CHARGE: i32 = 1000;
const MAX_CURSORS: usize = 32;
const MAX_CURSOR_DISTANCE: i32 = 1024;
const MAX_GROWTH_RATE_RADIUS: i32 = 24;
const SHRIEKER_PLACEMENT_RATE: i32 = 11;

pub struct SculkPatch {
    pub vein: MultifaceStates,
    pub blocks: SculkBlocks,
    pub sculk: BlockId,
    pub water: BlockId,
    pub sensor: BlockId,
    pub shrieker: BlockId,
    pub inhibitors: Vec<u16>,
    pub charge_count: i32,
    pub amount_per_charge: i32,
    pub spread_attempts: i32,
    pub growth_rounds: i32,
    pub spread_rounds: i32,
}

struct Cursor {
    pos: IVec3,
    charge: i32,
    update_delay: i32,
    decay_delay: i32,
    facings: Option<Vec<Direction>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Behaviour {
    Default,
    Sculk,
    Vein,
}

fn non_corner_neighbours() -> Vec<IVec3> {
    let mut offsets = Vec::with_capacity(18);
    for z in -1..=1 {
        for y in -1..=1 {
            for x in -1..=1 {
                let offset = IVec3::new(x, y, z);
                if (x == 0 || y == 0 || z == 0) && offset != IVec3::ZERO {
                    offsets.push(offset);
                }
            }
        }
    }
    offsets
}

fn full(region: &Region, pos: IVec3) -> bool {
    region.blocks.entry(region.get(pos)).full
}

impl SculkPatch {
    fn vein_spreader(&self) -> Spreader<'_> {
        Spreader {
            states: &self.vein,
            types: DEFAULT_ORDER,
            sculk: Some(&self.blocks),
            post_process: true,
        }
    }

    fn same_space_spreader(&self) -> Spreader<'_> {
        Spreader {
            states: &self.vein,
            types: SAME_POSITION,
            sculk: Some(&self.blocks),
            post_process: true,
        }
    }

    fn behaviour(&self, region: &Region, state: BlockId) -> Behaviour {
        let kind = region.blocks.entry(state).kind;
        if kind == self.blocks.sculk {
            Behaviour::Sculk
        } else if kind == self.vein.kind {
            Behaviour::Vein
        } else {
            Behaviour::Default
        }
    }

    fn can_spread_from(&self, region: &Region, origin: IVec3) -> bool {
        let start = region.get(origin);
        if self.behaviour(region, start) != Behaviour::Default {
            return true;
        }
        let blocks = region.blocks;
        if !blocks.is_air(start) && blocks.name_of(start) != WATER {
            return false;
        }
        Direction::ALL.iter().any(|d| full(region, origin + d.offset()))
    }

    pub fn place(&self, ctx: &mut Context, random: &mut dyn RandomSource, origin: IVec3) -> bool {
        let region = &mut *ctx.region;
        if !self.can_spread_from(region, origin) {
            return false;
        }
        let mut cursors: Vec<Cursor> = Vec::new();
        for round in 0..self.spread_rounds + self.growth_rounds {
            for _ in 0..self.charge_count {
                let mut charge = self.amount_per_charge;
                while charge > 0 {
                    let current = charge.min(MAX_CHARGE);
                    if cursors.len() < MAX_CURSORS {
                        cursors.push(Cursor {
                            pos: origin,
                            charge: current,
                            update_delay: 0,
                            decay_delay: 1,
                            facings: None,
                        });
                    }
                    charge -= current;
                }
            }
            let spread_veins = round < self.spread_rounds;
            for _ in 0..self.spread_attempts {
                let mut kept = Vec::with_capacity(cursors.len());
                for mut cursor in cursors {
                    if (cursor.pos - origin).abs().max_element() > MAX_CURSOR_DISTANCE {
                        continue;
                    }
                    self.update(region, &mut cursor, origin, random, spread_veins);
                    if cursor.charge > 0 {
                        kept.push(cursor);
                    }
                }
                cursors = kept;
            }
            cursors.clear();
        }
        true
    }

    fn update(&self, region: &mut Region, cursor: &mut Cursor, origin: IVec3, random: &mut dyn RandomSource, spread_veins: bool) {
        if cursor.charge <= 0 {
            return;
        }
        if cursor.update_delay > 0 {
            cursor.update_delay -= 1;
            return;
        }
        let mut state = region.get(cursor.pos);
        let mut behaviour = self.behaviour(region, state);
        if spread_veins && self.attempt_spread_vein(region, behaviour, cursor.pos, state, cursor.facings.as_deref()) && behaviour != Behaviour::Sculk {
            state = region.get(cursor.pos);
            behaviour = self.behaviour(region, state);
        }
        cursor.charge = self.attempt_use_charge(region, behaviour, cursor, origin, random, spread_veins);
        if cursor.charge <= 0 {
            self.on_discharged(region, behaviour, state, cursor.pos);
            return;
        }
        match self.movement_pos(region, cursor.pos, random, origin) {
            Some(target) => {
                self.on_discharged(region, behaviour, state, cursor.pos);
                cursor.pos = target;
                state = region.get(target);
            }
            None => {
                self.on_discharged(region, behaviour, state, cursor.pos);
                cursor.charge = 0;
                return;
            }
        }
        match self.behaviour(region, state) {
            Behaviour::Vein => cursor.facings = Some(self.vein.faces(state)),
            Behaviour::Sculk => cursor.facings = Some(Vec::new()),
            Behaviour::Default => {}
        }
        cursor.decay_delay = if behaviour == Behaviour::Default { (cursor.decay_delay - 1).max(0) } else { 1 };
        cursor.update_delay = 1;
    }

    fn attempt_spread_vein(&self, region: &mut Region, behaviour: Behaviour, pos: IVec3, state: BlockId, facings: Option<&[Direction]>) -> bool {
        if behaviour == Behaviour::Default {
            match facings {
                None => {
                    let current = region.get(pos);
                    return self.same_space_spreader().spread_all(region, current, pos) > 0;
                }
                Some(faces) if !faces.is_empty() => {
                    let blocks = region.blocks;
                    if !blocks.is_air(state) && blocks.name_of(state) != WATER {
                        return false;
                    }
                    return self.regrow(region, pos, state, faces);
                }
                Some(_) => {}
            }
        }
        self.vein_spreader().spread_all(region, state, pos) > 0
    }

    fn regrow(&self, region: &mut Region, pos: IVec3, existing: BlockId, faces: &[Direction]) -> bool {
        let mut state = self.vein.state(0);
        let mut any = false;
        for &face in faces {
            if full(region, pos + face.offset()) {
                state = self.vein.with_face(state, face, true);
                any = true;
            }
        }
        if !any {
            return false;
        }
        if fluid_at(region.blocks, existing).is_some() {
            state = self.vein.state(self.vein.mask(state).unwrap_or(0) | 1 << 6);
        }
        region.set(pos, state);
        true
    }

    fn attempt_use_charge(&self, region: &mut Region, behaviour: Behaviour, cursor: &Cursor, origin: IVec3, random: &mut dyn RandomSource, spread_veins: bool) -> i32 {
        let charge = cursor.charge;
        match behaviour {
            Behaviour::Default => {
                if cursor.decay_delay > 0 {
                    charge
                } else {
                    0
                }
            }
            Behaviour::Vein => {
                if spread_veins && self.attempt_place_sculk(region, cursor.pos, random) {
                    charge - 1
                } else if random.next_int_bounded(CHARGE_DECAY_RATE) == 0 {
                    (charge as f32 * 0.5).floor() as i32
                } else {
                    charge
                }
            }
            Behaviour::Sculk => {
                if charge == 0 || random.next_int_bounded(CHARGE_DECAY_RATE) != 0 {
                    return charge;
                }
                let pos = cursor.pos;
                let close_to_catalyst = (pos - origin).length_squared() < NO_GROWTH_RADIUS * NO_GROWTH_RADIUS;
                if !close_to_catalyst && self.can_place_growth(region, pos) {
                    if random.next_int_bounded(GROWTH_SPAWN_COST) < charge {
                        let growth = pos + IVec3::Y;
                        let mut state = if random.next_int_bounded(SHRIEKER_PLACEMENT_RATE) == 0 { self.shrieker } else { self.sensor };
                        if fluid_at(region.blocks, region.get(growth)).is_some() {
                            state = region.blocks.flooded(state);
                        }
                        region.set(growth, state);
                    }
                    (charge - GROWTH_SPAWN_COST).max(0)
                } else if random.next_int_bounded(ADDITIONAL_DECAY_RATE) != 0 {
                    charge
                } else {
                    charge - if close_to_catalyst { 1 } else { decay_penalty(pos, origin, charge) }
                }
            }
        }
    }

    fn can_place_growth(&self, region: &Region, pos: IVec3) -> bool {
        let above = region.get(pos + IVec3::Y);
        let blocks = region.blocks;
        if !blocks.is_air(above) && blocks.name_of(above) != WATER {
            return false;
        }
        let mut matched = 0;
        for x in -4..=4 {
            for y in 0..=2 {
                for z in -4..=4 {
                    if self.inhibitors.contains(&blocks.entry(region.get(pos + IVec3::new(x, y, z))).kind) {
                        matched += 1;
                        if matched > 2 {
                            return false;
                        }
                    }
                }
            }
        }
        true
    }

    fn attempt_place_sculk(&self, region: &mut Region, pos: IVec3, random: &mut dyn RandomSource) -> bool {
        let state = region.get(pos);
        let mut supports = Direction::ALL;
        shuffle(&mut supports, random);
        for support in supports {
            if !self.vein.has_face(state, support) {
                continue;
            }
            let support_pos = pos + support.offset();
            if !region.blocks.is(region.get(support_pos), Tag::SculkReplaceableWorldGen) {
                continue;
            }
            region.set(support_pos, self.sculk);
            self.vein_spreader().spread_all(region, self.sculk, support_pos);
            let skip = support.opposite();
            for direction in Direction::ALL {
                if direction == skip {
                    continue;
                }
                let vein_pos = support_pos + direction.offset();
                let vein = region.get(vein_pos);
                if self.vein.is(region.blocks, vein) {
                    self.on_discharged(region, Behaviour::Vein, vein, vein_pos);
                }
            }
            return true;
        }
        false
    }

    fn on_discharged(&self, region: &mut Region, behaviour: Behaviour, state: BlockId, pos: IVec3) {
        if behaviour != Behaviour::Vein || !self.vein.is(region.blocks, state) {
            return;
        }
        let mut state = state;
        for direction in Direction::ALL {
            if self.vein.has_face(state, direction) && region.blocks.entry(region.get(pos + direction.offset())).kind == self.blocks.sculk {
                state = self.vein.with_face(state, direction, false);
            }
        }
        if !self.vein.has_any_face(state) {
            state = if fluid_at(region.blocks, region.get(pos)).is_some() { self.water } else { AIR };
        }
        region.set(pos, state);
    }

    fn movement_pos(&self, region: &Region, pos: IVec3, random: &mut dyn RandomSource, origin: IVec3) -> Option<IVec3> {
        let mut offsets = non_corner_neighbours();
        shuffle(&mut offsets, random);
        let mut chosen = pos;
        for offset in offsets {
            let neighbour = pos + offset;
            let (dx, dz) = (origin.x - neighbour.x, origin.z - neighbour.z);
            if dx * dx + dz * dz > 144 {
                continue;
            }
            let state = region.get(neighbour);
            if self.behaviour(region, state) != Behaviour::Default && unobstructed(region, pos, neighbour) {
                chosen = neighbour;
                if self.has_substrate_access(region, state, neighbour) {
                    break;
                }
            }
        }
        (chosen != pos).then_some(chosen)
    }

    fn has_substrate_access(&self, region: &Region, state: BlockId, pos: IVec3) -> bool {
        self.vein.is(region.blocks, state)
            && Direction::ALL
                .iter()
                .any(|&d| self.vein.has_face(state, d) && region.blocks.is(region.get(pos + d.offset()), Tag::SculkReplaceable))
    }
}

fn decay_penalty(pos: IVec3, origin: IVec3, charge: i32) -> i32 {
    let distance = ((pos - origin).length_squared() as f64).sqrt() as f32 - NO_GROWTH_RADIUS as f32;
    let outer = distance * distance;
    let reach = ((MAX_GROWTH_RATE_RADIUS - NO_GROWTH_RADIUS) * (MAX_GROWTH_RATE_RADIUS - NO_GROWTH_RADIUS)) as f32;
    let factor = (outer / reach).min(1.0);
    1.max((charge as f32 * factor * 0.5) as i32)
}

fn sturdy_toward(region: &Region, from: IVec3, direction: Direction) -> bool {
    full(region, from + direction.offset())
}

fn unobstructed(region: &Region, from: IVec3, to: IVec3) -> bool {
    let delta = to - from;
    if delta.abs().element_sum() == 1 {
        return true;
    }
    let x = if delta.x < 0 { Direction::West } else { Direction::East };
    let y = if delta.y < 0 { Direction::Down } else { Direction::Up };
    let z = if delta.z < 0 { Direction::North } else { Direction::South };
    let open = |direction| !sturdy_toward(region, from, direction);
    if delta.x == 0 {
        open(y) || open(z)
    } else if delta.y == 0 {
        open(x) || open(z)
    } else {
        open(x) || open(y)
    }
}
