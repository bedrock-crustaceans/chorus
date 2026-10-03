pub mod aquifer;
pub mod carver;
pub mod features;
pub mod surface;

#[cfg(test)]
mod tests;

pub(crate) use super::SEA_LEVEL;
pub(crate) use super::shared::{biome, blocks, moss_carpet, noise, proto, random, region, shape, survival, tags, terrain};

use super::{Edition, OverworldGenerator};
use crate::block::block_id::{LAVA, STONE, WATER};
use aquifer::{Aquifer, Fluid};
use blocks::{BlockId, BlockTable};
use features::Catalog;
use proto::{ProtoChunk, QuartBiomes};
use random::PositionalFactory;
use region::Changes;
use shape::Shapes;
use surface::{Floor, MaterialSystem};
use terrain::{HEIGHT, MIN_Y, NoiseChunk};

#[derive(Clone, Copy)]
pub(crate) struct Rules {
    pub fuzzy_biomes: bool,
    pub floor: Floor,
}

impl Rules {
    pub(crate) const JAVA: Self = Self {
        fuzzy_biomes: true,
        floor: Floor::Gradient,
    };
}

pub struct Java {
    rules: Rules,
    aquifer_random: PositionalFactory,
    material: MaterialSystem,
    catalog: Catalog,
}

impl Java {
    pub(crate) fn create(seed: i64, random: &PositionalFactory, table: &mut BlockTable, rules: Rules) -> Self {
        let material = MaterialSystem::new(table, *random, SEA_LEVEL, rules.floor);
        table.name(STONE);
        table.name(WATER);
        table.name(LAVA);
        let catalog = Catalog::new(table, seed);
        Self {
            rules,
            aquifer_random: random.hash_of("minecraft:aquifer").fork_positional(),
            material,
            catalog,
        }
    }

    pub(crate) fn fill_noise<E: Edition>(&self, generator: &OverworldGenerator<E>, x: i32, z: i32, around: [&QuartBiomes; 9], noise: NoiseChunk) -> (ProtoChunk, NoiseChunk, Aquifer) {
        let mut chunk = ProtoChunk::new(x, z, MIN_Y, HEIGHT, generator.water, generator.lava, self.rules.fuzzy_biomes.then_some(generator.zoom_seed), around);
        let densities = noise.densities();
        let known = around.iter().enumerate().flat_map(|(i, quarts)| quarts.surface_levels(x + i as i32 / 3 - 1, z + i as i32 % 3 - 1));
        let mut aquifer = Aquifer::new(&generator.terrain, self.aquifer_random, x, z, MIN_Y, HEIGHT, SEA_LEVEL, known);
        for lz in 0..16 {
            for lx in 0..16 {
                for y in (MIN_Y..MIN_Y + HEIGHT).rev() {
                    let density = densities.get(lx, y, lz) as f64;
                    let block = match aquifer.compute_substance(&generator.terrain, chunk.min_x() + lx, y, chunk.min_z() + lz, density) {
                        None => generator.stone,
                        Some(Fluid::Air) => continue,
                        Some(Fluid::Water) => generator.water,
                        Some(Fluid::Lava) => generator.lava,
                    };
                    chunk.set(lx, y, lz, block);
                }
            }
        }
        (chunk, noise, aquifer)
    }

    pub(crate) fn build_surface<E: Edition>(&self, generator: &OverworldGenerator<E>, noise: &NoiseChunk, chunk: &mut ProtoChunk) {
        self.material.build(&generator.terrain, noise, chunk);
    }

    pub(crate) fn apply_carvers<E: Edition>(&self, generator: &OverworldGenerator<E>, noise: &NoiseChunk, aquifer: &mut Aquifer, chunk: &mut ProtoChunk) {
        let mask = carver::carve(generator.seed, chunk.x, chunk.z);
        let corners = MaterialSystem::surface_corners(&generator.terrain, chunk);
        let blocks = &self.material.blocks;
        mask.visit(|x, z, bottom, top| {
            let mut has_grass = false;
            let (world_x, world_z) = (chunk.min_x() + x, chunk.min_z() + z);
            for y in (bottom..=top).rev() {
                let block = chunk.get(x, y, z);
                if block == blocks.bedrock {
                    continue;
                }
                if block == blocks.grass_block || block == blocks.mycelium {
                    has_grass = true;
                }
                let fluid = aquifer.compute_substance(&generator.terrain, world_x, y, world_z, 0.0).unwrap_or(Fluid::Air);
                chunk.set(x, y, z, fluid_block(generator, fluid));
                if has_grass && chunk.get(x, y - 1, z) == blocks.dirt {
                    let under_fluid = fluid != Fluid::Air;
                    if let Some(top) = self.material.top_material(&generator.terrain, noise, chunk, corners, x, y - 1, z, under_fluid) {
                        chunk.set(x, y - 1, z, top);
                    }
                }
            }
        });
        chunk.freeze_worldgen_heights(&generator.blocks);
    }

    pub(crate) fn decorate<E: Edition>(&self, generator: &OverworldGenerator<E>, owner_x: i32, owner_z: i32, chunks: [ProtoChunk; 9]) -> Changes {
        self.catalog.decorate(generator.seed, owner_x, owner_z, &generator.blocks, chunks)
    }

    pub(crate) fn shapes(&self, water: BlockId) -> Shapes<'_> {
        Shapes {
            carpet: &self.catalog.moss_carpet,
            water,
        }
    }
}

fn fluid_block<E: Edition>(generator: &OverworldGenerator<E>, fluid: Fluid) -> BlockId {
    match fluid {
        Fluid::Air => blocks::AIR,
        Fluid::Water => generator.water,
        Fluid::Lava => generator.lava,
    }
}

impl Edition for Java {
    type Aquifer = Aquifer;

    fn new(seed: i64, random: &PositionalFactory, table: &mut BlockTable) -> Self {
        Self::create(seed, random, table, Rules::JAVA)
    }

    fn fill_noise(&self, generator: &OverworldGenerator<Self>, x: i32, z: i32, around: [&QuartBiomes; 9], noise: NoiseChunk) -> (ProtoChunk, NoiseChunk, Aquifer) {
        Java::fill_noise(self, generator, x, z, around, noise)
    }

    fn build_surface(&self, generator: &OverworldGenerator<Self>, noise: &NoiseChunk, chunk: &mut ProtoChunk) {
        Java::build_surface(self, generator, noise, chunk);
    }

    fn apply_carvers(&self, generator: &OverworldGenerator<Self>, noise: &NoiseChunk, aquifer: &mut Aquifer, chunk: &mut ProtoChunk) {
        Java::apply_carvers(self, generator, noise, aquifer, chunk);
    }

    fn decorate(&self, generator: &OverworldGenerator<Self>, owner_x: i32, owner_z: i32, chunks: [ProtoChunk; 9]) -> Changes {
        Java::decorate(self, generator, owner_x, owner_z, chunks)
    }

    fn shapes(&self, water: BlockId) -> Shapes<'_> {
        Java::shapes(self, water)
    }
}
