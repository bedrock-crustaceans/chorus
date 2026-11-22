use super::draw::pick_weighted;
use super::site::{block, put};
use super::spec::{MangroveCanopy, MangroveRoots, MangroveTrunk};
use super::{Anchor, Tree};
use crate::bedrock::overworld::decoration::ore::Draw;
use crate::shared::chunk::region::Region;
use glam::IVec3;
use std::collections::HashSet;

const BRANCH_STEP: [IVec3; 4] = [IVec3::new(0, 0, -1), IVec3::new(0, 0, 1), IVec3::new(-1, 0, 0), IVec3::new(1, 0, 0)];
const ROOT_DIRECTIONS: [IVec3; 4] = [IVec3::new(0, 0, -1), IVec3::new(1, 0, 0), IVec3::new(0, 0, 1), IVec3::new(-1, 0, 0)];
const SEARCH_DEPTH: i32 = 2;
const EXCLUDE_XZ: i32 = 1;
const EXCLUDE_Y: i32 = 1;
const ROOT_DOWN_CHANCE: f32 = 0.2;
const ROOT_WIDTH_MARGIN: i32 = 3;

impl MangroveTrunk {
    pub fn place<D: Draw>(&self, tree: &Tree, region: &mut Region, random: &mut D, origin: IVec3) -> Option<IVec3> {
        let first = random.below(self.height_rand_a + 1);
        let height = self.height_base + first + random.below(self.height_rand_b + 1);
        let origin = tree.roots_then(region, random, origin, height)?;
        let mut candidates = Vec::new();
        let mut anchor = origin;
        for index in 0..height {
            let pos = origin + IVec3::Y * index;
            if tree.valid_position(region, pos) && put(region, pos, self.block) {
                if let Some(decoration) = &self.decoration {
                    decoration.place(region, &tree.foliage, random, pos, [true; 4]);
                }
                if index < height - 1 && random.boolean() {
                    let facing = random.below(4) as usize;
                    let first = self.branch_length.draw_inclusive(random);
                    let second = self.branch_length.draw_inclusive(random);
                    let steps = self.branch_steps.draw_inclusive(random);
                    let start = (first - second - 1).max(0);
                    self.branch(tree, region, random, pos, (facing, start, steps, height), &mut candidates);
                }
            }
            if index == height - 1 {
                anchor = origin + IVec3::Y * (index + 1);
                candidates.push(anchor);
            }
        }
        tree.place_canopies(region, random, &[Anchor { at: anchor, candidates }]);
        Some(origin)
    }

    fn branch<D: Draw>(&self, tree: &Tree, region: &mut Region, random: &mut D, from: IVec3, (facing, start, steps, height): (usize, i32, i32, i32), candidates: &mut Vec<IVec3>) {
        let step = BRANCH_STEP[facing];
        let base = from.y;
        let mut column = from;
        let mut tip = start + base;
        let mut last = from;
        if start < height && steps >= 1 {
            let (mut level, mut remaining) = (start, steps);
            loop {
                if level >= 1 {
                    column += step;
                    let pos = IVec3::new(column.x, level + base, column.z);
                    tip = pos.y;
                    if tree.valid_position(region, pos) && put(region, pos, self.block) {
                        if let Some(decoration) = &self.decoration {
                            decoration.place(region, &tree.foliage, random, pos, [true; 4]);
                        }
                        tip = pos.y + 1;
                    }
                    last = column;
                    candidates.push(pos);
                }
                level += 1;
                if level >= height {
                    break;
                }
                remaining -= 1;
                if remaining < 1 {
                    break;
                }
            }
        }
        if tip - base > 1 {
            candidates.push(IVec3::new(last.x, tip, last.z));
            candidates.push(IVec3::new(last.x, tip - 2, last.z));
        }
    }
}

impl MangroveRoots {
    pub fn place<D: Draw>(&self, tree: &Tree, region: &mut Region, random: &mut D, origin: IVec3, height: i32) -> Option<IVec3> {
        let offset = self.y_offset.draw(random);
        let top = origin.y + offset;
        if !tree.prepare(region, origin, height + offset, 1) {
            return None;
        }
        if (origin.y..top).any(|y| !self.can_place(tree, region, IVec3::new(origin.x, y, origin.z))) {
            return None;
        }
        let base = IVec3::new(origin.x, top, origin.z);
        let mut roots = vec![base - IVec3::Y];
        for direction in ROOT_DIRECTIONS {
            let start = base + direction;
            let mut branch = Vec::new();
            if !self.simulate(tree, region, random, (start, base, direction), &mut branch, 0) || branch.is_empty() {
                return None;
            }
            roots.append(&mut branch);
            roots.push(start);
        }
        for pos in roots {
            self.place_root(tree, region, random, pos);
        }
        Some(base)
    }

    fn can_place(&self, tree: &Tree, region: &Region, pos: IVec3) -> bool {
        let found = block(region, pos);
        self.may_grow_through.allows(region.blocks, found) || tree.foliage.leaf_replaceable(region.blocks, found) || region.blocks.entry(found).name.as_ref() == chorus_block::block_id::WATER
    }

    fn next_positions<D: Draw>(&self, random: &mut D, at: IVec3, origin: IVec3, direction: IVec3) -> Vec<IVec3> {
        let distance = (at - origin).abs().element_sum();
        let down = at - IVec3::Y;
        let forward = at + direction;
        if self.max_width < distance {
            return vec![down];
        }
        if self.max_width - ROOT_WIDTH_MARGIN >= distance {
            if random.float() >= ROOT_DOWN_CHANCE && random.boolean() {
                return vec![forward];
            }
            return vec![down];
        }
        if random.float() < ROOT_DOWN_CHANCE { vec![down, forward - IVec3::Y] } else { vec![down] }
    }

    fn simulate<D: Draw>(&self, tree: &Tree, region: &Region, random: &mut D, (at, origin, direction): (IVec3, IVec3, IVec3), branch: &mut Vec<IVec3>, depth: i32) -> bool {
        if depth == self.max_length || branch.len() as i32 > self.max_length {
            branch.clear();
            return false;
        }
        for next in self.next_positions(random, at, origin, direction) {
            if !self.can_place(tree, region, next) {
                continue;
            }
            branch.push(next);
            if !self.simulate(tree, region, random, (next, origin, direction), branch, depth + 1) {
                branch.clear();
                return false;
            }
        }
        true
    }

    fn place_root<D: Draw>(&self, tree: &Tree, region: &mut Region, random: &mut D, pos: IVec3) {
        if !self.can_place(tree, region, pos) {
            return;
        }
        let decoration = self.decoration.as_ref();
        if block(region, pos) == self.mud {
            super::decoration::Decoration::place_decorated(decoration, region, &tree.foliage, random, pos, self.muddy_root);
            return;
        }
        super::decoration::Decoration::place_decorated(decoration, region, &tree.foliage, random, pos, self.root);
        let Some((above_block, odds)) = self.above_root else { return };
        if !odds.roll(random) {
            return;
        }
        let above = pos + IVec3::Y;
        if region.blocks.is_air(block(region, above)) {
            super::decoration::Decoration::place_decorated(decoration, region, &tree.foliage, random, above, above_block);
        }
    }
}

impl MangroveCanopy {
    pub fn place<D: Draw>(&self, tree: &Tree, region: &mut Region, random: &mut D, anchor: &Anchor) {
        if anchor.candidates.is_empty() {
            return;
        }
        let height = self.height.draw_inclusive_always(random);
        let radius = self.radius.draw_inclusive_always(random);
        let mut leaves = Vec::new();
        for &log in &anchor.candidates {
            for _ in 0..self.attempts {
                let leaf = pick_weighted(&self.leaves, random);
                let mut jitter = |bound: i32| {
                    let first = random.below(bound);
                    first - random.below(bound)
                };
                let offset = IVec3::new(jitter(radius), jitter(height), jitter(radius));
                let pos = log + offset;
                if tree.valid_position(region, pos) && put(region, pos, leaf) {
                    leaves.push(pos);
                }
            }
        }
        for index in 1..leaves.len() {
            let other = random.below(index as i32 + 1) as usize;
            leaves.swap(index, other);
        }
        if let Some(decoration) = &self.decoration {
            for &pos in &leaves {
                decoration.place(region, &tree.foliage, random, pos, [true; 4]);
            }
        }
        let mut taken = HashSet::new();
        for &pos in &leaves {
            let below = pos - IVec3::Y;
            if taken.contains(&below) || !self.hanging_chance.roll(random) {
                continue;
            }
            if (1..=SEARCH_DEPTH).any(|depth| !region.blocks.is_air(block(region, pos - IVec3::Y * depth))) {
                continue;
            }
            put(region, below, self.hanging);
            for dx in -EXCLUDE_XZ..=EXCLUDE_XZ {
                for dy in -EXCLUDE_Y..=EXCLUDE_Y {
                    for dz in -EXCLUDE_XZ..=EXCLUDE_XZ {
                        taken.insert(below + IVec3::new(dx, dy, dz));
                    }
                }
            }
        }
    }
}
