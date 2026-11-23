mod canyon;
mod carve;
mod cave;
mod classes;
mod fluids;
mod trig;

#[cfg(test)]
mod tests;

use std::sync::OnceLock;

use super::aquifer::{Aquifer, AquiferNoises};
use super::decoration::seed::large_feature_multipliers;
use super::terrain::BedrockTerrain;
use crate::shared::blocks::{BlockTable, Blocks};
use crate::shared::chunk::proto::ProtoChunk;
use crate::shared::rng::mersenne::MersenneTwister;
use canyon::{Canyon, Step};
use cave::{CaveSettings, EXTRA_CAVE_CARVER, Segment, UNDERGROUND_CAVE_CARVER};
use classes::Classes;
use fluids::Fluids;

const SOURCE_RANGE: i32 = 8;
const CACHE_SIDE: usize = 64;
const CAVE_CARVERS: [&CaveSettings; 2] = [&UNDERGROUND_CAVE_CARVER, &EXTRA_CAVE_CARVER];

type Key = (i32, i32, i32);

pub struct Carvers {
    level_seed: u32,
    x_multiplier: u32,
    z_multiplier: u32,
    classes: Classes,
    plans: Box<[OnceLock<(Key, SourcePlan)>]>,
}

struct SourcePlan {
    caves: [Vec<Segment>; 2],
    canyon: Option<Box<(Canyon, Vec<Step>)>>,
}

impl Carvers {
    pub fn new(level_seed: u32, table: &mut BlockTable) -> Self {
        let (x_multiplier, z_multiplier) = large_feature_multipliers(level_seed);
        Self {
            level_seed,
            x_multiplier,
            z_multiplier,
            classes: Classes::new(table),
            plans: (0..CACHE_SIDE * CACHE_SIDE).map(|_| OnceLock::new()).collect(),
        }
    }

    fn source_seed(&self, chunk_x: i32, chunk_z: i32) -> u32 {
        self.level_seed ^ (chunk_z as u32).wrapping_mul(self.z_multiplier).wrapping_add((chunk_x as u32).wrapping_mul(self.x_multiplier))
    }

    fn plan(&self, x: i32, z: i32, min_y: i32) -> SourcePlan {
        let seed = self.source_seed(x, z);
        let words = MersenneTwister::leading_words::<4>(seed);
        let mut caves = [Vec::new(), Vec::new()];
        for (settings, segments) in CAVE_CARVERS.into_iter().zip(&mut caves) {
            if settings.starts(words) {
                settings.segments(&mut MersenneTwister::new(seed), x, z, segments);
            }
        }
        let canyon = if canyon::starts(words[0]) {
            let mut steps = Vec::new();
            canyon::canyon(&mut MersenneTwister::new(seed), x, z, min_y, &mut steps).map(|canyon| Box::new((canyon, steps)))
        } else {
            None
        };
        SourcePlan { caves, canyon }
    }

    fn with_plan(&self, x: i32, z: i32, min_y: i32, apply: impl FnOnce(&SourcePlan)) {
        let key = (x, z, min_y);
        let slot = &self.plans[(x as usize & (CACHE_SIDE - 1)) | ((z as usize & (CACHE_SIDE - 1)) * CACHE_SIDE)];
        let cached = slot.get_or_init(|| (key, self.plan(x, z, min_y)));
        if cached.0 == key {
            apply(&cached.1);
        } else {
            apply(&self.plan(x, z, min_y));
        }
    }

    pub fn carve(&self, blocks: &Blocks, noises: &AquiferNoises, terrain: &BedrockTerrain, aquifer: &mut Aquifer, chunk: &mut ProtoChunk) {
        let mut fluids = Fluids::new(noises, terrain, chunk, aquifer, self.classes.air);
        let (chunk_x, chunk_z) = (chunk.x, chunk.z);
        let min_y = chunk.min_y;
        let sources = || (chunk_x - SOURCE_RANGE..=chunk_x + SOURCE_RANGE).flat_map(move |x| (chunk_z - SOURCE_RANGE..=chunk_z + SOURCE_RANGE).map(move |z| (x, z)));
        for (index, settings) in CAVE_CARVERS.into_iter().enumerate() {
            for (x, z) in sources() {
                self.with_plan(x, z, min_y, |plan| {
                    for segment in plan.caves[index].iter().filter(|segment| carve::reaches(segment, chunk_x, chunk_z)) {
                        carve::carve_cave(&self.classes, blocks, &mut fluids, chunk, segment, settings.height_limit);
                    }
                });
            }
        }
        for (x, z) in sources() {
            self.with_plan(x, z, min_y, |plan| {
                if let Some((canyon, steps)) = plan.canyon.as_deref() {
                    carve::carve_canyon(&self.classes, blocks, &mut fluids, chunk, canyon, steps);
                }
            });
        }
    }
}
