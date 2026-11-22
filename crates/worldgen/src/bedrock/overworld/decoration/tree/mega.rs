use super::site::{block, put};
use super::spec::{BaseCluster, Canopy, MegaBranches, MegaTrunk};
use super::{Anchor, Tree};
use crate::bedrock::overworld::decoration::ore::Draw;
use crate::bedrock::overworld::decoration::sine;
use crate::shared::blocks::BlockId;
use crate::shared::chunk::region::Region;
use glam::IVec3;

const CLUSTER_WINDOW: i32 = 64;
const CLUSTER_ROW: i32 = 8;
const CLUSTER_EDGE: i32 = 7;
const CLUSTER_SHIFT: i32 = 3;
const CIRCLE_DEPTHS: [i32; 6] = [2, 1, 0, -1, -2, -3];
const BRANCH_ROUND: f32 = 1.5;

impl MegaTrunk {
    pub fn place<D: Draw>(&self, tree: &Tree, region: &mut Region, random: &mut D, origin: IVec3) -> Option<IVec3> {
        let height = self.height_intervals.iter().fold(self.height_base, |total, &interval| total + random.below(interval));
        let origin = tree.roots_then(region, random, origin, height)?;
        if !self.spawn(tree, region, origin, height) {
            return None;
        }
        tree.place_canopies(region, random, &[Anchor::at(origin + IVec3::Y * height)]);
        if let Some(branches) = &self.branches {
            branches.place(tree, region, random, origin, height, self.block);
        }
        let replaceable = |region: &Region, pos: IVec3| tree.may_replace.allows(region.blocks, block(region, pos));
        for dx in 0..self.width {
            for dz in 0..self.width {
                let pos = origin + IVec3::new(dx, 0, dz);
                if replaceable(region, pos) {
                    put(region, pos, self.block);
                }
            }
        }
        for dx in 0..self.width {
            for dy in 1..height - 1 {
                for dz in 0..self.width {
                    let pos = origin + IVec3::new(dx, dy, dz);
                    if !replaceable(region, pos) || !put(region, pos, self.block) {
                        continue;
                    }
                    if let Some(decoration) = &self.decoration {
                        let mask = [dx == 0, dx != 0 && dx == self.width - 1, dz == 0, dz != 0 && dz == self.width - 1];
                        decoration.place(region, &tree.foliage, random, pos, mask);
                    }
                }
            }
        }
        let top = origin + IVec3::Y * (height - 1);
        if replaceable(region, top) {
            put(region, top, self.block);
        }
        for dx in 0..self.width {
            for dz in 0..self.width {
                tree.place_base_block(region, origin + IVec3::new(dx, -1, dz));
            }
        }
        if let (Some(cluster), Some(&target)) = (&tree.base_cluster, tree.base_block.first()) {
            cluster.place(region, random, origin - IVec3::Y, self.width, target);
        }
        Some(origin)
    }

    fn spawn(&self, tree: &Tree, region: &Region, origin: IVec3, height: i32) -> bool {
        if origin.y <= region.min_y() || origin.y + height > region.max_y() || !tree.may_grow_on.allows(region.blocks, block(region, origin - IVec3::Y)) {
            return false;
        }
        (0..height + 2).all(|dy| {
            let radius = if dy == 0 { (self.width - 1).abs().max(1) } else { self.width };
            (region.min_y()..=region.max_y()).contains(&(origin.y + dy))
                && (origin.x - radius..=origin.x + radius).all(|x| (origin.z - radius..=origin.z + radius).all(|z| tree.grows_through(region, block(region, IVec3::new(x, origin.y + dy, z)))))
        })
    }
}

impl MegaBranches {
    fn place<D: Draw>(&self, tree: &Tree, region: &mut Region, random: &mut D, origin: IVec3, height: i32, log: BlockId) {
        if self.length < 1 {
            return;
        }
        let scaled = height as f32;
        let mut level = ((origin.y - self.interval.draw(random)) as f32 + self.altitude_factor.1 * scaled) as i32;
        while self.altitude_factor.0 * scaled + (origin.y as f32) < level as f32 {
            let angle = random.float() * std::f32::consts::PI * 2.0;
            let (cos, sin) = (sine::cos(angle), sine::sin(angle));
            let start = (level - 1) as f32 - (self.length - 1) as f32 * self.slope;
            let mut tip = origin;
            for step in 0..self.length {
                let along = step as f32;
                tip = IVec3::new(
                    origin.x + (cos * along + BRANCH_ROUND) as i32,
                    (along * self.slope + start) as i32,
                    origin.z + (sin * along + BRANCH_ROUND) as i32,
                );
                put(region, tip, log);
            }
            if let Some(canopy) = &self.canopy {
                canopy.place(tree, region, random, &Anchor::at(IVec3::new(tip.x, level, tip.z)));
            }
            level -= self.interval.draw(random);
        }
    }
}

impl BaseCluster {
    fn place<D: Draw>(&self, region: &mut Region, random: &mut D, ground: IVec3, reach: i32, target: BlockId) {
        if self.count <= 0 {
            return;
        }
        for (dx, dz) in [(-1, -1), (reach, -1), (-1, reach), (reach, reach)] {
            self.circle(region, ground + IVec3::new(dx, 0, dz), target);
        }
        for _ in 0..self.count {
            let roll = random.below(CLUSTER_WINDOW);
            let (row, column) = (roll / CLUSTER_ROW, roll % CLUSTER_ROW);
            if row == 0 || row == CLUSTER_EDGE || column == 0 || column == CLUSTER_EDGE {
                self.circle(region, ground + IVec3::new(column - CLUSTER_SHIFT, 0, row - CLUSTER_SHIFT), target);
            }
        }
    }

    fn circle(&self, region: &mut Region, centre: IVec3, target: BlockId) {
        for dx in -self.radius..=self.radius {
            for dz in -self.radius..=self.radius {
                if dx.abs() == self.radius && dz.abs() == self.radius {
                    continue;
                }
                for dy in CIRCLE_DEPTHS {
                    let pos = centre + IVec3::new(dx, dy, dz);
                    let existing = block(region, pos);
                    if existing == target {
                        break;
                    }
                    if self.may_replace.allows(region.blocks, existing) {
                        put(region, pos, target);
                        break;
                    }
                }
            }
        }
    }
}

pub fn place_mega_canopy<D: Draw>(canopy: &Canopy, tree: &Tree, region: &mut Region, random: &mut D, at: IVec3) {
    match *canopy {
        Canopy::Mega {
            leaf,
            height,
            base_radius,
            core_width,
            simplify,
        } => {
            let value = height.draw(random);
            for index in 0..value.max(0) {
                let dy = 1 - value + index;
                let radius = value - 1 - index + base_radius + core_width / 2;
                radial(tree, region, at + IVec3::Y * dy, leaf, radius, core_width.max(1), simplify);
            }
        }
        Canopy::MegaPine {
            leaf,
            height,
            base_radius,
            radius_step_modifier,
            core_width,
        } => {
            let value = height.draw(random);
            let mut previous = 0;
            for dy in -value..=0 {
                let y = at.y + dy;
                let (radius, plain) = if dy == 0 {
                    (base_radius, base_radius)
                } else {
                    let ratio = (-dy) as f32 / value as f32;
                    let plain = base_radius + (radius_step_modifier * ratio).floor() as i32;
                    let bump = i32::from(plain == previous && y & 1 == 0);
                    (plain + bump, plain)
                };
                previous = plain;
                radial(tree, region, IVec3::new(at.x, y, at.z), leaf, radius, core_width.max(1), false);
            }
        }
        _ => {}
    }
}

fn radial(tree: &Tree, region: &mut Region, at: IVec3, leaf: BlockId, radius: i32, core: i32, strict: bool) {
    if radius < 0 {
        return;
    }
    let within = |dx: i32, dz: i32| dx * dx + dz * dz <= radius * radius;
    for dx in -radius..=core - 1 + radius {
        for dz in -radius..=core - 1 + radius {
            let keep = within(dx, dz) || !strict && (within(dx - 1, dz - 1) || within(dx, dz - 1) || within(dx - 1, dz));
            let pos = at + IVec3::new(dx, 0, dz);
            if keep && tree.may_replace.allows(region.blocks, block(region, pos)) {
                put(region, pos, leaf);
            }
        }
    }
}
