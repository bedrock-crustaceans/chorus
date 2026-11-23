mod ores;
mod placements;
mod view;

use crate::shared::CHUNK_WIDTH;
pub use ores::{LegacyOres, debris};
pub use placements::hardcoded;
pub use view::{View, light_heights, opaque_solid, solid, thin};

const FACES: [[i32; 3]; 6] = [[0, -1, 0], [0, 1, 0], [-1, 0, 0], [1, 0, 0], [0, 0, -1], [0, 0, 1]];
