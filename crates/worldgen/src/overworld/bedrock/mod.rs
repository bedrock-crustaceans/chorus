use super::java::aquifer::Aquifer;
use super::java::surface::Floor;
use super::java::{Java, Rules};
use super::shared::blocks::{BlockId, BlockTable};
use super::shared::proto::{ProtoChunk, QuartBiomes};
use super::shared::random::PositionalFactory;
use super::shared::region::Changes;
use super::shared::shape::Shapes;
use super::shared::terrain::NoiseChunk;
use super::{Edition, OverworldGenerator};

const RULES: Rules = Rules {
    fuzzy_biomes: false,
    floor: Floor::Columns,
};

pub struct Bedrock {
    java: Java,
}

impl Edition for Bedrock {
    type Aquifer = Aquifer;

    fn new(seed: i64, random: &PositionalFactory, table: &mut BlockTable) -> Self {
        Self {
            java: Java::create(seed, random, table, RULES),
        }
    }

    fn fill_noise(&self, generator: &OverworldGenerator<Self>, x: i32, z: i32, around: [&QuartBiomes; 9], noise: NoiseChunk) -> (ProtoChunk, NoiseChunk, Aquifer) {
        self.java.fill_noise(generator, x, z, around, noise)
    }

    fn build_surface(&self, generator: &OverworldGenerator<Self>, noise: &NoiseChunk, chunk: &mut ProtoChunk) {
        self.java.build_surface(generator, noise, chunk);
    }

    fn apply_carvers(&self, generator: &OverworldGenerator<Self>, noise: &NoiseChunk, aquifer: &mut Aquifer, chunk: &mut ProtoChunk) {
        self.java.apply_carvers(generator, noise, aquifer, chunk);
    }

    fn decorate(&self, generator: &OverworldGenerator<Self>, owner_x: i32, owner_z: i32, chunks: [ProtoChunk; 9]) -> Changes {
        self.java.decorate(generator, owner_x, owner_z, chunks)
    }

    fn shapes(&self, water: BlockId) -> Shapes<'_> {
        self.java.shapes(water)
    }
}
