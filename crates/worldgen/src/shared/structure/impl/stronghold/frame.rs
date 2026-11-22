use super::material::{Material, StrongholdBlocks};
use crate::shared::blocks::{BlockId, Blocks};
use crate::shared::chunk::proto::ProtoChunk;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::BoundingBox;
use crate::shared::{CHUNK_MASK, CHUNK_WIDTH};

pub type Cell = (i32, i32, i32);

pub const SOUTH: u8 = 0;
pub const WEST: u8 = 1;
pub const NORTH: u8 = 2;
pub const EAST: u8 = 3;

const STAIRS_BY_ORIENTATION: [[usize; 4]; 4] = [[0, 1, 3, 2], [2, 3, 0, 1], [0, 1, 2, 3], [2, 3, 1, 0]];
const FACING_BY_ORIENTATION: [[u8; 4]; 4] = [[3, 2, 5, 4], [4, 5, 3, 2], [2, 3, 4, 5], [5, 4, 2, 3]];
const DIRECTION_BY_ORIENTATION: [[u8; 4]; 4] = [[2, 3, 0, 1], [3, 0, 1, 2], [0, 1, 2, 3], [1, 2, 3, 0]];

pub fn orient_box((x, y, z): Cell, (ox, oy, oz): Cell, (sx, sy, sz): Cell, orientation: u8) -> BoundingBox {
    match orientation {
        NORTH => BoundingBox::new(x + ox, y + oy, z - sz + 1 + oz, x + sx - 1 + ox, y + sy - 1 + oy, z + oz),
        WEST => BoundingBox::new(x - sz + 1 + oz, y + oy, z + ox, x + oz, y + sy - 1 + oy, z + sx - 1 + ox),
        EAST => BoundingBox::new(x + oz, y + oy, z + ox, x + sz - 1 + oz, y + sy - 1 + oy, z + sx - 1 + ox),
        _ => BoundingBox::new(x + ox, y + oy, z + oz, x + sx - 1 + ox, y + sy - 1 + oy, z + sz - 1 + oz),
    }
}

pub struct Canvas<'a> {
    pub chunk: &'a mut ProtoChunk,
    pub blocks: &'a Blocks,
    pub ids: &'a StrongholdBlocks,
    pub bounds: BoundingBox,
}

impl<'a> Canvas<'a> {
    pub fn new(chunk: &'a mut ProtoChunk, blocks: &'a Blocks, ids: &'a StrongholdBlocks) -> Self {
        let (chunk_x, chunk_z) = (chunk.min_x().div_euclid(CHUNK_WIDTH), chunk.min_z().div_euclid(CHUNK_WIDTH));
        let definition = chunk.definition();
        Self {
            chunk,
            blocks,
            ids,
            bounds: BoundingBox::chunk(chunk_x, chunk_z, definition),
        }
    }

    fn get(&self, x: i32, y: i32, z: i32) -> BlockId {
        self.chunk.get(x & CHUNK_MASK, y, z & CHUNK_MASK)
    }

    fn set(&mut self, x: i32, y: i32, z: i32, material: Material) {
        let (local_x, local_z) = (x & CHUNK_MASK, z & CHUNK_MASK);
        let block = self.ids.id(material);
        let block = if self.chunk.get(local_x, y, local_z) == self.ids.id(Material::Water) {
            self.blocks.flooded(block)
        } else {
            block
        };
        self.chunk.set(local_x, y, local_z, block);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Frame {
    pub bounds: BoundingBox,
    pub orientation: u8,
    pub depth: i32,
}

impl Frame {
    pub fn world(&self, (x, y, z): Cell) -> Cell {
        let world_x = match self.orientation {
            WEST => self.bounds.x1 - z,
            EAST => self.bounds.x0 + z,
            _ => self.bounds.x0 + x,
        };
        let world_z = match self.orientation {
            SOUTH => self.bounds.z0 + z,
            NORTH => self.bounds.z1 - z,
            _ => self.bounds.z0 + x,
        };
        (world_x, self.bounds.y0 + y, world_z)
    }

    pub fn facing(&self, data: u8) -> u8 {
        FACING_BY_ORIENTATION[self.orientation as usize][data as usize - 2]
    }

    pub fn direction(&self, data: u8) -> u8 {
        DIRECTION_BY_ORIENTATION[self.orientation as usize][data as usize]
    }

    pub fn stairs(&self, family: [Material; 4], data: usize) -> Material {
        family[STAIRS_BY_ORIENTATION[self.orientation as usize][data]]
    }

    pub fn place(&self, canvas: &mut Canvas, material: Material, local: Cell) {
        let (x, y, z) = self.world(local);
        if canvas.bounds.contains(x, y, z) {
            canvas.set(x, y, z, material);
        }
    }

    pub fn generate_box(&self, canvas: &mut Canvas, (x0, y0, z0): Cell, (x1, y1, z1): Cell, material: Material) {
        for y in y0..=y1 {
            for x in x0..=x1 {
                for z in z0..=z1 {
                    self.place(canvas, material, (x, y, z));
                }
            }
        }
    }

    pub fn scatter_box(&self, canvas: &mut Canvas, random: &mut MersenneTwister, chance: f32, (x0, y0, z0): Cell, (x1, y1, z1): Cell, material: Material) {
        for y in y0..=y1 {
            for x in x0..=x1 {
                for z in z0..=z1 {
                    if random.next_float() <= chance {
                        self.place(canvas, material, (x, y, z));
                    }
                }
            }
        }
    }

    pub fn chest(&self, canvas: &mut Canvas, random: &mut MersenneTwister, local: Cell, material: Material) {
        let (x, y, z) = self.world(local);
        if canvas.bounds.contains(x, y, z) && !canvas.ids.is_chest(canvas.get(x, y, z)) {
            canvas.set(x, y, z, material);
            random.next_int();
        }
    }
}
