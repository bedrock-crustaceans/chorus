use crate::shared::footprint;

mod decoration;
mod finish;
mod structured;
mod terrain;

pub use decoration::DecoratePhase;
pub use finish::ChunkPhase;
pub use structured::{PortalStartPhase, StartsPhase, StructureBodyPhase, StructureJoinPhase, Structured};
pub use terrain::{BiomePhase, CarverPhase, FloorPhase, ShapePhase, SurfacePhase};

pub(super) const CENTRE: usize = 4;

pub(super) const REACH_TWO: &[(i32, i32)] = &footprint::square::<{ footprint::cells(2) }>(2);
