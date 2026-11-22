use super::material::{Material, ScatteredBlocks};
use crate::shared::CHUNK_WIDTH;
use crate::shared::blocks::{BlockId, Blocks};
use crate::shared::chunk::proto::ProtoChunk;

pub type Cell = (i32, i32, i32);

const SOUTH_FACING_TABLE: [usize; 4] = [0, 1, 3, 2];

pub struct Piece<'a> {
    pub chunk: &'a mut ProtoChunk,
    pub blocks: &'a Blocks,
    pub ids: &'a ScatteredBlocks,
    pub origin: Cell,
}

pub fn turned_stairs(family: [Material; 4], data: usize) -> Material {
    family[SOUTH_FACING_TABLE[data]]
}

impl Piece<'_> {
    pub fn world(&self, (x, y, z): Cell) -> Cell {
        (self.origin.0 + x, self.origin.1 + y, self.origin.2 + z)
    }

    pub fn contains(&self, (x, y, z): Cell) -> bool {
        (0..CHUNK_WIDTH).contains(&(x - self.chunk.min_x())) && (0..CHUNK_WIDTH).contains(&(z - self.chunk.min_z())) && (self.chunk.min_y..=self.chunk.max_y()).contains(&y)
    }

    pub fn block_at(&self, world: Cell) -> BlockId {
        if self.contains(world) {
            self.chunk.get(world.0 - self.chunk.min_x(), world.1, world.2 - self.chunk.min_z())
        } else {
            self.ids.id(Material::Air)
        }
    }

    pub fn get(&self, local: Cell) -> BlockId {
        self.block_at(self.world(local))
    }

    pub fn set_world(&mut self, world: Cell, block: BlockId) {
        if self.contains(world) {
            let (min_x, min_z) = (self.chunk.min_x(), self.chunk.min_z());
            self.chunk.set(world.0 - min_x, world.1, world.2 - min_z, block);
        }
    }

    pub fn place(&mut self, material: Material, local: Cell) {
        let world = self.world(local);
        self.set_world(world, self.ids.id(material));
    }

    pub fn generate_box(&mut self, (x0, y0, z0): Cell, (x1, y1, z1): Cell, edge: Material, fill: Material, skip_air: bool) {
        let air = self.ids.id(Material::Air);
        for y in y0..=y1 {
            for x in x0..=x1 {
                for z in z0..=z1 {
                    if skip_air && self.get((x, y, z)) == air {
                        continue;
                    }
                    let on_edge = y == y0 || y == y1 || x == x0 || x == x1 || z == z0 || z == z1;
                    self.place(if on_edge { edge } else { fill }, (x, y, z));
                }
            }
        }
    }

    pub fn fill_column_down(&mut self, material: Material, local: Cell) {
        let (wx, mut wy, wz) = self.world(local);
        if !self.contains((wx, wy, wz)) {
            return;
        }
        let air = self.ids.id(Material::Air);
        loop {
            let here = self.block_at((wx, wy, wz));
            let entry = self.blocks.entry(here);
            let passable = here == air || entry.liquid || !entry.solid;
            if !passable || wy <= self.chunk.min_y + 1 {
                break;
            }
            self.set_world((wx, wy, wz), self.ids.id(material));
            wy -= 1;
        }
        if self.block_at((wx, wy, wz)) == self.ids.id(Material::GrassBlock) {
            self.set_world((wx, wy, wz), self.ids.id(Material::Dirt));
        }
    }
}
