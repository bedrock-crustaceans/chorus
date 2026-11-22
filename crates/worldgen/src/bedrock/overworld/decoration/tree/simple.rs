use super::site::{block, put};
use super::spec::SimpleTrunk;
use super::{Anchor, Tree};
use crate::bedrock::overworld::decoration::ore::Draw;
use crate::shared::blocks::BlockId;
use crate::shared::chunk::region::Region;
use glam::IVec3;

impl Tree {
    pub(super) fn grows_through(&self, region: &Region, value: BlockId) -> bool {
        if self.may_grow_through.is_empty() {
            return region.blocks.is_air(value) || region.blocks.entry(value).name.as_ref() == chorus_block::block_id::WATER;
        }
        self.may_grow_through.contains(region.blocks, value) || self.may_replace.allows(region.blocks, value)
    }

    pub(super) fn valid_position(&self, region: &Region, pos: IVec3) -> bool {
        let found = block(region, pos);
        self.may_replace.allows(region.blocks, found) || self.foliage.leaf_replaceable(region.blocks, found) || region.blocks.entry(found).name.as_ref() == chorus_block::block_id::WATER
    }

    pub(super) fn place_base_block(&self, region: &mut Region, pos: IVec3) {
        let Some(&first) = self.base_block.first() else { return };
        if !self.base_block.contains(&block(region, pos)) {
            put(region, pos, first);
        }
    }

    pub(super) fn prepare(&self, region: &Region, origin: IVec3, height: i32, width: i32) -> bool {
        let top = region.max_y() + 1;
        if origin.y <= region.min_y() || origin.y + height >= top {
            return false;
        }
        let grounded = (0..width).all(|dx| (0..width).all(|dz| self.may_grow_on.allows(region.blocks, block(region, origin + IVec3::new(dx, -1, dz)))));
        if !grounded {
            return false;
        }
        (origin.y..=(origin.y + height + 1).min(top - 1)).all(|y| {
            let radius = if y >= origin.y + height - 1 { 2 } else { i32::from(y != origin.y) };
            (origin.x - radius..=origin.x + radius).all(|x| (origin.z - radius..=origin.z + radius).all(|z| self.grows_through(region, block(region, IVec3::new(x, y, z)))))
        })
    }
}

impl SimpleTrunk {
    pub fn place<D: Draw>(&self, tree: &Tree, region: &mut Region, random: &mut D, origin: IVec3) -> Option<IVec3> {
        let height = self.height.draw(random);
        let origin = tree.roots_then(region, random, origin, height)?;
        let mut base = origin;
        let sinks = |region: &Region, pos: IVec3| {
            let found = block(region, pos);
            if tree.may_grow_through.is_empty() {
                region.blocks.entry(found).name.as_ref() == chorus_block::block_id::WATER
            } else {
                tree.may_grow_through.contains(region.blocks, found)
            }
        };
        if self.max_depth >= 1 && sinks(region, base - IVec3::Y) {
            let mut budget = self.max_depth;
            loop {
                budget -= 1;
                base.y -= 1;
                if budget <= 0 || !sinks(region, base - IVec3::Y) {
                    break;
                }
            }
        }
        if !tree.prepare(region, base, height, 1) {
            return None;
        }
        let total = height + self.height_modifier.draw(random);
        if total < 1 {
            return None;
        }
        let mut top = None;
        for y in base.y..base.y + total {
            let pos = IVec3::new(base.x, y, base.z);
            let gate = if y >= origin.y { &tree.may_replace } else { &tree.may_grow_through };
            if gate.allows(region.blocks, block(region, pos)) {
                put(region, pos, self.block);
                if let Some(decoration) = &self.decoration {
                    decoration.place(region, &tree.foliage, random, pos, [true; 4]);
                }
                top = Some(y);
            }
        }
        let top = top?;
        let crown = IVec3::new(base.x, top + 1, base.z);
        tree.place_canopies(region, random, &[Anchor::at(crown)]);
        tree.place_base_block(region, base - IVec3::Y);
        Some(crown)
    }
}
