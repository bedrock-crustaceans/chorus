use super::lake::{BOX_XZ, BOX_Y, Cell, FLUID_LEVELS, LakeShape, cell};
use super::ore::Draw;
use super::world::read;
use crate::shared::blocks::{BlockId, BlockTable};
use crate::shared::chunk::region::Region;
use chorus_block::block_id::{AIR, SULFUR, SULFUR_SPIKE, WATER};
use glam::IVec3;

const BORDER_DRAW_BOUND: i32 = 2;

#[derive(Debug)]
pub struct SulfurPool {
    water: BlockId,
    air: BlockId,
    sulfur: BlockId,
    spike: u16,
}

impl SulfurPool {
    pub fn new(table: &mut BlockTable) -> Self {
        Self {
            water: table.name(WATER),
            air: table.name(AIR),
            sulfur: table.name(SULFUR),
            spike: table.kind(SULFUR_SPIKE),
        }
    }

    pub fn place<D: Draw>(&self, region: &mut Region, random: &mut D, origin: IVec3) -> bool {
        region.set_any_rule();
        let Some(shape) = LakeShape::build(region, random, origin) else { return false };
        let base = shape.base;
        for x in 0..BOX_XZ {
            for z in 0..BOX_XZ {
                for y in 0..BOX_Y {
                    if !shape.borders(x, z, y) {
                        continue;
                    }
                    let pos = base + IVec3::new(x, y, z);
                    let found = cell(region, pos);
                    let liquid = matches!(found, Cell::Water | Cell::Lava);
                    if (y >= FLUID_LEVELS && liquid) || (y < FLUID_LEVELS && found != Cell::Solid && found != Cell::Water) {
                        return false;
                    }
                    if region.blocks.entry(read(region, pos)).kind == self.spike {
                        return false;
                    }
                }
            }
        }

        let mut border = Vec::new();
        for x in 0..BOX_XZ {
            for z in 0..BOX_XZ {
                for y in 0..BOX_Y {
                    if shape.borders(x, z, y) && (y < FLUID_LEVELS || random.below(BORDER_DRAW_BOUND) != 0) && cell(region, base + IVec3::new(x, y, z)) == Cell::Solid {
                        border.push(base + IVec3::new(x, y, z));
                    }
                }
            }
        }
        for x in 0..BOX_XZ {
            for z in 0..BOX_XZ {
                for y in 0..BOX_Y {
                    if shape.filled(x, z, y) {
                        region.set(base + IVec3::new(x, y, z), if y < FLUID_LEVELS { self.water } else { self.air });
                    }
                }
            }
        }
        for pos in border {
            region.set(pos, self.sulfur);
        }
        true
    }
}
