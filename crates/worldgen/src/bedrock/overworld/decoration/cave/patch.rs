use super::super::ore::resolve_block;
use super::super::placed::Placed;
use super::super::world::{is_air, read};
use super::support::holds_face_at;
use super::{BlockSet, Span};
use crate::shared::blocks::{BlockId, BlockTable};
use crate::shared::chunk::region::{Region, RuleKind};
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::Probed;
use glam::IVec3;
use serde_json::Value;
use std::sync::Arc;

const SURFACE_CEILING: &str = "ceiling";
const EDGE_NEIGHBOURS: [IVec3; 5] = [IVec3::NEG_Z, IVec3::X, IVec3::Z, IVec3::NEG_X, IVec3::NEG_Y];

#[derive(Debug)]
pub struct Patch {
    pub replaceable: BlockSet,
    pub ground: BlockId,
    pub vegetation: Arc<Placed>,
    pub depth: Span,
    pub extra_deep_chance: f32,
    pub vertical_range: i32,
    pub vegetation_chance: f32,
    pub radius: Span,
    pub extra_edge_chance: f32,
    pub waterlogged: bool,
    pub ceiling: bool,
}

impl Patch {
    pub fn parse(table: &mut BlockTable, vegetation: Arc<Placed>, body: &'static Value) -> Option<Self> {
        let chance = |key: &str| body.get(key).and_then(Value::as_f64).unwrap_or(0.0) as f32;
        Some(Self {
            replaceable: BlockSet::parse(table, body.get("replaceable_blocks")),
            ground: resolve_block(table, body.get("ground_block")?)?,
            vegetation,
            depth: Span::parse(body.get("depth"))?,
            extra_deep_chance: chance("extra_deep_block_chance"),
            vertical_range: body.get("vertical_range")?.as_i64()? as i32,
            vegetation_chance: chance("vegetation_chance"),
            radius: Span::parse(body.get("horizontal_radius"))?,
            extra_edge_chance: chance("extra_edge_column_chance"),
            waterlogged: body.get("waterlogged").and_then(Value::as_bool).unwrap_or(false),
            ceiling: body.get("surface").and_then(Value::as_str) == Some(SURFACE_CEILING),
        })
    }

    pub fn place(&self, region: &mut Region, random: &mut MersenneTwister, pos: IVec3) -> Probed<bool> {
        region.set_rule(self.replaceable.kinds(), RuleKind::Replaceable);
        let radius_x = self.radius.draw(random) + 1;
        let radius_z = self.radius.draw(random) + 1;
        let mut cells = self.lay_ground(region, random, pos, radius_x, radius_z);
        if self.waterlogged {
            cells.retain(|&cell| !self.exposed(region, cell));
            let water = region.water();
            for &cell in &cells {
                region.set(cell, water);
            }
        }
        if cells.is_empty() {
            return Ok(false);
        }
        let up = if self.ceiling { IVec3::NEG_Y } else { IVec3::Y };
        let drop = if self.waterlogged { IVec3::NEG_Y } else { IVec3::ZERO };
        for &cell in &cells {
            if self.vegetation_chance > 0.0 && random.next_float() < self.vegetation_chance {
                self.vegetation.place(region, random, cell + drop + up)?;
            }
        }
        Ok(true)
    }

    fn lay_ground(&self, region: &mut Region, random: &mut MersenneTwister, origin: IVec3, radius_x: i32, radius_z: i32) -> Vec<IVec3> {
        let scan = if self.ceiling { IVec3::Y } else { IVec3::NEG_Y };
        let mut cells = Vec::new();
        for dx in -radius_x..=radius_x {
            let x_edge = dx.abs() == radius_x;
            for dz in -radius_z..=radius_z {
                let z_edge = dz.abs() == radius_z;
                if x_edge && z_edge {
                    continue;
                }
                if x_edge != z_edge && (self.extra_edge_chance == 0.0 || random.next_float() > self.extra_edge_chance) {
                    continue;
                }
                let mut pos = origin + IVec3::new(dx, 0, dz);
                if is_air(region, pos) {
                    for steps in 1.. {
                        pos += scan;
                        if !is_air(region, pos) || steps >= self.vertical_range {
                            break;
                        }
                    }
                }
                if !is_air(region, pos) {
                    for steps in 1.. {
                        pos -= scan;
                        if is_air(region, pos) || steps >= self.vertical_range {
                            break;
                        }
                    }
                }
                let ground = pos + scan;
                if !is_air(region, pos) || !holds_face_at(region, ground) {
                    continue;
                }
                let deep = i32::from(self.extra_deep_chance > 0.0 && random.next_float() < self.extra_deep_chance);
                let depth = self.depth.draw(random) + deep;
                if self.fill_column(region, ground, scan, depth) {
                    cells.push(ground);
                }
            }
        }
        cells
    }

    fn fill_column(&self, region: &mut Region, ground: IVec3, scan: IVec3, depth: i32) -> bool {
        let mut written = 0;
        for layer in 0..depth {
            let pos = ground + scan * layer;
            let block = read(region, pos);
            if block == self.ground {
                written += 1;
            } else if self.replaceable.contains(region.blocks, block) {
                region.set(pos, self.ground);
                written += 1;
            } else {
                return written > 0;
            }
        }
        true
    }

    fn exposed(&self, region: &Region, cell: IVec3) -> bool {
        EDGE_NEIGHBOURS.iter().any(|&offset| !holds_face_at(region, cell + offset))
    }
}
