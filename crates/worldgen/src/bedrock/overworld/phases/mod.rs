use super::OverworldGenerator;
use chorus_level::generator::dimension::Generator;
use chorus_level::generator::engine::LocateJob;
use chorus_level::generator::phase::{Dependency, Phase};
use std::sync::Arc;

mod feature;
mod finish;
mod structure;
mod terrain;

pub use feature::{FeaturePhase0, FeaturePhase1, FeaturePhase2, FeaturePhase3};
pub use finish::{ChunkPhase, ColumnPhase, PostProcessPhase};
pub use structure::{HeightsPhase, StartsPhase, StructurePhase};
pub use terrain::{BiomePhase, CarverPhase, CornerPhase, FillPhase, NoisePhase, RawTerrainPhase, SurfaceHintsPhase, SurfacePhase};

impl Generator for OverworldGenerator {
    type Terminal = ChunkPhase;

    fn locate_job(self: &Arc<Self>, name: &str, from: (i32, i32), radius: i32) -> Option<Box<dyn LocateJob<Self>>> {
        super::locate::job(self.clone(), name, from, radius)
    }

    fn fluid_rules(&self) -> Option<chorus_level::fluid::FluidRules> {
        OverworldGenerator::fluid_rules(self)
    }
}

pub(crate) fn chain<P: Phase<OverworldGenerator>>(offsets: &'static [(i32, i32)]) -> Dependency<OverworldGenerator> {
    Dependency::on::<P>(offsets)
}
