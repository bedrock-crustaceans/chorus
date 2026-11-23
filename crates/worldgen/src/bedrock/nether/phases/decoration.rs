use super::super::decorate;
use super::super::{CHUNK_AREA, CHUNK_MASK, CHUNK_WIDTH, NetherGenerator};
use super::structured::{StructureJoinPhase, Structured};
use super::terrain::BiomePhase;
use crate::bedrock::overworld::decoration_seed;
use crate::shared::CHUNK_NEIGHBORHOOD;
use crate::shared::decoration_class::{self, Decorates, neighborhood};
use crate::shared::decoration_window::{SLOTS, SourceChanges, window_chunks};
use crate::shared::rng::mersenne::MersenneTwister;
use chorus_level::generator::error::PhaseError;
use chorus_level::generator::phase::{Dependency, PhaseContext, SAME_CELL};
use glam::IVec2;
use std::sync::Arc;

impl NetherGenerator {
    pub(in super::super) fn decorate_source(&self, source: (i32, i32), structured: &[Arc<Structured>; 9], biome: i32, earlier: &[((i32, i32), &SourceChanges)]) -> SourceChanges {
        let mut chunks = window_chunks(source, earlier.iter().copied(), |slot| (*structured[slot].chunk).clone());
        let heights: [Arc<[i32; CHUNK_AREA]>; SLOTS] = std::array::from_fn(|slot| structured[slot].heights.clone());
        let decoration_seed = decoration_seed(self.seed, source.0, source.1);
        let mut chunk_random = MersenneTwister::new(decoration_seed);
        let materials = self.materials();
        let mut view = decorate::View::new(&mut chunks, &heights, &self.blocks, &self.ids, source);
        decorate::hardcoded(&mut chunk_random, source, &mut view, &self.ids, &self.ores);
        let mut rule_random = MersenneTwister::new(decoration_seed);
        self.rules.place(decoration_seed, &mut rule_random, source, biome, &mut view, materials.as_ref());
        decorate::debris(&mut rule_random, source, &mut view, &self.ids);
        view.take_changes()
    }
}

pub type DecoratePhase<const CLASS: u8> = decoration_class::DecoratePhase<NetherGenerator, CLASS>;

impl Decorates for NetherGenerator {
    type Decoration = SourceChanges;

    const RETAIN: usize = 1024;

    fn inputs() -> Vec<Dependency<NetherGenerator>> {
        vec![Dependency::on::<StructureJoinPhase>(CHUNK_NEIGHBORHOOD), Dependency::on::<BiomePhase>(SAME_CELL)]
    }

    fn decorate(&self, cell: IVec2, context: &PhaseContext<NetherGenerator>, earlier: &[((i32, i32), &SourceChanges)]) -> Result<SourceChanges, PhaseError> {
        let structured = neighborhood::<NetherGenerator, StructureJoinPhase>(context, cell)?;
        let biomes = context.get::<BiomePhase>(cell)?;
        Ok(self.decorate_source((cell.x, cell.y), &structured, biomes[(CHUNK_MASK * CHUNK_WIDTH + CHUNK_MASK) as usize], earlier))
    }
}
