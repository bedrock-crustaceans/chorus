mod canyon;
mod carve;
mod cave;
mod classes;
mod fluids;
mod trig;

#[cfg(test)]
mod tests;

use super::aquifer::{Aquifer, AquiferNoises};
use super::decoration::seed::large_feature_multipliers;
use super::terrain::BedrockTerrain;
use crate::shared::blocks::{BlockTable, Blocks};
use crate::shared::chunk::proto::ProtoChunk;
use crate::shared::rng::mersenne::MersenneTwister;
use canyon::Step;
use cave::{CaveSettings, EXTRA_CAVE_CARVER, Segment, UNDERGROUND_CAVE_CARVER};
use classes::Classes;
use fluids::Fluids;

const SOURCE_RANGE: i32 = 8;
const CAVE_CARVERS: [&CaveSettings; 2] = [&UNDERGROUND_CAVE_CARVER, &EXTRA_CAVE_CARVER];

pub struct Carvers {
    level_seed: u32,
    x_multiplier: u32,
    z_multiplier: u32,
    classes: Classes,
}

#[derive(Default)]
struct Scratch {
    segments: Vec<Segment>,
    steps: Vec<Step>,
}

impl Carvers {
    pub fn new(level_seed: u32, table: &mut BlockTable) -> Self {
        let (x_multiplier, z_multiplier) = large_feature_multipliers(level_seed);
        Self {
            level_seed,
            x_multiplier,
            z_multiplier,
            classes: Classes::new(table),
        }
    }

    fn source_seed(&self, chunk_x: i32, chunk_z: i32) -> u32 {
        self.level_seed ^ (chunk_z as u32).wrapping_mul(self.z_multiplier).wrapping_add((chunk_x as u32).wrapping_mul(self.x_multiplier))
    }

    pub fn carve(&self, blocks: &Blocks, noises: &AquiferNoises, terrain: &BedrockTerrain, aquifer: &mut Aquifer, chunk: &mut ProtoChunk) {
        let mut fluids = Fluids::new(noises, terrain, chunk, aquifer, self.classes.air);
        let mut scratch = Scratch::default();
        let (chunk_x, chunk_z) = (chunk.x, chunk.z);
        let min_y = chunk.min_y;
        let sources = || (chunk_x - SOURCE_RANGE..=chunk_x + SOURCE_RANGE).flat_map(move |x| (chunk_z - SOURCE_RANGE..=chunk_z + SOURCE_RANGE).map(move |z| (x, z)));
        let words: Vec<[u32; 4]> = sources().map(|(x, z)| MersenneTwister::leading_words::<4>(self.source_seed(x, z))).collect();
        for settings in CAVE_CARVERS {
            for ((x, z), words) in sources().zip(&words) {
                if !settings.starts(*words) {
                    continue;
                }
                settings.segments(&mut MersenneTwister::new(self.source_seed(x, z)), x, z, &mut scratch.segments);
                for segment in scratch.segments.iter().filter(|segment| carve::reaches(segment, chunk_x, chunk_z)) {
                    carve::carve_cave(&self.classes, blocks, &mut fluids, chunk, segment, settings.height_limit);
                }
            }
        }
        for ((x, z), words) in sources().zip(&words) {
            if !canyon::starts(words[0]) {
                continue;
            }
            if let Some(canyon) = canyon::canyon(&mut MersenneTwister::new(self.source_seed(x, z)), x, z, min_y, &mut scratch.steps) {
                carve::carve_canyon(&self.classes, blocks, &mut fluids, chunk, &canyon, &scratch.steps);
            }
        }
    }
}
