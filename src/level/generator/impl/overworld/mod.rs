mod bedrock;
mod java;
mod shared;

use crate::block::block_id::{LAVA, STONE, WATER};
use crate::registry::block_registry::BlockRegistry;
use glam::IVec3;
use shared::biome::{Biome, BiomeSource};
use shared::blocks::{BlockId, BlockTable, Blocks};
use shared::proto::{self, ProtoChunk, QuartBiomes};
use shared::random::{PositionalFactory, Xoroshiro};
use shared::region::Changes;
use shared::shape::Shapes;
use shared::terrain::{HEIGHT, MIN_Y, NoiseChunk, Terrain};

pub use bedrock::Bedrock;
pub use java::Java;

pub(crate) const SEA_LEVEL: i32 = 63;

pub trait Edition: Send + Sync + Sized + 'static {
    type Aquifer: Clone + Send + Sync + 'static;

    fn new(seed: i64, random: &PositionalFactory, table: &mut BlockTable) -> Self;

    fn fill_noise(&self, generator: &OverworldGenerator<Self>, x: i32, z: i32, around: [&QuartBiomes; 9], noise: NoiseChunk) -> (ProtoChunk, NoiseChunk, Self::Aquifer);

    fn build_surface(&self, generator: &OverworldGenerator<Self>, noise: &NoiseChunk, chunk: &mut ProtoChunk);

    fn apply_carvers(&self, generator: &OverworldGenerator<Self>, noise: &NoiseChunk, aquifer: &mut Self::Aquifer, chunk: &mut ProtoChunk);

    fn decorate(&self, generator: &OverworldGenerator<Self>, owner_x: i32, owner_z: i32, chunks: [ProtoChunk; 9]) -> Changes;

    fn shapes(&self, water: BlockId) -> Shapes<'_>;
}

pub struct OverworldGenerator<E: Edition> {
    pub seed: i64,
    zoom_seed: i64,
    terrain: Terrain,
    biomes: BiomeSource,
    blocks: Blocks,
    stone: BlockId,
    water: BlockId,
    lava: BlockId,
    edition: E,
}

impl<E: Edition> OverworldGenerator<E> {
    pub fn new(seed: i64, registry: &BlockRegistry) -> Self {
        let random = Xoroshiro::new(seed).fork_positional();
        let mut table = BlockTable::new(registry);
        let edition = E::new(seed, &random, &mut table);
        let stone = table.name(STONE);
        let water = table.name(WATER);
        let lava = table.name(LAVA);
        Self {
            seed,
            zoom_seed: proto::zoom_seed(seed),
            terrain: Terrain::new(&random),
            biomes: BiomeSource::new(),
            blocks: table.finish(),
            stone,
            water,
            lava,
            edition,
        }
    }

    fn quart_biomes(&self, x: i32, z: i32) -> QuartBiomes {
        let quarts: [_; 16] = std::array::from_fn(|i| self.terrain.quart_column((x << 4) + (i as i32 % 4) * 4, (z << 4) + (i as i32 / 4) * 4));
        let levels = (HEIGHT / 4 + 2) as usize;
        let mut biomes = vec![Biome::Plains; 16 * levels];
        let mut column_biomes = vec![Biome::Plains; levels];
        let mut hints = self.biomes.column_hints();
        for (index, (column, _)) in quarts.iter().enumerate() {
            let climate = column.at(MIN_Y);
            let depths: Vec<f32> = (-1..=HEIGHT / 4).map(|qy| column.at(MIN_Y + qy * 4).depth).collect();
            let fixed = [climate.temperature, climate.vegetation, climate.continents, climate.erosion, climate.ridges];
            self.biomes.find_column(fixed, &depths, &mut hints, &mut column_biomes);
            for (level, &biome) in column_biomes.iter().enumerate() {
                biomes[level * 16 + index] = biome;
            }
        }
        QuartBiomes::new(biomes, quarts.map(|(_, level)| level.floor() as i32))
    }

    fn standalone_noise(&self, x: i32, z: i32) -> (ProtoChunk, NoiseChunk, E::Aquifer) {
        let around: Vec<QuartBiomes> = (0..9).map(|i| self.quart_biomes(x + i / 3 - 1, z + i % 3 - 1)).collect();
        self.fill_noise(x, z, std::array::from_fn(|i| &around[i]), self.terrain.noise_chunk(x, z))
    }

    fn fill_noise(&self, x: i32, z: i32, around: [&QuartBiomes; 9], noise: NoiseChunk) -> (ProtoChunk, NoiseChunk, E::Aquifer) {
        self.edition.fill_noise(self, x, z, around, noise)
    }

    fn build_surface(&self, noise: &NoiseChunk, chunk: &mut ProtoChunk) {
        self.edition.build_surface(self, noise, chunk);
    }

    fn apply_carvers(&self, noise: &NoiseChunk, aquifer: &mut E::Aquifer, chunk: &mut ProtoChunk) {
        self.edition.apply_carvers(self, noise, aquifer, chunk);
    }

    fn decorate(&self, owner_x: i32, owner_z: i32, chunks: [ProtoChunk; 9]) -> Changes {
        self.edition.decorate(self, owner_x, owner_z, chunks)
    }

    pub fn find_spawn(&self) -> IVec3 {
        for radius in 0i32..32 {
            for cz in -radius..=radius {
                for cx in -radius..=radius {
                    if cx.abs() != radius && cz.abs() != radius {
                        continue;
                    }
                    let (x, z) = ((cx << 4) + 8, (cz << 4) + 8);
                    if self.terrain.column(x, z).continents < -0.11 || self.terrain.preliminary_surface_level(x, z) < SEA_LEVEL as f32 {
                        continue;
                    }
                    let (mut chunk, noise, _) = self.standalone_noise(cx, cz);
                    self.build_surface(&noise, &mut chunk);
                    let y = chunk.surface_height(8, 8);
                    if chunk.get(8, y, 8) != self.water && y >= SEA_LEVEL {
                        return IVec3::new(x, y + 1, z);
                    }
                }
            }
        }
        IVec3::new(8, SEA_LEVEL + 1, 8)
    }
}

