use super::site::put;
use super::spec::{AcaciaBranches, AcaciaTrunk};
use super::{Anchor, Tree};
use crate::bedrock::overworld::decoration::ore::Draw;
use crate::shared::chunk::region::Region;
use glam::IVec3;

const LEAN: [IVec3; 4] = [IVec3::new(0, 0, 1), IVec3::new(-1, 0, 0), IVec3::new(0, 0, -1), IVec3::new(1, 0, 0)];
const DIRECTIONS: i32 = 4;

fn edge_mask(dx: i32, dz: i32, width: i32) -> [bool; 4] {
    [dx == 0, dx != 0 && dx == width - 1, dz == 0, dz != 0 && dz == width - 1]
}

impl AcaciaTrunk {
    pub fn place<D: Draw>(&self, tree: &Tree, region: &mut Region, random: &mut D, origin: IVec3) -> Option<IVec3> {
        let height = self.height_base + self.height_intervals.iter().map(|&interval| random.below(interval)).sum::<i32>();
        let origin = tree.roots_then(region, random, origin, height)?;
        if height < 1 || !tree.prepare(region, origin, height, 1) {
            return None;
        }
        for dx in 0..self.width {
            for dz in 0..self.width {
                tree.place_base_block(region, origin + IVec3::new(dx, -1, dz));
            }
        }
        let direction = random.below(DIRECTIONS) as usize;
        let lean_start = height - self.lean_height.draw(random);
        let mut lean_steps = self.lean_steps.draw(random);
        let lean_length = self.lean_length.draw(random);
        let mut column = origin;
        let mut candidates = Vec::new();
        for index in 0..height + lean_length {
            if index >= lean_start && lean_steps > 0 {
                lean_steps -= 1;
                column += LEAN[direction];
            }
            let layer = IVec3::new(column.x, origin.y + index.min(height - 1), column.z);
            if !tree.valid_position(region, layer) {
                continue;
            }
            let base = if self.allow_diagonal { layer } else { IVec3::new(origin.x, origin.y + index, origin.z) };
            for dx in 0..self.width {
                for dz in 0..self.width {
                    let pos = base + IVec3::new(dx, 0, dz);
                    if !tree.valid_position(region, pos) {
                        continue;
                    }
                    put(region, pos, self.block);
                    if let Some(decoration) = &self.decoration {
                        decoration.place(region, &tree.foliage, random, pos, edge_mask(dx, dz, self.width));
                    }
                    if index >= self.min_height_for_canopy {
                        candidates.push(pos);
                    }
                }
            }
        }
        let anchor = candidates.last().copied().unwrap_or(IVec3::ZERO);
        tree.place_canopies(region, random, &[Anchor { at: anchor, candidates }]);
        if self.allow_diagonal {
            self.branches.leaning(self, tree, region, random, origin, height, direction, lean_start);
        } else {
            self.branches.vertical(self, tree, region, random, origin, anchor);
        }
        Some(origin)
    }
}

impl AcaciaBranches {
    fn leaning<D: Draw>(&self, trunk: &AcaciaTrunk, tree: &Tree, region: &mut Region, random: &mut D, origin: IVec3, height: i32, lean: usize, lean_start: i32) {
        let direction = random.below(DIRECTIONS) as usize;
        if direction == lean || !self.chance.roll(random) {
            return;
        }
        let position = self.position.draw(random);
        let mut length = self.length.draw(random);
        let mut index = lean_start - position;
        if index >= height || length < 1 {
            return;
        }
        let mut column = origin;
        let mut last = None;
        loop {
            if index >= 1 {
                column += LEAN[direction];
                let pos = IVec3::new(column.x, origin.y + index, column.z);
                if tree.valid_position(region, pos) {
                    put(region, pos, trunk.block);
                    if let Some(decoration) = &trunk.decoration {
                        decoration.place(region, &tree.foliage, random, pos, [true; 4]);
                    }
                    last = Some(pos);
                }
            }
            index += 1;
            if index >= height || length <= 1 {
                break;
            }
            length -= 1;
        }
        if let (Some(pos), Some(canopy)) = (last, &self.canopy) {
            canopy.place(tree, region, random, &Anchor::at(IVec3::new(column.x, pos.y, column.z)));
        }
    }

    fn vertical<D: Draw>(&self, trunk: &AcaciaTrunk, tree: &Tree, region: &mut Region, random: &mut D, origin: IVec3, anchor: IVec3) {
        let width = trunk.width;
        for dx in -1..=width {
            for dz in -1..=width {
                let ring = dx < 0 || dx >= width || dz < 0 || dz >= width;
                if !ring || !self.chance.roll(random) {
                    continue;
                }
                let length = self.length.draw(random);
                let position = self.position.draw(random);
                for step in 0..length.max(0) {
                    let pos = IVec3::new(origin.x + dx, anchor.y - position - step, origin.z + dz);
                    if tree.valid_position(region, pos) {
                        put(region, pos, trunk.block);
                    }
                }
                if let Some(canopy) = &self.canopy {
                    canopy.place(tree, region, random, &Anchor::at(IVec3::new(anchor.x + dx, anchor.y - position, anchor.z + dz)));
                }
            }
        }
    }
}
