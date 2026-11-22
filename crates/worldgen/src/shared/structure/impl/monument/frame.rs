use super::material::{Material, MonumentBlocks};
use crate::shared::blocks::{BlockId, Blocks};
use crate::shared::chunk::proto::ProtoChunk;
use crate::shared::structure::BoundingBox;
use crate::shared::{CHUNK_MASK, CHUNK_WIDTH};

pub type Cell = (i32, i32, i32);

pub const SOUTH: u8 = 0;
pub const WEST: u8 = 1;
pub const NORTH: u8 = 2;
pub const EAST: u8 = 3;

#[derive(Clone, Copy)]
pub struct Fill {
    pub material: Material,
    pub from: Cell,
    pub to: Cell,
}

pub const fn fill(material: Material, from: Cell, to: Cell) -> Fill {
    Fill { material, from, to }
}

pub struct Canvas<'a> {
    pub chunk: &'a mut ProtoChunk,
    pub blocks: &'a Blocks,
    pub ids: &'a MonumentBlocks,
    pub bounds: BoundingBox,
    pub sea_level: i32,
}

impl<'a> Canvas<'a> {
    pub fn new(chunk: &'a mut ProtoChunk, blocks: &'a Blocks, ids: &'a MonumentBlocks, sea_level: i32) -> Self {
        let (chunk_x, chunk_z) = (chunk.min_x().div_euclid(CHUNK_WIDTH), chunk.min_z().div_euclid(CHUNK_WIDTH));
        let definition = chunk.definition();
        Self {
            chunk,
            blocks,
            ids,
            bounds: BoundingBox::chunk(chunk_x, chunk_z, definition),
            sea_level,
        }
    }

    fn get(&self, (x, y, z): Cell) -> BlockId {
        self.chunk.get(x & CHUNK_MASK, y, z & CHUNK_MASK)
    }

    fn set(&mut self, (x, y, z): Cell, material: Material) {
        self.chunk.set(x & CHUNK_MASK, y, z & CHUNK_MASK, self.ids.id(material));
    }

    pub fn touches(&self, (ax, az): (i32, i32), (bx, bz): (i32, i32)) -> bool {
        ax.min(bx) <= self.bounds.x1 && ax.max(bx) >= self.bounds.x0 && az.min(bz) <= self.bounds.z1 && az.max(bz) >= self.bounds.z0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Frame {
    pub bounds: BoundingBox,
    pub orientation: u8,
}

impl Frame {
    pub fn world_x(&self, x: i32, z: i32) -> i32 {
        match self.orientation {
            WEST => self.bounds.x1 - z,
            EAST => self.bounds.x0 + z,
            _ => self.bounds.x0 + x,
        }
    }

    pub fn world_z(&self, x: i32, z: i32) -> i32 {
        match self.orientation {
            SOUTH => self.bounds.z0 + z,
            NORTH => self.bounds.z1 - z,
            _ => self.bounds.z0 + x,
        }
    }

    pub fn world(&self, (x, y, z): Cell) -> Cell {
        (self.world_x(x, z), self.bounds.y0 + y, self.world_z(x, z))
    }

    pub fn place(&self, canvas: &mut Canvas, material: Material, local: Cell) {
        let world = self.world(local);
        if canvas.bounds.contains(world.0, world.1, world.2) {
            canvas.set(world, material);
        }
    }

    pub fn fill(&self, canvas: &mut Canvas, material: Material, (x0, y0, z0): Cell, (x1, y1, z1): Cell) {
        for y in y0..=y1 {
            for x in x0..=x1 {
                for z in z0..=z1 {
                    self.place(canvas, material, (x, y, z));
                }
            }
        }
    }

    pub fn fill_all(&self, canvas: &mut Canvas, fills: &[Fill]) {
        for fill in fills {
            self.fill(canvas, fill.material, fill.from, fill.to);
        }
    }

    pub fn flood(&self, canvas: &mut Canvas, from: Cell, to: Cell) {
        for y in from.1..=to.1 {
            let material = if self.bounds.y0 + y < canvas.sea_level { Material::Water } else { Material::Air };
            self.fill(canvas, material, (from.0, y, from.2), (to.0, y, to.2));
        }
    }

    pub fn flood_where(&self, canvas: &mut Canvas, boxes: &[(bool, Cell, Cell)]) {
        for &(open, from, to) in boxes {
            if open {
                self.flood(canvas, from, to);
            }
        }
    }

    pub fn fill_over_water(&self, canvas: &mut Canvas, material: Material, (x0, y0, z0): Cell, (x1, y1, z1): Cell) {
        let water = canvas.ids.id(Material::Water);
        for y in y0..=y1 {
            for x in x0..=x1 {
                for z in z0..=z1 {
                    let world = self.world((x, y, z));
                    if canvas.bounds.contains(world.0, world.1, world.2) && canvas.get(world) == water {
                        canvas.set(world, material);
                    }
                }
            }
        }
    }

    pub fn pillar_down(&self, canvas: &mut Canvas, material: Material, local: Cell) {
        let (x, mut y, z) = self.world(local);
        if !canvas.bounds.contains(x, y, z) {
            return;
        }
        let air = canvas.ids.id(Material::Air);
        while y > canvas.chunk.min_y + 1 {
            let here = canvas.get((x, y, z));
            let entry = canvas.blocks.entry(here);
            if here != air && entry.solid && !entry.liquid {
                break;
            }
            canvas.set((x, y, z), material);
            y -= 1;
        }
    }

    pub fn reaches(&self, canvas: &Canvas, from: (i32, i32), to: (i32, i32)) -> bool {
        canvas.touches((self.world_x(from.0, from.1), self.world_z(from.0, from.1)), (self.world_x(to.0, to.1), self.world_z(to.0, to.1)))
    }
}
