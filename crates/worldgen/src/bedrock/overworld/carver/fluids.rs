use crate::bedrock::overworld::aquifer::{Aquifer, AquiferNoises, AquiferSampler, FluidTick, SurfaceWindow};
use crate::bedrock::overworld::terrain::BedrockTerrain;
use crate::shared::blocks::BlockId;
use crate::shared::chunk::proto::ProtoChunk;

pub struct Fluids<'a> {
    noises: &'a AquiferNoises,
    terrain: &'a BedrockTerrain,
    chunk_x: i32,
    chunk_z: i32,
    window: SurfaceWindow,
    sampler: Option<AquiferSampler<'a>>,
    ticks: &'a mut Aquifer,
    air: BlockId,
    water: BlockId,
    lava: BlockId,
}

impl<'a> Fluids<'a> {
    pub fn new(noises: &'a AquiferNoises, terrain: &'a BedrockTerrain, chunk: &ProtoChunk, ticks: &'a mut Aquifer, air: BlockId) -> Self {
        Self {
            noises,
            terrain,
            chunk_x: chunk.x,
            chunk_z: chunk.z,
            window: ticks.window.clone(),
            sampler: None,
            ticks,
            air,
            water: chunk.water,
            lava: chunk.lava,
        }
    }

    pub fn empty_cell(&mut self, chunk: &mut ProtoChunk, local_x: i32, y: i32, local_z: i32) {
        let (noises, terrain, chunk_x, chunk_z) = (self.noises, self.terrain, self.chunk_x, self.chunk_z);
        let window = &self.window;
        let sampler = self.sampler.get_or_insert_with(|| AquiferSampler::new(noises, terrain, window.clone(), chunk_x, chunk_z));
        let substance = sampler.substance((chunk_x << 4) + local_x, y, (chunk_z << 4) + local_z);
        if substance.barrier > 0.0 {
            return;
        }
        let block = if y >= substance.level {
            self.air
        } else if substance.lava {
            self.lava
        } else {
            self.water
        };
        if block != self.air && substance.schedule_tick {
            self.ticks.scheduled_ticks.push(FluidTick {
                x: local_x as u8,
                z: local_z as u8,
                y: y as i16,
                lava: substance.lava,
            });
        }
        chunk.set(local_x, y, local_z, block);
    }
}
