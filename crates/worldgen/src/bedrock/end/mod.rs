mod cities;
mod decorate;
mod generator;
mod locate;
pub mod phases;
mod spikes;
mod terrain;

use crate::shared::CHUNK_MASK;
use crate::shared::blocks::BlockId;
use crate::shared::chunk::proto::ProtoChunk;

pub use generator::{EndBlocks, EndGenerator, EndWrites};
pub(crate) use spikes::Spike;

pub const AIR: BlockId = crate::shared::blocks::AIR;
const TERRAIN_HEIGHT: i32 = terrain::HEIGHT;

pub type EndChunk = ProtoChunk;

impl EndChunk {
    fn column_tops(&self) -> [i32; 256] {
        self.column_tops_below(TERRAIN_HEIGHT.min(self.definition().top()))
    }
}
