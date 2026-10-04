use crate::bit_array::BitArray;
use bedrock::protocol::error::ProtoCodecError;
use bedrock::protocol::{ProtoCodec, ProtoCodecLE, ProtoCodecVAR};
use indexmap::IndexMap;
use indexmap::map::Entry;
use rustc_hash::FxBuildHasher;
use std::io::{Read, Write};

type PaletteMap = IndexMap<i32, u16, FxBuildHasher>;

#[derive(Clone)]
pub enum Palette {
    Uniform { value: i32 },
    Indexed { values: PaletteMap, indices: BitArray<4096> },
}

impl Palette {
    pub fn new(value: i32) -> Self {
        Self::Uniform { value }
    }

    pub fn from_blocks(blocks: &[i32; 4096]) -> Self {
        if blocks.iter().all(|&value| value == blocks[0]) {
            return Self::Uniform { value: blocks[0] };
        }

        let mut unique: Vec<(i32, u16)> = Vec::new();
        let mut block_indices = [0u16; 4096];
        let mut last: Option<(i32, usize)> = None;

        for (i, &value) in blocks.iter().enumerate() {
            let index = match last {
                Some((previous, index)) if previous == value => index,
                _ => match unique.iter().position(|&(v, _)| v == value) {
                    Some(index) => index,
                    None => {
                        unique.push((value, 0));
                        unique.len() - 1
                    }
                },
            };
            unique[index].1 += 1;
            last = Some((value, index));
            block_indices[i] = index as u16;
        }

        if unique.len() == 1 {
            return Self::Uniform { value: unique[0].0 };
        }

        let mut values = PaletteMap::default();
        for (value, count) in unique {
            values.insert(value, count);
        }

        let bits = BitArray::<4096>::bits_for((values.len() - 1) as u16);
        let mut indices = BitArray::<4096>::with_bits(bits);

        for (i, &index) in block_indices.iter().enumerate() {
            indices.set(i, index);
        }

        Self::Indexed { values, indices }
    }

    pub fn get(&self, index: usize) -> i32 {
        match &self {
            Self::Uniform { value } => *value,
            Self::Indexed { values, indices } => *values.get_index(indices.get(index) as usize).unwrap().0,
        }
    }

    pub fn count(&self, value: i32) -> u32 {
        match self {
            Self::Uniform { value: v } => {
                if *v == value {
                    4096
                } else {
                    0
                }
            }
            Self::Indexed { values, .. } => values.get(&value).copied().unwrap_or(0) as u32,
        }
    }

    pub fn set(&mut self, index: usize, value: i32) {
        match self {
            Self::Uniform { value: val } => {
                if *val == value {
                    return;
                }

                let mut values = PaletteMap::default();
                let mut indices = BitArray::<4096>::new();

                values.insert(*val, 4096);
                let (i, _) = values.insert_full(value, 1);

                indices.set(index, i as u16);

                *self = Self::Indexed { values, indices };
            }
            Self::Indexed { values, indices } => {
                let i = match values.entry(value) {
                    Entry::Occupied(mut occupied) => {
                        *occupied.get_mut() += 1;
                        occupied.index()
                    }
                    Entry::Vacant(vacant) => vacant.insert_entry(1).index(),
                };

                let old = indices.set(index, i as u16) as usize;
                if old == i {
                    return;
                }

                let Some(mut entry) = values.get_index_entry(old) else {
                    return;
                };

                let count = entry.get_mut();
                *count = count.saturating_sub(1);
                if *count > 0 {
                    return;
                }

                // This guard prevents removal of a dead palette entry unless every entry from
                // `old` to the end of the palette is also dead. Removing a middle entry normally
                // requires remapping all bitarray indices, because `swap_remove_index` moves the
                // last palette entry into the removed slot. However, if the entire tail is dead,
                // none of those palette indices appear in the bitarray, so removing them does
                // not change the meaning of any stored block. This allows safe, remap‑free
                // cleanup of dead palette entries at the end, while leaving dead middle entries
                // in place until they naturally drift to the tail.
                if !{
                    let mut dead = true;
                    for i in old..values.len() {
                        let (_, count) = values.get_index(i).expect("index is in bounds");
                        if *count > 0 {
                            dead = false;
                            break;
                        }
                    }
                    dead
                } {
                    return;
                }

                // remove all the dead tail entries
                while old < values.len() {
                    values.swap_remove_index(old);
                }

                match values.len() {
                    1 => {
                        let (&value, _) = values.first().expect("len is 1");

                        *self = Self::Uniform { value };
                    }
                    len if old == len => {
                        let max = len - 1;
                        let bits = BitArray::<4096>::bits_for(max as u16);
                        if bits < indices.get_bits() {
                            indices.resize(bits);
                        }
                    }
                    _ => {
                        // maybe remap bitarray indices & attempt bitarray resize?
                        // not necessarily required, currently values that aren't the last index will just become "dead"
                        // and eventually be removed once/if all the other indices after it are removed, essentially acting as a lazy-removal
                    }
                }
            }
        }
    }
}

impl Palette {
    pub fn write_blocks_disk(&self, buf: &mut Vec<u8>, mut entry: impl FnMut(i32, &mut Vec<u8>)) {
        match self {
            Self::Uniform { value } => {
                buf.push(0);
                buf.extend(1u32.to_le_bytes());
                entry(*value, buf);
            }
            Self::Indexed { values, indices } => {
                write_indices(buf, indices);
                buf.extend((values.len() as u32).to_le_bytes());
                for &value in values.keys() {
                    entry(value, buf);
                }
            }
        }
    }

    pub fn write_biomes_disk(&self, buf: &mut Vec<u8>) {
        match self {
            Self::Uniform { value } => {
                buf.push(0);
                buf.extend(value.to_le_bytes());
            }
            Self::Indexed { values, indices } => {
                write_indices(buf, indices);
                buf.extend((values.len() as u32).to_le_bytes());
                for value in values.keys() {
                    buf.extend(value.to_le_bytes());
                }
            }
        }
    }
}

fn write_indices(buf: &mut Vec<u8>, indices: &BitArray<4096>) {
    buf.push(indices.get_bits() << 1);
    for word in indices.get_blocks() {
        buf.extend(word.to_le_bytes());
    }
}

impl ProtoCodec for Palette {
    fn serialize<W: Write>(&self, stream: &mut W) -> Result<(), ProtoCodecError> {
        match &self {
            Self::Uniform { value } => {
                u8::serialize(&0x01, stream)?;
                <i32 as ProtoCodecVAR>::serialize(value, stream)?;
            }
            Self::Indexed { values, indices } => {
                u8::serialize(&((indices.get_bits() << 1) | 1), stream)?;
                for block in indices.get_blocks() {
                    <u32 as ProtoCodecLE>::serialize(block, stream)?;
                }

                <i32 as ProtoCodecVAR>::serialize(&(values.len() as i32), stream)?;
                for (id, _) in values {
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
