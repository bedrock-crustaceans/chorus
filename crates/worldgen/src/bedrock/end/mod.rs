mod cities;
mod decorate;
mod generator;
mod locate;
pub mod phases;
mod spikes;
mod terrain;

use crate::shared::CHUNK_MASK;
use crate::shared::blocks::BlockId;
use crate::shared::chunk::column::ColumnChunk;

pub use generator::{EndBlocks, EndGenerator, EndWrites};
pub(crate) use spikes::Spike;

pub const AIR: BlockId = crate::shared::blocks::AIR;
const TERRAIN_HEIGHT: i32 = terrain::HEIGHT;

pub type EndChunk = ColumnChunk;

impl EndChunk {
    fn column_tops(&self) -> [i32; 256] {
        let mut tops = [0i32; 256];
        for x in 0..16 {
            for z in 0..16 {
                tops[(z * 16 + x) as usize] = (self.min_y()..TERRAIN_HEIGHT.min(self.definition().top()))
                    .rev()
                    .find(|&y| self.get(x, y, z) != AIR)
                    .map_or(0, |y| y + 1);
            }
        }
        tops
    }
}
