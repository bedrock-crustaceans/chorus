use super::super::{NetherChunk, NetherGenerator};
use crate::shared::blocks::AIR;
use crate::shared::cell_phase::{CellOutput, CellPhase};
use crate::shared::chunk::store::{give_scratch, take_scratch};
use crate::shared::rng::mersenne::MersenneTwister;
use chorus_level::generator::error::PhaseError;
use chorus_level::generator::phase::{Dependency, Phase, PhaseContext, SAME_CELL};
use glam::IVec2;

impl NetherGenerator {
    pub(in super::super) fn fill_shape(&self, chunk_x: i32, chunk_z: i32) -> NetherChunk {
        let mut chunk = NetherChunk::empty(self.definition);
        let (stone, water) = (self.ids.stone, self.ids.water);
        let mut blocks = take_scratch(chunk.cells(), AIR);
        let (min_y, top) = (chunk.min_y, chunk.definition().top());
        self.noises.fill_shape(chunk_x, chunk_z, |x, y, z, cell| {
            if !(min_y..top).contains(&y) {
                return;
            }
            blocks[((y - min_y) as usize) * 256 + (z << 4) + x] = match cell {
                super::super::terrain::Cell::Stone => stone,
                super::super::terrain::Cell::Water => water,
                super::super::terrain::Cell::Air => AIR,
            };
        });
        chunk.fill_empty(&blocks);
        give_scratch(blocks);
        chunk
    }
    pub(in super::super) fn place_floor(&self, chunk_x: i32, chunk_z: i32, mut chunk: NetherChunk) -> (NetherChunk, MersenneTwister) {
        let bedrock = self.ids.bedrock;
        let mut random = super::super::surface::chunk_random(chunk_x, chunk_z);
        super::super::surface::place_bedrock(&mut random, |x, y, z| chunk.set(x, y, z, bedrock));
        (chunk, random)
    }
    pub(in super::super) fn build_surface(&self, chunk: &mut NetherChunk, chunk_x: i32, chunk_z: i32, column_biomes: &[i32; 256], random: &mut MersenneTwister) {
        let context = super::super::surface::Context { noises: &self.noises, ids: &self.ids };
        self.surface.build(chunk, &context, chunk_x, chunk_z, column_biomes, random);
    }
    pub(in super::super) fn carve(&self, chunk: &mut NetherChunk, chunk_x: i32, chunk_z: i32) {
        self.carver.carve(chunk, chunk_x, chunk_z, &self.ids);
    }
}

pub struct BiomeCells;

impl CellOutput<NetherGenerator> for BiomeCells {
    type Value = [i32; 256];

    const NAME: &'static str = "BiomePhase";

    const RETAIN: usize = 1024;

    fn compute(generator: &NetherGenerator, cell: IVec2) -> [i32; 256] {
        generator.biomes.chunk_biomes(cell.x, cell.y)
    }
}

pub type BiomePhase = CellPhase<BiomeCells>;

pub struct ShapePhase;

impl Phase<NetherGenerator> for ShapePhase {
    type Output = NetherChunk;

    const RETAIN: usize = 0;

    fn run(generator: &NetherGenerator, cell: IVec2, _inputs: &mut PhaseContext<NetherGenerator>) -> Result<NetherChunk, PhaseError> {
        Ok(generator.fill_shape(cell.x, cell.y))
    }
}

pub struct FloorPhase;

impl Phase<NetherGenerator> for FloorPhase {
    type Output = (NetherChunk, MersenneTwister);

    const RETAIN: usize = 0;

    fn dependencies() -> Vec<Dependency<NetherGenerator>> {
        vec![Dependency::on::<ShapePhase>(SAME_CELL)]
    }

    fn run(generator: &NetherGenerator, cell: IVec2, context: &mut PhaseContext<NetherGenerator>) -> Result<(NetherChunk, MersenneTwister), PhaseError> {
        let shape = context.take::<ShapePhase>(cell)?;
        Ok(generator.place_floor(cell.x, cell.y, shape))
    }
}

pub struct SurfacePhase;

impl Phase<NetherGenerator> for SurfacePhase {
    type Output = NetherChunk;

    const RETAIN: usize = 0;

    fn dependencies() -> Vec<Dependency<NetherGenerator>> {
        vec![Dependency::on::<FloorPhase>(SAME_CELL), Dependency::on::<BiomePhase>(SAME_CELL)]
    }

    fn run(generator: &NetherGenerator, cell: IVec2, context: &mut PhaseContext<NetherGenerator>) -> Result<NetherChunk, PhaseError> {
        let (mut chunk, mut random) = context.take::<FloorPhase>(cell)?;
        let biomes = context.get::<BiomePhase>(cell)?;
        generator.build_surface(&mut chunk, cell.x, cell.y, &biomes, &mut random);
        Ok(chunk)
    }
}

pub struct CarverPhase;

impl Phase<NetherGenerator> for CarverPhase {
    type Output = NetherChunk;

    const RETAIN: usize = 1024;

    fn dependencies() -> Vec<Dependency<NetherGenerator>> {
        vec![Dependency::on::<SurfacePhase>(SAME_CELL)]
    }

    fn run(generator: &NetherGenerator, cell: IVec2, context: &mut PhaseContext<NetherGenerator>) -> Result<NetherChunk, PhaseError> {
        let mut chunk = context.take::<SurfacePhase>(cell)?;
        generator.carve(&mut chunk, cell.x, cell.y);
        chunk.compact();
        Ok(chunk)
    }
}
