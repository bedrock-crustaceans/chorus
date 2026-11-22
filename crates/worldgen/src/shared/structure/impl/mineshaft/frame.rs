use super::canvas::{Canvas, Cell};
use crate::shared::blocks::BlockId;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::{BoundingBox, Probed};

pub const SOUTH: u8 = 0;
pub const WEST: u8 = 1;
pub const NORTH: u8 = 2;
pub const EAST: u8 = 3;

const SPHERE_LIMIT: f32 = 1.05;
const SPHERE_HALF: f32 = 0.5;
const NEIGHBOURS: [Cell; 6] = [(0, -1, 0), (0, 1, 0), (0, 0, -1), (0, 0, 1), (-1, 0, 0), (1, 0, 0)];
const MIN_SUPPORTING_NEIGHBOURS: usize = 2;

#[derive(Clone, Copy)]
pub struct Scatter {
    pub chance: f32,
    pub interior_only: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct Frame {
    pub bounds: BoundingBox,
    pub orientation: Option<u8>,
    pub depth: i32,
}

impl Frame {
    pub fn world_x(&self, x: i32, z: i32) -> i32 {
        match self.orientation {
            Some(WEST) => self.bounds.x1 - z,
            Some(EAST) => self.bounds.x0 + z,
            Some(_) => self.bounds.x0 + x,
            None => x,
        }
    }

    pub fn world_z(&self, x: i32, z: i32) -> i32 {
        match self.orientation {
            Some(SOUTH) => self.bounds.z0 + z,
            Some(NORTH) => self.bounds.z1 - z,
            Some(_) => self.bounds.z0 + x,
            None => z,
        }
    }

    pub fn world_y(&self, y: i32) -> i32 {
        if self.orientation.is_some() { self.bounds.y0 + y } else { y }
    }

    pub fn world(&self, (x, y, z): Cell) -> Cell {
        (self.world_x(x, z), self.world_y(y), self.world_z(x, z))
    }

    pub fn get(&self, canvas: &Canvas, local: Cell) -> Probed<BlockId> {
        let (x, y, z) = self.world(local);
        canvas.get_clipped(x, y, z)
    }

    pub fn interior(&self, canvas: &Canvas, local: Cell) -> Probed<bool> {
        let (x, y, z) = self.world(local);
        canvas.interior(x, y, z)
    }

    pub fn place(&self, canvas: &mut Canvas, block: BlockId, local: Cell) -> Probed<()> {
        let (x, y, z) = self.world(local);
        if canvas.bounds.contains(x, y, z) && !canvas.shaft.is_kept(canvas.get(x, y, z)?) && !canvas.fences.contains(&(x, y, z)) {
            canvas.set(x, y, z, block);
        }
        Ok(())
    }

    pub fn generate_box(&self, canvas: &mut Canvas, (x0, y0, z0): Cell, (x1, y1, z1): Cell, edge: BlockId, fill: BlockId) -> Probed<()> {
        for y in y0..=y1 {
            for x in x0..=x1 {
                for z in z0..=z1 {
                    let on_edge = y == y0 || y == y1 || x == x0 || x == x1 || z == z0 || z == z1;
                    self.place(canvas, if on_edge { edge } else { fill }, (x, y, z))?;
                }
            }
        }
        Ok(())
    }

    pub fn generate_maybe_box(&self, canvas: &mut Canvas, random: &mut MersenneTwister, scatter: Scatter, (x0, y0, z0): Cell, (x1, y1, z1): Cell, (edge, fill): (BlockId, BlockId)) -> Probed<()> {
        for y in y0..=y1 {
            for x in x0..=x1 {
                for z in z0..=z1 {
                    if random.next_float() > scatter.chance || scatter.interior_only && !self.interior(canvas, (x, y + 1, z))? {
                        continue;
                    }
                    let on_edge = y == y0 || y == y1 || x == x0 || x == x1 || z == z0 || z == z1;
                    self.place(canvas, if on_edge { edge } else { fill }, (x, y, z))?;
                }
            }
        }
        Ok(())
    }

    pub fn upper_half_sphere(&self, canvas: &mut Canvas, (x0, y0, z0): Cell, (x1, y1, z1): Cell, fill: BlockId) -> Probed<()> {
        let height = (y1 - y0 + 1) as f32;
        let (radius_x, radius_z) = ((x1 - x0 + 1) as f32 * SPHERE_HALF, (z1 - z0 + 1) as f32 * SPHERE_HALF);
        for y in y0..=y1 {
            let fy = (y - y0) as f32 / height;
            for x in x0..=x1 {
                let fx = (x as f32 - (x0 as f32 + radius_x)) / radius_x;
                for z in z0..=z1 {
                    let fz = (z as f32 - (z0 as f32 + radius_z)) / radius_z;
                    if fz * fz + fx * fx + fy * fy <= SPHERE_LIMIT {
                        self.place(canvas, fill, (x, y, z))?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn maybe_block_if_not_floating(&self, canvas: &mut Canvas, random: &mut MersenneTwister, chance: f32, local: Cell, block: BlockId) -> Probed<()> {
        if random.next_float() >= chance {
            return Ok(());
        }
        let (x, y, z) = self.world(local);
        let mut supported = 0;
        for &(dx, dy, dz) in &NEIGHBOURS {
            if !canvas.is_air(canvas.get_clipped(x + dx, y + dy, z + dz)?) {
                supported += 1;
            }
        }
        if supported >= MIN_SUPPORTING_NEIGHBOURS {
            self.place(canvas, block, local)?;
        }
        Ok(())
    }
}
