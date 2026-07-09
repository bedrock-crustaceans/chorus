use crate::level::bit_array::bit_array::BitArray;
use crate::level::bit_array::bit_array_version::BitArrayVersion;
use bedrock::protocol::error::ProtoCodecError;
use bedrock::protocol::{ProtoCodec, ProtoCodecLE, ProtoCodecVAR};
use std::io::ErrorKind::InvalidData;
use std::io::{Read, Write};

pub struct Palette<V: PartialEq + Clone, const N: usize> {
    palette: Vec<V>,
    bit_array: BitArray,
}

impl<V: PartialEq + Clone, const N: usize> Palette<V, N> {
    pub fn set(&mut self, index: usize, value: V) {
        let idx = self.index_for(value);
        self.bit_array.set(index, idx as u32);
    }

    fn index_for(&mut self, value: V) -> usize {
        if let Some(i) = self.palette.iter().position(|v| *v == value) {
            return i;
        }

        let new_index = self.palette.len();
        self.palette.push(value);

        if new_index > self.bit_array.version.max_value() as usize {
            if let Some(next) = self.bit_array.version.next() {
                self.resize(next);
            }
        }

        new_index
    }

    fn resize(&mut self, new_version: BitArrayVersion) {
        let mut new_array = BitArray::new(new_version, self.bit_array.size);
        for i in 0..self.bit_array.size {
            new_array.set(i, self.bit_array.get(i));
        }
        self.bit_array = new_array;
    }
}

impl<V: ProtoCodec + PartialEq + Clone, const N: usize> ProtoCodec for Palette<V, N> {
    fn serialize<W: Write>(&self, stream: &mut W) -> Result<(), ProtoCodecError> {
        u8::serialize(&((self.bit_array.version.bits() as u8) << 1), stream)?;
        for chunk in &self.bit_array.chunks {
            <u32 as ProtoCodecLE>::serialize(&chunk, stream)?;
        }

        <i32 as ProtoCodecVAR>::serialize(&(self.palette.len() as i32), stream)?;
        for value in self.palette.iter() {
            value.serialize(stream)?;
        }

        Ok(())
    }

    fn deserialize<R: Read>(stream: &mut R) -> Result<Self, ProtoCodecError> {
        let header = u8::deserialize(stream)?;
        let version = BitArrayVersion::get_version(header >> 1).ok_or(ProtoCodecError::IOError(InvalidData.into()))?;

        let num_words = version.get_words_for_size(N);

        let mut words = Vec::with_capacity(num_words as usize);
        for _ in 0..num_words {
            words.push(<i32 as ProtoCodecLE>::deserialize(stream)?);
        }

        let bit_array = BitArray::new(version, words.len()); // TODO: from words instead of new

        let states = <i32 as ProtoCodecVAR>::deserialize(stream)?;
        let mut palette = vec![];
        for _ in 0..states {
            palette.push(V::deserialize(stream)?);
        }

        Ok(Self { palette, bit_array })
    }

    fn size_hint(&self) -> usize {
        size_of::<u8>() + self.bit_array.chunks.len() * size_of::<i32>() + size_of::<i32>() + self.palette.iter().map(|v| v.size_hint()).sum::<usize>()
    }
}
