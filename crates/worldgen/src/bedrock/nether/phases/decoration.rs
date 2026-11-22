use super::super::decorate::{self, SourceDecoration};
use super::super::{CHUNK_AREA, CHUNK_MASK, CHUNK_WIDTH, NetherGenerator, liquid};
use super::structured::{StructureJoinPhase, Structured};
use crate::bedrock::overworld::decoration_seed;
use crate::shared::CHUNK_NEIGHBORHOOD;
use crate::shared::decoration_class::{self, Decorates, neighborhood};
use crate::shared::decoration_window::{SLOTS, window_chunks};
use crate::shared::rng::mersenne::MersenneTwister;
use chorus_level::generator::error::PhaseError;
use chorus_level::generator::phase::{Dependency, PhaseContext};
use glam::IVec2;
use std::sync::Arc;

impl NetherGenerator {
    pub(in super::super) fn decorate_source(&self, source: (i32, i32), structured: &[Arc<Structured>; 9], earlier: &[((i32, i32), &SourceDecoration)]) -> SourceDecoration {
        let mut chunks = window_chunks(source, earlier.iter().map(|(cell, decoration)| (*cell, &decoration.writes)), |slot| (*structured[slot].chunk).clone());
        let heights: [Arc<[i32; CHUNK_AREA]>; SLOTS] = std::array::from_fn(|slot| structured[slot].heights.clone());
        let decoration_seed = decoration_seed(self.seed, source.0, source.1);
        let mut chunk_random = MersenneTwister::new(decoration_seed);
        let biome = self.biomes.biome(source.0 * CHUNK_WIDTH + CHUNK_MASK, source.1 * CHUNK_WIDTH + CHUNK_MASK);
        let materials = self.materials();
        let mut view = decorate::View::new(&mut chunks, &heights, &self.blocks, &self.ids, source);
        decorate::hardcoded(&mut chunk_random, source, &mut view, &self.ids, &self.ores);
        let mut rule_random = MersenneTwister::new(decoration_seed);
        self.rules.place(decoration_seed, &mut rule_random, source, biome, &mut view, materials.as_ref());
        decorate::debris(&mut rule_random, source, &mut view, &self.ids);
        liquid::instatick(&mut view);
        SourceDecoration {
            writes: view.take_writes(),
            routed: std::mem::take(&mut view.routed),
        }
    }
}

pub type DecoratePhase<const CLASS: u8> = decoration_class::DecoratePhase<NetherGenerator, CLASS>;

impl Decorates for NetherGenerator {
    type Decoration = SourceDecoration;

    const RETAIN: usize = 1024;

    fn inputs() -> Vec<Dependency<NetherGenerator>> {
        vec![Dependency::on::<StructureJoinPhase>(CHUNK_NEIGHBORHOOD)]
    }

    fn decorate(&self, cell: IVec2, context: &PhaseContext<NetherGenerator>, earlier: &[((i32, i32), &SourceDecoration)]) -> Result<SourceDecoration, PhaseError> {
        let structured = neighborhood::<NetherGenerator, StructureJoinPhase>(context, cell)?;
        Ok(self.decorate_source((cell.x, cell.y), &structured, earlier))
    }
}
