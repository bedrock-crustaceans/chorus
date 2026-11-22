use crate::shared::decoration_window::Writes;

mod ores;
mod placements;
mod view;

use crate::shared::CHUNK_WIDTH;
pub use ores::{LegacyOres, debris};
pub use placements::hardcoded;
pub use view::{View, light_heights, opaque_solid, solid, thin};

const FACES: [[i32; 3]; 6] = [[0, -1, 0], [0, 1, 0], [-1, 0, 0], [1, 0, 0], [0, 0, -1], [0, 0, 1]];

#[derive(Clone, Copy)]
pub struct RoutedTick {
    pub target: (i32, i32),
    pub pos: [i32; 3],
    pub lava: bool,
    pub delay: u64,
}

pub struct SourceDecoration {
    pub writes: Writes,
    pub routed: Vec<RoutedTick>,
}
