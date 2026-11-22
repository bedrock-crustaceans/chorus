mod corners;
mod layers;
mod settings;

pub use corners::{Corner, CornerColumns, NoiseChunk};
pub use layers::SettingsLayers;
pub use settings::{BiomeSource, Checkerboard, ClimateScale, OverworldSettings, Preset, SettingsError, TerrainShape};

pub const CELL_WIDTH: i32 = 4;
pub const CELL_HEIGHT: i32 = 8;
