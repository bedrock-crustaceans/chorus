use super::draw::pick_weighted;
use super::site::{block, put};
use super::spec::{PoplarCanopy, PoplarTrunk};
use super::{Anchor, Tree};
use crate::bedrock::overworld::decoration::ore::Draw;
use crate::shared::blocks::BlockId;
use crate::shared::chunk::region::Region;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::Probed;
use glam::IVec3;

const SIDES: [IVec3; 4] = [IVec3::new(0, 0, -1), IVec3::new(1, 0, 0), IVec3::new(0, 0, 1), IVec3::new(-1, 0, 0)];
const CLOSE_RADIUS: i32 = 3;
const NARROW_RADIUS: i32 = 5;
const WIDE_SKIRT: i32 = 2;
const SKIRT_INSET: i32 = 2;
const BRANCH_INSET: i32 = 3;
const TOP_LAYERS: i32 = 2;

fn branch_log(tree: &PoplarTrunk, offset: IVec3) -> BlockId {
    if offset.x == 0 { tree.log.along_z } else { tree.log.along_x }
}

impl PoplarTrunk {
    pub fn place(&self, tree: &Tree, region: &mut Region, random: &mut MersenneTwister, origin: IVec3) -> Probed<Option<IVec3>> {
        let height = self.height.draw(random);
        let Some(origin) = tree.roots_then(region, random, origin, height) else { return Ok(None) };
        if !tree.prepare(region, origin, height, 1) {
            return Ok(None);
        }
        tree.place_base_block(region, origin - IVec3::Y);
        let above = self.above_branches.draw_inclusive(random);
        for offset in 0..height {
            let pos = origin + IVec3::Y * offset;
            if tree.may_replace.allows(region.blocks, block(region, pos)) {
                put(region, pos, self.log.upright);
            }
            if let Some(feature) = &self.log_decoration {
                feature.place(region, random, pos)?;
            }
        }
        let mut sides = SIDES;
        for index in 1..sides.len() {
            let swap = (random.word() % (index as u32 + 1)) as usize;
            if swap != index {
                sides.swap(index, swap);
            }
        }
        let count = self.branches.draw_inclusive(random).max(0);
        let crown = origin + IVec3::Y * (height - above);
        for &side in sides.iter().take(count as usize) {
            let pos = crown - IVec3::Y + side;
            if tree.may_replace.allows(region.blocks, block(region, pos)) {
                put(region, pos, branch_log(self, side));
            }
        }
        tree.place_canopies(region, random, &[Anchor::at(crown)]);
        Ok(Some(origin))
    }
}

struct Shape<'a> {
    canopy: &'a PoplarCanopy,
    tree: &'a Tree,
    at: IVec3,
    height: i32,
    flipped: bool,
}

impl Shape<'_> {
    fn layer<D: Draw>(&self, region: &mut Region, random: &mut D, radius: i32, dy: i32) {
        let top = dy >= self.height - TOP_LAYERS;
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                if top && (dx.abs() == radius || dz.abs() == radius) {
                    continue;
                }
                let alike = (dx < 0 && dz < 0) || (dx > 0 && dz > 0);
                let wide = dx != 0 && dz != 0 && alike == self.flipped;
                let bonus = if wide { 1 } else { -i32::from(top) };
                let filled = i32::from(self.canopy.side_hole_chance < random.float());
                if dx.abs() + dz.abs() <= bonus + radius - 1 + filled {
                    let pos = self.at + IVec3::new(dx, dy, dz);
                    if self.tree.may_replace.allows(region.blocks, block(region, pos)) {
                        put(region, pos, self.canopy.leaf);
                    }
                }
            }
        }
    }
}

impl PoplarCanopy {
    fn radius<D: Draw>(&self, random: &mut D) -> i32 {
        pick_weighted(&self.radius, random)
    }

    pub fn place<D: Draw>(&self, tree: &Tree, region: &mut Region, random: &mut D, at: IVec3) {
        let radius = self.radius(random) - 1;
        let height = self.height.draw_inclusive(random);
        let shape = Shape {
            canopy: self,
            tree,
            at,
            height,
            flipped: random.boolean(),
        };
        shape.layer(region, random, radius - 2, height - 1);
        shape.layer(region, random, radius - 1, height - 2);
        shape.layer(region, random, radius - 1, height - 3);
        for dy in (1..=height - 4).rev() {
            shape.layer(region, random, radius, dy);
        }
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                let on_axis = dx == 0 && radius - dz.abs() > BRANCH_INSET || dz == 0 && radius - dx.abs() > BRANCH_INSET;
                let pos = at + IVec3::new(dx, height - 4, dz);
                if on_axis && tree.may_replace.allows(region.blocks, block(region, pos)) {
                    put(region, pos, if dz == 0 { self.branch.along_x } else { self.branch.along_z });
                }
            }
        }
        shape.layer(region, random, radius - 1, 0);
        let skirt = if radius < CLOSE_RADIUS {
            1
        } else if radius < NARROW_RADIUS {
            radius - SKIRT_INSET
        } else {
            WIDE_SKIRT
        };
        shape.layer(region, random, skirt, -1);
    }
}
