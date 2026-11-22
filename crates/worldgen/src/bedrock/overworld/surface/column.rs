use crate::shared::blocks::BlockId;
use crate::shared::chunk::proto::ProtoChunk;

pub struct Column<'a> {
    chunk: &'a mut ProtoChunk,
    x: i32,
    z: i32,
}

impl<'a> Column<'a> {
    pub fn new(chunk: &'a mut ProtoChunk, x: i32, z: i32) -> Self {
        Self { chunk, x, z }
    }

    pub fn min_y(&self) -> i32 {
        self.chunk.min_y
    }

    pub fn height(&self) -> i32 {
        self.chunk.height
    }

    pub fn max_y(&self) -> i32 {
        self.chunk.max_y()
    }

    pub fn get(&self, y: i32) -> BlockId {
        self.chunk.get(self.x, y, self.z)
    }

    pub fn set(&mut self, y: i32, block: BlockId) {
        self.chunk.set(self.x, y, self.z, block);
    }

    pub fn get_local(&self, y: i32) -> BlockId {
        self.get(y + self.chunk.min_y)
    }

    pub fn set_local(&mut self, y: i32, block: BlockId) {
        self.set(y + self.chunk.min_y, block);
    }
}
