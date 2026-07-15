use crate::level::bit_array::BitArray;
use bedrock::protocol::error::ProtoCodecError;
use bedrock::protocol::{ProtoCodec, ProtoCodecLE, ProtoCodecVAR};
use indexmap::IndexMap;
use indexmap::map::Entry;
use std::io::{Read, Write};

pub enum Palette {
    Uniform { value: i32 },
    Indexed { values: IndexMap<i32, u16>, indices: BitArray<4096> },
}

impl Palette {
    pub fn new(value: i32) -> Self {
        Self::Uniform { value }
    }

    pub fn get(&self, index: usize) -> i32 {
        match &self {
            Self::Uniform { value } => *value,
            Self::Indexed { values, indices } => *values.get_index(indices.get(index) as usize).unwrap().0,
        }
    }

    pub fn set(&mut self, index: usize, value: i32) {
        match self {
            Self::Uniform { value: val } => {
                if *val == value {
                    return;
                }

                let mut values = IndexMap::new();
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
