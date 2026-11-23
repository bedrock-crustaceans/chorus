pub const SEA_LEVEL: i32 = 63;

pub mod biome;
pub mod biome_documents;
pub mod blocks;
pub mod cell_phase;
pub mod chunk;
pub mod decoration_class;
pub mod decoration_window;
pub mod footprint;
pub mod hash;
pub mod math;
pub mod moss_carpet;
pub mod noise;
pub(crate) mod resources;
pub mod rng;
pub mod shape;
pub mod structure;
pub mod survival;
pub mod tags;
pub mod terrain;

pub const CHUNK_WIDTH: i32 = 16;
pub const CHUNK_MASK: i32 = 15;
pub const CHUNK_SHIFT: i32 = 4;
pub const CHUNK_AREA: usize = 16 * 16;

pub const CHUNK_NEIGHBORHOOD: &[(i32, i32)] = &[(-1, -1), (-1, 0), (-1, 1), (0, -1), (0, 0), (0, 1), (1, -1), (1, 0), (1, 1)];

pub(crate) fn into_array<T, const N: usize>(items: Vec<T>) -> [T; N] {
    items.try_into().unwrap_or_else(|_| unreachable!("the caller collected exactly {N} items"))
}
