use super::super::{NetherChunk, NetherGenerator};
use crate::shared::blocks::AIR;
use crate::shared::rng::mersenne::MersenneTwister;
use chorus_level::generator::error::PhaseError;
use chorus_level::generator::phase::{Dependency, Phase, PhaseContext, SAME_CELL};
use glam::IVec2;

impl NetherGenerator {
    pub(in super::super) fn fill_shape(&self, chunk_x: i32, chunk_z: i32) -> NetherChunk {
        let mut chunk = NetherChunk::empty(self.definition);
        let (stone, water) = (self.ids.stone, self.ids.water);
        self.noises.fill_shape(chunk_x, chunk_z, |x, y, z, cell| {
            let block = match cell {
                super::super::terrain::Cell::Stone => stone,
                super::super::terrain::Cell::Water => water,
                super::super::terrain::Cell::Air => AIR,
            };
            chunk.set(x as i32, y, z as i32, block);
        });
        chunk
    }
    pub(in super::super) fn place_floor(&self, chunk_x: i32, chunk_z: i32, mut chunk: NetherChunk) -> (NetherChunk, MersenneTwister) {
        let bedrock = self.ids.bedrock;
        let mut random = super::super::surface::chunk_random(chunk_x, chunk_z);
        super::super::surface::place_bedrock(&mut random, |x, y, z| chunk.set(x, y, z, bedrock));
        (chunk, random)
    }
    pub(in super::super) fn build_surface(&self, chunk: &mut NetherChunk, chunk_x: i32, chunk_z: i32, random: &mut MersenneTwister) {
        let context = super::super::surface::Context {
            biomes: &self.biomes,
            noises: &self.noises,
            ids: &self.ids,
        };
        self.surface.build(chunk, &context, chunk_x, chunk_z, random);
    }
    pub(in super::super) fn carve(&self, chunk: &mut NetherChunk, chunk_x: i32, chunk_z: i32) {
        self.carver.carve(chunk, chunk_x, chunk_z, &self.ids);
    }
}

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
        vec![Dependency::on::<FloorPhase>(SAME_CELL)]
    }

    fn run(generator: &NetherGenerator, cell: IVec2, context: &mut PhaseContext<NetherGenerator>) -> Result<NetherChunk, PhaseError> {
        let (mut chunk, mut random) = context.take::<FloorPhase>(cell)?;
        generator.build_surface(&mut chunk, cell.x, cell.y, &mut random);
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
