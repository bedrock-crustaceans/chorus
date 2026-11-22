use super::super::aquatic::is_solid_blocking;
use super::super::ore::Draw;
use super::super::world::read;
use super::kinds::Coral;
use super::{CLOCKWISE_TURN, CORAL_COLORS, DOWN, EAST, FACES, FAN_DIRECTIONS, NORTH, ROTATION_COUNT, SOUTH, WALL_DIRECTION, WEST, turn};
use crate::shared::blocks::BlockId;
use crate::shared::chunk::region::Region;
use glam::IVec3;

const UP: usize = 1;

const OPPOSITE: [usize; 6] = [UP, DOWN, SOUTH, NORTH, EAST, WEST];

const BEYOND_FACE: [[usize; 3]; 4] = [[EAST, SOUTH, WEST], [EAST, NORTH, WEST], [EAST, SOUTH, NORTH], [NORTH, SOUTH, WEST]];

const SIDE_BOUND: i32 = 4;

const STAR_BASE_CHANCE: f32 = 0.5;

const STAR_OFFSET_CHANCE: f32 = 0.1;

const STAR_LAYER_CHANCE: f32 = 0.15;

const DIAGONAL_CHANCE: f32 = 0.25;

const SIDE_CHANCE: f32 = 0.4;

const TOP_CHANCE: f32 = 0.2;

const MOUND_TOP_CHANCE: f32 = 0.4;

const PILLAR_TOP_CHANCE: f32 = 0.2;

const PILLAR_TALL_CHANCE: f32 = 0.25;

const PILLAR_TIP_CHANCE: f32 = 0.1;

const RUN_TOP_CHANCE: f32 = 0.1;

const RUN_SIDE_CHANCE: f32 = 0.6;

const RUN_RISE_CHANCE: f32 = 0.25;

const MOUND_LOW_CHANCE: f32 = 0.33;

const MOUND_HIGH_CHANCE: f32 = 0.7;

const TALL_BRANCH_HEIGHT: i32 = 2;

const LADDER_RUN_LENGTH: i32 = 4;

const LADDER_STEM_RUN: i32 = 2;

fn third_face(first: usize, second: usize) -> [usize; 2] {
    match (first.min(second), first.max(second)) {
        (NORTH, EAST) => [SOUTH, WEST],
        (NORTH, SOUTH) => [EAST, WEST],
        (NORTH, WEST) => [SOUTH, EAST],
        (SOUTH, EAST) => [NORTH, WEST],
        (WEST, EAST) => [SOUTH, NORTH],
        _ => [NORTH, EAST],
    }
}

pub(super) struct Reef<'r, 'b, D: Draw> {
    coral: &'r Coral,
    region: &'r mut Region<'b>,
    random: &'r mut D,
    tops: Vec<IVec3>,
    sides: Vec<(IVec3, usize)>,
}

impl<'r, 'b, D: Draw> Reef<'r, 'b, D> {
    pub(super) fn new(coral: &'r Coral, region: &'r mut Region<'b>, random: &'r mut D) -> Self {
        Self {
            coral,
            region,
            random,
            tops: Vec::new(),
            sides: Vec::new(),
        }
    }

    fn set(&mut self, pos: IVec3, block: BlockId) -> bool {
        self.region.set(pos, block)
    }

    fn try_set(&mut self, pos: IVec3, color: usize) -> bool {
        if !self.coral.can_overwrite(self.region, pos, color) {
            return false;
        }
        self.region.set(pos, self.coral.blocks[color]);
        true
    }

    fn solid(&self, pos: IVec3) -> bool {
        is_solid_blocking(self.region, pos)
    }

    fn water(&self, pos: IVec3) -> bool {
        self.coral.water(self.region, pos)
    }

    fn fits(&self, pos: IVec3) -> bool {
        self.coral.fits(self.region, pos)
    }

    fn star(&mut self, pos: IVec3, color: usize, chance: f32, reach: i32) {
        if !self.try_set(pos, color) {
            return;
        }
        for offset in [IVec3::new(reach, 0, 0), IVec3::new(-reach, 0, 0), IVec3::new(0, 0, reach), IVec3::new(0, 0, -reach)] {
            if self.random.float() < chance {
                let target = pos + offset;
                if !self.coral.can_overwrite(self.region, target, color) || !self.solid(target - IVec3::Y) {
                    return;
                }
                self.set(target, self.coral.blocks[color]);
            }
        }
    }

    fn diagonals(&mut self, at: IVec3, color: usize, reach: i32) {
        for offset in [IVec3::new(reach, 0, reach), IVec3::new(-reach, 0, reach), IVec3::new(-reach, 0, reach), IVec3::new(reach, 0, -reach)] {
            if self.random.float() < DIAGONAL_CHANCE {
                let target = at + offset;
                if !self.coral.can_overwrite(self.region, target, color) || !self.solid(target - IVec3::Y) {
                    return;
                }
                self.set(target, self.coral.blocks[color]);
            }
        }
    }

    fn side(&mut self, pos: IVec3) {
        let face = self.random.below(SIDE_BOUND) as usize + NORTH;
        if self.random.float() < SIDE_CHANCE {
            self.sides.push((pos, face));
        }
    }

    pub(super) fn place_coral(&mut self, at: IVec3, color: usize) {
        let shape = self.random.below(3);
        if shape == 0 {
            self.branching(at, color);
        }
        if shape <= 1 {
            self.mound(at, color);
        }
        self.pillar(at, color);
    }

    fn branching(&mut self, at: IVec3, color: usize) {
        let height = self.random.below(2) + 1;
        for layer in 0..=height {
            let width = 2 * height + 1 - 2 * layer;
            if !self.fits(at + IVec3::Y * (layer + 1)) || !self.try_set(at, color) {
                return;
            }
            if width < 2 {
                continue;
            }
            let chance = |offset: i32, layer: i32| offset as f32 * STAR_OFFSET_CHANCE + STAR_BASE_CHANCE - layer as f32 * STAR_LAYER_CHANCE;
            if height == TALL_BRANCH_HEIGHT && layer == 0 {
                for offset in 1..=width / 2 {
                    self.star(at, color, chance(offset, 0), 0);
                    self.diagonals(at, color, offset);
                }
            } else {
                let centre = at + IVec3::Y * layer;
                for offset in 1..=width / 2 {
                    self.star(centre, color, chance(offset, layer), layer);
                }
            }
        }
        let base = at + IVec3::Y * height;
        if !self.fits(base + IVec3::Y) {
            return;
        }
        let first = self.random.below(ROTATION_COUNT) as usize + NORTH;
        let second = BEYOND_FACE[first - NORTH][self.random.below(3) as usize];
        let third = third_face(first, second)[self.random.below(2) as usize];
        if !self.try_set(base, color) {
            return;
        }
        self.arm(base + FACES[first], color, first, false);
        let extra = self.random.below(2);
        for (step, face) in (1..=extra + 1).zip([second, third]) {
            let pos = base + IVec3::Y * step;
            if !self.try_set(pos, color) {
                return;
            }
            self.arm(pos + FACES[face], color, face, step == extra + 1);
        }
    }

    fn mound(&mut self, at: IVec3, color: usize) {
        let n = self.random.below(2);
        let side = n + 1;
        let max = 2 * n + 2;
        let corner = at - IVec3::new(side, 0, side);
        for a in 0..=max {
            for b in 0..=max {
                if (a == 0 || a == max) && (b == 0 || b == max) {
                    continue;
                }
                let pos = corner + IVec3::new(a, 0, b);
                if self.random.below(5) != 0 && self.solid(pos - IVec3::Y) && !self.try_set(pos, color) {
                    return;
                }
            }
        }
        self.try_set(corner, color);
        let stem = at + IVec3::Y;
        if !self.fits(stem + IVec3::Y) || !self.try_set(stem, color) {
            return;
        }
        for (dx, dz, bound) in [(1, 0, 2), (-1, 0, 3), (0, 1, 4), (0, -1, 3)] {
            if self.random.below(bound) != 0 && !self.try_set(at + IVec3::new(dx, 1, dz), color) {
                return;
            }
        }
        self.star(at + IVec3::Y * 2, color, MOUND_LOW_CHANCE, 1);
        self.star(at + IVec3::Y * 3, color, MOUND_HIGH_CHANCE, 1);
        let top = at + IVec3::Y * 4;
        let tip = self.random.below(2);
        for height in 0..=tip {
            if !self.fits(top + IVec3::Y * (height + 1)) || !self.try_set(top + IVec3::Y * height, color) {
                return;
            }
            self.side(top);
        }
        if self.random.float() < MOUND_TOP_CHANCE {
            self.tops.push(top + IVec3::Y * (tip + 1));
        }
    }

    fn pillar(&mut self, at: IVec3, color: usize) {
        if self.random.below(7) == 0 {
            self.ladders(at, color);
            return;
        }
        for a in 0..3 {
            for b in 0..3 {
                let pos = at + IVec3::new(a - 1, 0, b - 1);
                if self.random.below(3) != 0 && !self.water(pos - IVec3::Y) && !self.try_set(pos, color) {
                    return;
                }
            }
        }
        let stem = at + IVec3::Y;
        if !self.fits(stem + IVec3::Y) || !self.try_set(stem, color) {
            return;
        }
        for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let pos = at + IVec3::new(dx, 1, dz);
            if self.random.below(4) != 0 && !self.water(pos - IVec3::Y) {
                if !self.try_set(pos, color) {
                    return;
                }
                if self.random.float() < PILLAR_TOP_CHANCE {
                    self.tops.push(pos + IVec3::Y);
                }
            }
        }
        if self.fits(at + IVec3::Y * 3) {
            if !self.try_set(at + IVec3::Y * 2, color) {
                return;
            }
            if self.random.float() < PILLAR_TOP_CHANCE {
                self.tops.push(at + IVec3::Y * 3);
            }
        }
        if self.random.float() >= PILLAR_TALL_CHANCE || !self.fits(at + IVec3::Y * 4) || !self.try_set(at + IVec3::Y * 3, color) {
            return;
        }
        if self.random.float() < PILLAR_TIP_CHANCE {
            self.tops.push(at + IVec3::Y * 4);
        }
    }

    fn ladders(&mut self, at: IVec3, color: usize) {
        let choice = self.random.below(ROTATION_COUNT) as usize;
        let side = CLOCKWISE_TURN[choice];
        let other = OPPOSITE[side];
        let wide = self.random.below(2) != 0;
        let height = self.random.below(3);
        if !self.fits(at + IVec3::Y * (height + 3)) {
            return;
        }
        for level in 0..=height {
            if !self.try_set(at + IVec3::Y * level, color) {
                return;
            }
        }
        let centre = at + IVec3::Y * (height + 1);
        let left = centre + FACES[side];
        let right = centre + FACES[other];
        let mut cells: Vec<IVec3> = Vec::with_capacity(3);
        if !wide {
            cells.extend([left, right]);
        } else if !self.random.boolean() {
            cells.extend([right, left, right + FACES[other]]);
        } else {
            cells.extend([left, left + FACES[side], right]);
        }
        for &pos in &cells {
            if !self.try_set(pos, color) {
                return;
            }
        }
        let left_start = cells[cells.len() - 2];
        let right_start = cells[cells.len() - 1];
        let run = choice + NORTH;
        let starts = [centre + FACES[run], left_start + FACES[run] + IVec3::Y, right_start + FACES[run] + IVec3::Y];
        for (index, start) in starts.into_iter().enumerate() {
            let length = self.random.below(2) + if index != 0 { LADDER_RUN_LENGTH } else { LADDER_STEM_RUN };
            let mut pos = start;
            for _ in 0..length {
                if !self.try_set(pos, color) {
                    return;
                }
                if self.random.float() < RUN_TOP_CHANCE {
                    self.tops.push(pos + IVec3::Y);
                }
                for face in [side, other] {
                    if self.random.float() < RUN_SIDE_CHANCE {
                        self.sides.push((pos, face));
                    }
                }
                let rise = self.random.float() < RUN_RISE_CHANCE && self.fits(pos + IVec3::Y * 2);
                pos += FACES[run] + if rise { IVec3::Y } else { IVec3::ZERO };
            }
        }
    }

    fn arm_cell(&mut self, pos: IVec3, color: usize) -> bool {
        self.fits(pos + IVec3::Y) && self.try_set(pos, color)
    }

    fn arm(&mut self, at: IVec3, color: usize, face: usize, short: bool) {
        let kind = if short { self.random.below(2) + 1 } else { self.random.below(3) };
        match kind {
            0 => {
                if !self.arm_cell(at, color) {
                    return;
                }
                let c = at + FACES[face];
                if !self.arm_cell(c + IVec3::Y, color) {
                    return;
                }
                let c = c + FACES[face] + IVec3::Y * 2;
                if !self.arm_cell(c, color) {
                    return;
                }
                self.side(c);
                let c = c + IVec3::Y;
                if !self.arm_cell(c, color) {
                    return;
                }
                self.side(c);
                let c = c + FACES[OPPOSITE[face]] + IVec3::Y;
                if !self.arm_cell(c, color) || self.random.float() >= TOP_CHANCE {
                    return;
                }
                self.tops.push(c + IVec3::Y);
            }
            1 => {
                if !self.arm_cell(at, color) {
                    return;
                }
                self.side(at);
                if !self.arm_cell(at + IVec3::Y, color) {
                    return;
                }
                self.side(at);
                let c = at + FACES[face] + IVec3::Y;
                for dy in 0..3 {
                    if !self.arm_cell(c + IVec3::Y * dy, color) {
                        return;
                    }
                    self.side(c);
                }
                if self.random.float() < TOP_CHANCE {
                    self.tops.push(c + IVec3::Y * 3);
                }
            }
            _ => {
                if !self.try_set(at, color) {
                    return;
                }
                let mut c = at + FACES[face] + IVec3::Y;
                for turned in [false, true] {
                    if turned {
                        let clockwise = self.random.boolean();
                        c = c + FACES[turn(face, clockwise)] + IVec3::Y;
                    }
                    for dy in 0..2 {
                        if !self.arm_cell(c + IVec3::Y * dy, color) {
                            return;
                        }
                        self.side(c);
                    }
                }
                if self.random.float() < TOP_CHANCE {
                    self.tops.push(c + IVec3::Y * 2);
                }
            }
        }
    }

    pub(super) fn decorate(&mut self) {
        for top in std::mem::take(&mut self.tops) {
            if !self.water(top) || self.water(top - IVec3::Y) || !self.solid(top - IVec3::Y) {
                continue;
            }
            let color = self.random.below(CORAL_COLORS as i32) as usize;
            let direction = self.random.below(FAN_DIRECTIONS as i32) as usize;
            self.set(top, self.coral.fans[color][direction]);
        }
        for (pos, recorded) in std::mem::take(&mut self.sides) {
            let on = read(self.region, pos);
            let on_coral = self.coral.is_coral(self.region, pos) || self.region.blocks.entry(on).kind == self.coral.stone_kind;
            let face = BEYOND_FACE[recorded - NORTH][self.random.below(3) as usize];
            let target = pos + FACES[face];
            if !self.water(target) || !on_coral || !self.coral.submerged(self.region, target) {
                continue;
            }
            let color = (self.random.below(2) + 2 * self.random.below(3)).clamp(0, CORAL_COLORS as i32 - 1) as usize;
            self.set(target, self.coral.wall_fans[color][WALL_DIRECTION[face - NORTH] as usize]);
        }
    }

    pub(super) fn coral_base(&mut self, markers: &[IVec3]) {
        let color = self.random.below(CORAL_COLORS as i32) as usize;
        let coral = self.coral.blocks[color];
        'markers: for &marker in markers {
            if !self.solid(marker - IVec3::Y) {
                continue;
            }
            let mut pos = if self.solid(marker) { marker + IVec3::Y } else { marker };
            if self.random.below(4) == 0 {
                if !self.fits(pos) {
                    continue;
                }
                for face in [EAST, NORTH, WEST, SOUTH] {
                    let target = pos + FACES[face];
                    if !self.coral.is_coral(self.region, target) {
                        self.set(target, coral);
                    }
                }
                self.set(pos, coral);
                if self.set(pos + IVec3::Y, coral) && self.random.below(4) == 0 {
                    self.tops.push(pos + IVec3::Y * 2);
                }
                continue;
            }
            let length = self.random.below(3);
            if !self.water(pos) {
                continue;
            }
            self.set(pos, coral);
            let mut along_z = self.random.below(2) != 0;
            for _ in 0..=length {
                let face = match (along_z, self.random.below(2) == 0) {
                    (false, true) => WEST,
                    (false, false) => EAST,
                    (true, true) => SOUTH,
                    (true, false) => NORTH,
                };
                pos += FACES[OPPOSITE[face]];
                if !self.fits(pos) {
                    break;
                }
                if self.coral.is_coral(self.region, pos) {
                    continue 'markers;
                }
                if !self.set(pos, coral) {
                    break;
                }
                if self.random.below(4) == 0 {
                    self.tops.push(pos + IVec3::Y);
                }
                for bound in [2, 2, 3] {
                    if self.random.below(bound) == 0 {
                        self.sides.push((pos, face));
                    }
                }
                along_z = !along_z;
            }
        }
    }
}
