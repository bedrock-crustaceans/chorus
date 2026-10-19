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

    pub fn is_all(&self, value: i32) -> bool {
        self.count(value) == 4096
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
        let indices = BitArray::<4096>::from_indices(bits, &block_indices);

        Self::Indexed { values, indices }
    }

    pub fn get(&self, index: usize) -> i32 {
        match &self {
            Self::Uniform { value } => *value,
            Self::Indexed { values, indices } => *values.get_index(indices.get(index) as usize).unwrap().0,
        }
    }

    pub fn to_array(&self) -> [i32; 4096] {
        match self {
            Self::Uniform { value } => [*value; 4096],
            Self::Indexed { values, indices } => {
                let table: Vec<i32> = values.keys().copied().collect();
                let mut blocks = [0i32; 4096];
                for (block, index) in blocks.iter_mut().zip(indices.indices()) {
                    *block = table[index as usize];
                }
                blocks
            }
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

                values.insert(*val, 4095);
                let (i, _) = values.insert_full(value, 1);

                indices.set(index, i as u16);

                *self = Self::Indexed { values, indices };
            }
            Self::Indexed { values, indices } => {
                let stored = indices.get(index) as usize;
                if values.get_index(stored).is_some_and(|(&current, _)| current == value) {
                    return;
                }

                let i = match values.entry(value) {
                    Entry::Occupied(mut occupied) => {
                        *occupied.get_mut() += 1;
                        occupied.index()
                    }
                    Entry::Vacant(vacant) => vacant.insert_entry(1).index(),
                };

                let old = indices.set(index, i as u16) as usize;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{Lcg, decode_disk, decode_disk_biomes, decode_network};

    fn bits_of(palette: &Palette) -> Option<u8> {
        match palette {
            Palette::Uniform { .. } => None,
            Palette::Indexed { indices, .. } => Some(indices.get_bits()),
        }
    }

    fn len_of(palette: &Palette) -> usize {
        match palette {
            Palette::Uniform { .. } => 1,
            Palette::Indexed { values, .. } => values.len(),
        }
    }

    fn assert_reads(palette: &Palette, reference: &[i32; 4096]) {
        for (i, &value) in reference.iter().enumerate() {
            assert_eq!(palette.get(i), value, "index {i}");
        }
    }

    fn assert_matches(palette: &Palette, reference: &[i32; 4096]) {
        assert_reads(palette, reference);
        let mut expected: std::collections::HashMap<i32, u32> = std::collections::HashMap::new();
        for &value in reference {
            *expected.entry(value).or_default() += 1;
        }
        for (&value, &count) in &expected {
            assert_eq!(palette.count(value), count, "count of {value}");
        }
        assert_eq!(palette.count(i32::MIN), 0);
    }

    fn random_palette(seed: u64, distinct: u32) -> (Palette, [i32; 4096]) {
        let mut rng = Lcg::new(seed);
        let mut reference = [0i32; 4096];
        for (i, value) in reference.iter_mut().enumerate() {
            *value = if (i as u32) < distinct { i as i32 } else { rng.below(distinct) as i32 };
            *value = *value * 37 - 100;
        }
        (Palette::from_blocks(&reference), reference)
    }

    #[test]
    fn uniform_reports_its_value_everywhere() {
        let palette = Palette::new(9);
        assert_eq!(palette.get(0), 9);
        assert_eq!(palette.get(4095), 9);
        assert_eq!(palette.count(9), 4096);
        assert_eq!(palette.count(8), 0);
        assert!(palette.is_all(9));
        assert!(!palette.is_all(8));
    }

    #[test]
    fn setting_the_same_value_keeps_the_palette_uniform() {
        let mut palette = Palette::new(9);
        palette.set(5, 9);
        assert!(matches!(palette, Palette::Uniform { value: 9 }));
    }

    #[test]
    fn a_different_value_turns_a_uniform_palette_into_a_one_bit_indexed_one() {
        let mut palette = Palette::new(9);
        palette.set(77, 4);

        assert_eq!(bits_of(&palette), Some(1));
        assert_eq!(palette.get(77), 4);
        assert_eq!(palette.get(76), 9);
        assert_eq!(palette.count(4), 1);
    }

    #[test]
    fn the_old_value_loses_one_count_when_a_uniform_palette_becomes_indexed() {
        let mut palette = Palette::new(9);
        palette.set(77, 4);

        assert_eq!(palette.count(9), 4095);
        assert!(!palette.is_all(9));
    }

    #[test]
    fn restoring_the_only_changed_entry_collapses_back_to_uniform() {
        let mut palette = Palette::new(9);
        palette.set(77, 4);
        palette.set(77, 9);

        assert!(matches!(palette, Palette::Uniform { value: 9 }));
        assert!(palette.is_all(9));
    }

    #[test]
    fn overwriting_an_entry_moves_the_counts() {
        let mut blocks = [1; 4096];
        blocks[0] = 2;
        blocks[1] = 2;
        blocks[2] = 3;
        let mut palette = Palette::from_blocks(&blocks);
        assert_eq!((palette.count(1), palette.count(2), palette.count(3)), (4093, 2, 1));

        palette.set(0, 3);
        assert_eq!((palette.count(1), palette.count(2), palette.count(3)), (4093, 1, 2));
    }

    #[test]
    fn rewriting_the_stored_value_leaves_the_counts_alone() {
        let mut blocks = [1; 4096];
        blocks[0] = 3;
        let mut palette = Palette::from_blocks(&blocks);
        palette.set(0, 3);
        assert_eq!((palette.count(1), palette.count(3)), (4095, 1));
    }

    #[test]
    fn the_bit_width_grows_at_every_boundary() {
        let boundaries = [
            (2usize, 1u8),
            (3, 2),
            (4, 2),
            (5, 3),
            (8, 3),
            (9, 4),
            (16, 4),
            (17, 5),
            (32, 5),
            (33, 6),
            (64, 6),
            (65, 8),
            (256, 8),
            (257, 16),
        ];
        let mut palette = Palette::new(0);
        let mut reference = [0i32; 4096];
        let mut next = 1;
        for (len, bits) in boundaries {
            while len_of(&palette) < len {
                palette.set(next, next as i32);
                reference[next] = next as i32;
                next += 1;
            }
            assert_eq!(len_of(&palette), len);
            assert_eq!(bits_of(&palette), Some(bits), "palette of {len} entries");
            assert_reads(&palette, &reference);
        }
    }

    #[test]
    fn from_blocks_picks_the_width_for_the_palette_size() {
        for (distinct, bits) in [
            (2u32, 1u8),
            (3, 2),
            (4, 2),
            (5, 3),
            (8, 3),
            (9, 4),
            (16, 4),
            (17, 5),
            (32, 5),
            (33, 6),
            (64, 6),
            (65, 8),
            (256, 8),
            (257, 16),
            (1000, 16),
        ] {
            let (palette, reference) = random_palette(distinct as u64, distinct);
            assert_eq!(len_of(&palette), distinct as usize, "distinct {distinct}");
            assert_eq!(bits_of(&palette), Some(bits), "distinct {distinct}");
            assert_matches(&palette, &reference);
        }
    }

    #[test]
    fn from_blocks_of_one_value_is_uniform() {
        let palette = Palette::from_blocks(&[42; 4096]);
        assert!(matches!(palette, Palette::Uniform { value: 42 }));
    }

    #[test]
    fn random_edits_match_a_plain_array_at_every_step() {
        for (seed, distinct) in [(1u64, 2u32), (2, 6), (3, 20), (4, 70), (5, 300)] {
            let mut rng = Lcg::new(seed);
            let mut reference = [5i32; 4096];
            let mut palette = Palette::new(5);
            for step in 0..6000 {
                let index = rng.below(4096) as usize;
                let value = rng.below(distinct) as i32;
                reference[index] = value;
                palette.set(index, value);
                if step % 1500 == 0 {
                    assert_reads(&palette, &reference);
                }
            }
            assert_reads(&palette, &reference);
        }
    }

    #[test]
    fn random_edits_keep_the_counts_exact_once_the_palette_is_indexed() {
        for (seed, distinct) in [(1u64, 2u32), (2, 6), (3, 20), (4, 70), (5, 300)] {
            let (mut palette, mut reference) = random_palette(seed + 40, distinct.max(3));
            let mut rng = Lcg::new(seed);
            for _ in 0..6000 {
                let index = rng.below(4096) as usize;
                let value = rng.below(distinct) as i32 * 37 - 100;
                if palette.get(index) != value {
                    reference[index] = value;
                    palette.set(index, value);
                }
            }
            assert_matches(&palette, &reference);
        }
    }

    #[test]
    fn random_edits_from_a_uniform_palette_keep_the_counts_exact() {
        let mut rng = Lcg::new(1);
        let mut reference = [5i32; 4096];
        let mut palette = Palette::new(5);
        for _ in 0..6000 {
            let index = rng.below(4096) as usize;
            let value = rng.below(6) as i32;
            if palette.get(index) != value {
                reference[index] = value;
                palette.set(index, value);
            }
        }
        assert_matches(&palette, &reference);
    }

    #[test]
    fn counts_always_sum_to_the_block_total() {
        let mut rng = Lcg::new(11);
        let (mut palette, _) = random_palette(12, 12);
        for _ in 0..5000 {
            let index = rng.below(4096) as usize;
            let value = rng.below(12) as i32 * 37 - 100;
            if palette.get(index) != value {
                palette.set(index, value);
            }
        }
        let Palette::Indexed { values, .. } = &palette else { panic!("expected an indexed palette") };
        assert_eq!(values.values().map(|&count| count as u32).sum::<u32>(), 4096);
    }

    #[test]
    fn overwriting_everything_with_one_value_ends_uniform() {
        let (mut palette, _) = random_palette(21, 9);
        for index in 0..4096 {
            palette.set(index, 3);
        }
        assert!(palette.is_all(3));
        assert_eq!(palette.count(3), 4096);
        assert_eq!(palette.get(2048), 3);
    }

    #[test]
    fn disk_blocks_round_trip_for_uniform_and_indexed_palettes() {
        for distinct in [1u32, 2, 3, 9, 40, 300] {
            let (palette, reference) = random_palette(distinct as u64 + 100, distinct);
            let mut buf = Vec::new();
            palette.write_blocks_disk(&mut buf, |value, buf| buf.extend(value.to_le_bytes()));
            let decoded = decode_disk(&mut buf.as_slice());
            assert_eq!(decoded.as_slice(), reference.as_slice(), "distinct {distinct}");
        }
    }

    #[test]
    fn disk_biomes_round_trip() {
        for distinct in [1u32, 2, 7, 33] {
            let (palette, reference) = random_palette(distinct as u64 + 200, distinct);
            let mut buf = Vec::new();
            palette.write_biomes_disk(&mut buf);
            let mut reader = buf.as_slice();
            let decoded = decode_disk_biomes(&mut reader);
            assert!(reader.is_empty());
            assert_eq!(decoded.as_slice(), reference.as_slice(), "distinct {distinct}");
        }
    }

    #[test]
    fn disk_header_stores_the_width_shifted_left_with_the_disk_flag_clear() {
        let mut buf = Vec::new();
        Palette::new(1).write_biomes_disk(&mut buf);
        assert_eq!(buf, [0, 1, 0, 0, 0]);

        let (palette, _) = random_palette(1, 5);
        let mut buf = Vec::new();
        palette.write_biomes_disk(&mut buf);
        assert_eq!(buf[0], 3 << 1);
    }

    #[test]
    fn network_palettes_round_trip_for_uniform_and_indexed_palettes() {
        for distinct in [1u32, 2, 3, 9, 40, 300] {
            let (palette, reference) = random_palette(distinct as u64 + 300, distinct);
            let mut buf = Vec::new();
            palette.serialize(&mut buf).unwrap();
            let decoded = decode_network(&mut buf.as_slice());
            assert_eq!(decoded.as_slice(), reference.as_slice(), "distinct {distinct}");
        }
    }

    #[test]
    fn a_uniform_network_palette_is_a_flag_byte_and_a_zigzag_varint() {
        let mut buf = Vec::new();
        Palette::new(-1).serialize(&mut buf).unwrap();
        assert_eq!(buf, [0x01, 0x01]);

        let mut buf = Vec::new();
        Palette::new(2).serialize(&mut buf).unwrap();
        assert_eq!(buf, [0x01, 0x04]);
    }
}
