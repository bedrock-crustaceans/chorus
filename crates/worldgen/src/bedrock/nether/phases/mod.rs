mod decoration;
mod finish;
mod structured;
mod terrain;

pub use decoration::DecoratePhase;
pub use finish::{ChunkPhase, FluidTicksPhase};
pub use structured::{PortalStartPhase, StartsPhase, StructureBodyPhase, StructureJoinPhase, Structured};
pub use terrain::{CarverPhase, FloorPhase, ShapePhase, SurfacePhase};

pub(super) const CENTRE: usize = 4;

pub(super) const REACH_TWO: &[(i32, i32)] = &[
    (-2, -2),
    (-2, -1),
    (-2, 0),
    (-2, 1),
    (-2, 2),
    (-1, -2),
    (-1, -1),
    (-1, 0),
    (-1, 1),
    (-1, 2),
    (0, -2),
    (0, -1),
    (0, 0),
    (0, 1),
    (0, 2),
    (1, -2),
    (1, -1),
    (1, 0),
    (1, 1),
    (1, 2),
    (2, -2),
    (2, -1),
    (2, 0),
    (2, 1),
    (2, 2),
];
