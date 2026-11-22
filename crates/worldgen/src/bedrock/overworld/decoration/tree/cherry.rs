use super::draw::Range;
use super::site::{block, put};
use super::spec::{CherryCanopy, CherryTrunk};
use super::{Anchor, Tree};
use crate::bedrock::overworld::decoration::ore::Draw;
use crate::shared::blocks::BlockId;
use crate::shared::chunk::region::Region;
use glam::IVec3;

const LEAN_X: [i32; 4] = [0, -1, 0, 1];
const LEAN_Z: [i32; 4] = [1, 0, -1, 0];
const COUNTER_CLOCKWISE: [usize; 4] = [3, 0, 1, 2];
const WIDE_RADIUS: i32 = 3;

fn lean(direction: usize) -> IVec3 {
    IVec3::new(LEAN_X[direction], 0, LEAN_Z[direction])
}

fn log(tree: &Tree, region: &mut Region, pos: IVec3, value: BlockId) {
    if tree.valid_position(region, pos) {
        put(region, pos, value);
    }
}

impl CherryTrunk {
    fn height<D: Draw>(&self, random: &mut D) -> i32 {
        self.height_base + self.height_intervals.iter().map(|&interval| if interval < 0 { 0 } else { random.below(interval + 1) }).sum::<i32>()
    }

    fn tree_type<D: Draw>(&self, random: &mut D) -> i32 {
        let mut pick = random.below(self.weights.iter().sum());
        for (index, &weight) in self.weights.iter().enumerate() {
            if pick < weight {
                return index as i32;
            }
            pick -= weight;
        }
        -1
    }

    fn branch<D: Draw>(&self, tree: &Tree, region: &mut Region, random: &mut D, origin: IVec3, height: i32, direction: usize, start: i32, joins_below_top: bool) -> IVec3 {
        let end_y = height + self.end_offset.draw_inclusive(random) - 1;
        let extra = joins_below_top || end_y < start;
        let length = self.horizontal_length.draw_inclusive(random) + i32::from(extra);
        let step = lean(direction);
        let side = if step.x != 0 { self.log.along_x } else { self.log.along_z };
        let mut current = origin + step + IVec3::Y * start;
        log(tree, region, current, side);
        if extra {
            current += step;
            log(tree, region, current, side);
        }
        let target = IVec3::new(origin.x + step.x * length, origin.y + end_y, origin.z + step.z * length);
        while current != target {
            let dy = target.y - current.y;
            let distance = (target.x - current.x).abs() + dy.abs() + (target.z - current.z).abs();
            if random.float() < dy.abs() as f32 / distance as f32 {
                current.y += dy.signum();
                log(tree, region, current, self.log.upright);
            } else {
                current += step;
                log(tree, region, current, side);
            }
        }
        target + IVec3::Y
    }

    pub fn place<D: Draw>(&self, tree: &Tree, region: &mut Region, random: &mut D, origin: IVec3) -> Option<IVec3> {
        let height = self.height(random);
        let origin = tree.roots_then(region, random, origin, height)?;
        if !tree.prepare(region, origin, height, 1) {
            return None;
        }
        tree.place_base_block(region, origin - IVec3::Y);
        let tree_type = self.tree_type(random);
        let mut start_a = height - 1 + self.start_offset.draw_inclusive(random);
        let mut start_b = (height - 1 + Range::new(self.start_offset.min, self.start_offset.max - 1).draw_inclusive(random)).max(0);
        if start_a <= start_b {
            start_b += 1;
        }
        let direction = random.below(4) as usize;
        start_a = start_a.max(0);
        let stems = match tree_type {
            1 => start_a.max(start_b) + 1,
            2 => height,
            _ => start_a + 1,
        };
        for index in 0..stems {
            log(tree, region, origin + IVec3::Y * index, self.log.upright);
        }
        let mut anchors = Vec::with_capacity(3);
        if tree_type == 2 {
            anchors.push(Anchor::at(origin + IVec3::Y * height));
        }
        anchors.push(Anchor::at(self.branch(tree, region, random, origin, height, direction, start_a, start_a < stems - 1)));
        if tree_type >= 1 {
            anchors.push(Anchor::at(self.branch(tree, region, random, origin, height, (direction + 2) & 3, start_b, start_b < stems - 1)));
        }
        tree.place_canopies(region, random, &anchors);
        Some(origin)
    }
}

fn distance_to_trunk(offset: i32, width: i32) -> i32 {
    if width < 2 { offset.abs() } else { offset.abs().min((offset - (width - 1)).abs()) }
}

impl CherryCanopy {
    fn layer<D: Draw>(&self, tree: &Tree, region: &mut Region, random: &mut D, at: IVec3, dy: i32, radius: i32) {
        for dx in -radius..self.trunk_width + radius {
            let x_distance = distance_to_trunk(dx, self.trunk_width);
            for dz in -radius..self.trunk_width + radius {
                let z_distance = distance_to_trunk(dz, self.trunk_width);
                if dy == -1 && (x_distance == radius || z_distance == radius) && self.wide_bottom_hole.roll(random) {
                    continue;
                }
                let corner = x_distance == radius && z_distance == radius;
                if radius < WIDE_RADIUS {
                    if corner && self.corner_hole.roll(random) {
                        continue;
                    }
                } else if corner || x_distance + z_distance > 2 * radius - 2 && self.corner_hole.roll(random) {
                    continue;
                }
                let pos = at + IVec3::new(dx, dy, dz);
                if tree.may_replace.allows(region.blocks, block(region, pos)) {
                    put(region, pos, self.leaf);
                }
            }
        }
    }

    fn layer_with_hanging<D: Draw>(&self, tree: &Tree, region: &mut Region, random: &mut D, at: IVec3, dy: i32, radius: i32) {
        self.layer(tree, region, random, at, dy, radius);
        let count = self.trunk_width + 2 * radius - 1;
        if count <= 0 {
            return;
        }
        let y = at.y + dy;
        for direction in 0..4 {
            let counter = COUNTER_CLOCKWISE[direction];
            let edge = if counter == 3 || counter == 0 { self.trunk_width + radius - 1 } else { radius };
            let mut pos = IVec3::new(
                at.x + edge * LEAN_X[counter] - radius * LEAN_X[direction],
                y,
                at.z + edge * LEAN_Z[counter] - radius * LEAN_Z[direction],
            );
            for _ in 0..count {
                if block(region, pos) == self.leaf
                    && self.hanging_leaves.roll(random)
                    && tree.may_replace.allows(region.blocks, block(region, pos - IVec3::Y))
                    && put(region, pos - IVec3::Y, self.leaf)
                    && self.hanging_leaves_extension.roll(random)
                    && tree.may_replace.allows(region.blocks, block(region, pos - IVec3::Y * 2))
                {
                    put(region, pos - IVec3::Y * 2, self.leaf);
                }
                pos += lean(direction);
            }
        }
    }

    pub fn place<D: Draw>(&self, tree: &Tree, region: &mut Region, random: &mut D, at: IVec3) {
        let radius = self.radius.draw_inclusive(random);
        let height = self.height.draw_inclusive(random);
        let (first, second, third) = (self.trunk_width + radius - 4, self.trunk_width + radius - 3, self.trunk_width + radius - 2);
        self.layer(tree, region, random, at, height - 3, first);
        self.layer(tree, region, random, at, height - 4, second);
        for dy in (0..=height - 5).rev() {
            self.layer(tree, region, random, at, dy, third);
        }
        self.layer_with_hanging(tree, region, random, at, -1, third);
        self.layer_with_hanging(tree, region, random, at, -2, second);
    }
}
