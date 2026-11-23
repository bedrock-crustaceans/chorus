mod biome;
mod carver;
mod decorate;
mod feature;
mod generator;
mod locate;
mod noise;
pub mod phases;
mod rules;
mod structure;
mod surface;
mod terrain;

use crate::shared::chunk::proto::ProtoChunk;

use crate::shared::{CHUNK_AREA, CHUNK_MASK, CHUNK_WIDTH};
pub use generator::{NetherBlocks, NetherGenerator};
pub(crate) use terrain::{SEA_LEVEL, TERRAIN_HEIGHT};

pub type NetherChunk = ProtoChunk;
