use crate::level::bit_array::BitArray;
use bedrock::protocol::error::ProtoCodecError;
use bedrock::protocol::{ProtoCodec, ProtoCodecLE, ProtoCodecVAR};
use indexmap::IndexSet;
use std::io::{Read, Write};
use vek::num_traits::Zero;

pub struct Palette {
    palette: IndexSet<i32>,
    data: Option<BitArray<4096>>,
}

impl Palette {
    pub fn new(value: i32) -> Self {
        Self {
            palette: {
                let mut set = IndexSet::new();
                set.insert(value);
                set
            },
            data: None,
        }
    }

    pub fn get(&self, index: usize) -> i32 {
        match &self.data {
            None => self.palette[0],
            Some(indices) => self.palette[indices.get(index) as usize],
        }
    }

    pub fn set(&mut self, index: usize, value: i32) {
        let (palette_index, _) = self.palette.insert_full(value);

        if palette_index.is_zero() && self.data.is_none() {
            return;
        }

        let indices = self.data.get_or_insert_with(BitArray::new);
        indices.set(index, palette_index as u16);
    }
}

impl ProtoCodec for Palette {
    fn serialize<W: Write>(&self, stream: &mut W) -> Result<(), ProtoCodecError> {
        match &self.data {
            None => {
                u8::serialize(&0x01, stream)?;
                <i32 as ProtoCodecVAR>::serialize(&self.palette[0], stream)?;
            }
            Some(data) => {
                u8::serialize(&((data.get_bits() << 1) | 1), stream)?;
                for block in data.get_blocks() {
                    <u32 as ProtoCodecLE>::serialize(block, stream)?;
                }

                <i32 as ProtoCodecVAR>::serialize(&(self.palette.len() as i32), stream)?;
                for id in &self.palette {
                    <i32 as ProtoCodecVAR>::serialize(id, stream)?;
                }
            }
        }
        Ok(())
    }

    fn deserialize<R: Read>(_stream: &mut R) -> Result<Self, ProtoCodecError> {
        unimplemented!()
    }

    fn size_hint(&self) -> usize {
        unimplemented!()
    }
}
