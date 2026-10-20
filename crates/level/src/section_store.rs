use crate::bit_array::BitArray;
use crate::palette::Palette;
use std::cell::RefCell;
use std::sync::Arc;

#[derive(Clone, Copy)]
struct Meta {
    bits: u8,
    values_at: u32,
    values_len: u16,
    words_at: u32,
}

struct Data {
    meta: Box<[Meta]>,
    values: Box<[i32]>,
    words: Box<[u32]>,
}

#[derive(Clone)]
pub struct SectionStore(Arc<Data>);

type Spare = (Vec<Meta>, Vec<i32>, Vec<u32>);

thread_local! {
    static SPARE: RefCell<Spare> = const { RefCell::new((Vec::new(), Vec::new(), Vec::new())) };
}

pub struct SectionStoreBuilder {
    meta: Vec<Meta>,
    values: Vec<i32>,
    words: Vec<u32>,
}

impl Default for SectionStoreBuilder {
    fn default() -> Self {
        let (mut meta, mut values, mut words) = SPARE.try_with(|spare| std::mem::take(&mut *spare.borrow_mut())).unwrap_or_default();
        meta.clear();
        values.clear();
        words.clear();
        Self { meta, values, words }
    }
}

impl SectionStoreBuilder {
    pub fn push_palette(&mut self, palette: &Palette) {
        let values_at = self.values.len() as u32;
        palette.extend_values(&mut self.values);
        self.meta.push(Meta {
            bits: palette.bits(),
            values_at,
            values_len: (self.values.len() as u32 - values_at) as u16,
            words_at: self.words.len() as u32,
        });
        self.words.extend_from_slice(palette.words());
    }

    pub fn push_section(&mut self, store: &SectionStore, section: usize) {
        let meta = store.0.meta[section];
        let values = &store.0.values[meta.values_at as usize..meta.values_at as usize + meta.values_len as usize];
        let words = store.words_of(section);
        self.meta.push(Meta {
            bits: meta.bits,
            values_at: self.values.len() as u32,
            values_len: meta.values_len,
            words_at: self.words.len() as u32,
        });
        self.values.extend_from_slice(values);
        self.words.extend_from_slice(words);
    }

    pub fn push_blocks(&mut self, blocks: &[i32; 4096]) {
        let mut values = [0i32; 4096];
        let mut indices = [0u16; 4096];
        values[0] = blocks[0];
        let mut count = 1usize;
        let mut last = (blocks[0], 0usize);
        for (slot, &value) in indices.iter_mut().zip(blocks) {
            let index = if value == last.0 {
                last.1
            } else {
                let index = match values[..count].iter().position(|&known| known == value) {
                    Some(index) => index,
                    None => {
                        values[count] = value;
                        count += 1;
                        count - 1
                    }
                };
                last = (value, index);
                index
            };
            *slot = index as u16;
        }

        let values_at = self.values.len() as u32;
        self.values.extend_from_slice(&values[..count]);
        let words_at = self.words.len() as u32;
        let bits = if count == 1 { 0 } else { BitArray::<4096>::bits_for((count - 1) as u16) };
        if bits != 0 {
            let per_word = (32 / bits as u32) as usize;
            for group in indices.chunks(per_word) {
                self.words
                    .push(group.iter().enumerate().fold(0u32, |word, (slot, &index)| word | (index as u32) << (slot as u32 * bits as u32)));
            }
        }
        self.meta.push(Meta {
            bits,
            values_at,
            values_len: count as u16,
            words_at,
        });
    }

    pub fn build(self) -> SectionStore {
        let data = Data {
            meta: self.meta.as_slice().into(),
            values: self.values.as_slice().into(),
            words: self.words.as_slice().into(),
        };
        let _ = SPARE.try_with(|spare| *spare.borrow_mut() = (self.meta, self.values, self.words));
        SectionStore(Arc::new(data))
    }
}

impl SectionStore {
    pub fn uniform(value: i32, sections: usize) -> Self {
        let mut builder = SectionStoreBuilder::default();
        let palette = Palette::new(value);
        for _ in 0..sections {
            builder.push_palette(&palette);
        }
        builder.build()
    }

    pub fn len(&self) -> usize {
        self.0.meta.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.meta.is_empty()
    }

    fn words_of(&self, section: usize) -> &[u32] {
        let meta = self.0.meta[section];
        if meta.bits == 0 {
            return &[];
        }
        let per_word = 32 / meta.bits as usize;
        let start = meta.words_at as usize;
        &self.0.words[start..start + 4096usize.div_ceil(per_word)]
    }

    pub fn get(&self, section: usize, local: usize) -> i32 {
        let meta = self.0.meta[section];
        let values = &self.0.values[meta.values_at as usize..];
        if meta.bits == 0 {
            return values[0];
        }
        let bits = meta.bits as usize;
        let per_word = 32 / bits;
        let word = self.0.words[meta.words_at as usize + local / per_word];
        let index = (word >> ((local % per_word) * bits)) & ((1u32 << bits) - 1);
        values[index as usize]
    }

    pub fn to_array(&self, section: usize) -> [i32; 4096] {
        let meta = self.0.meta[section];
        let values = &self.0.values[meta.values_at as usize..meta.values_at as usize + meta.values_len as usize];
        let mut out = [0i32; 4096];
        if meta.bits == 0 {
            out.fill(values[0]);
            return out;
        }
        let bits = meta.bits as u32;
        let per_word = (32 / bits) as usize;
        let mask = (1u32 << bits) - 1;
        for (chunk, &word) in out.chunks_mut(per_word).zip(self.words_of(section)) {
            for (slot, value) in chunk.iter_mut().enumerate() {
                *value = values[((word >> (slot as u32 * bits)) & mask) as usize];
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Lcg;

    fn random_palette(seed: u64, distinct: u32) -> (Palette, [i32; 4096]) {
        let mut rng = Lcg::new(seed);
        let mut blocks = [0i32; 4096];
        for (i, value) in blocks.iter_mut().enumerate() {
            *value = if (i as u32) < distinct { i as i32 } else { rng.below(distinct) as i32 };
            *value = *value * 37 - 100;
        }
        (Palette::from_blocks(&blocks), blocks)
    }

    fn store_of(palettes: &[Palette]) -> SectionStore {
        let mut builder = SectionStoreBuilder::default();
        for palette in palettes {
            builder.push_palette(palette);
        }
        builder.build()
    }

    #[test]
    fn every_section_reads_back_what_its_palette_held() {
        let cases = [1u32, 2, 3, 5, 9, 17, 33, 65, 257, 1000];
        let built: Vec<(Palette, [i32; 4096])> = cases.iter().map(|&distinct| random_palette(distinct as u64 + 7, distinct)).collect();
        let palettes: Vec<Palette> = built.iter().map(|(palette, _)| palette.clone()).collect();
        let store = store_of(&palettes);
        assert_eq!(store.len(), cases.len());
        for (section, (_, blocks)) in built.iter().enumerate() {
            assert_eq!(&store.to_array(section), blocks, "section {section}");
            for local in (0..4096).step_by(97) {
                assert_eq!(store.get(section, local), blocks[local], "section {section} local {local}");
            }
        }
    }

    #[test]
    fn packing_blocks_directly_matches_packing_them_through_a_palette() {
        for (seed, distinct) in [(1u64, 1u32), (2, 2), (3, 3), (4, 5), (5, 17), (6, 65), (7, 300), (8, 1000), (9, 4096)] {
            let (palette, blocks) = random_palette(seed, distinct);
            let mut direct = SectionStoreBuilder::default();
            direct.push_blocks(&blocks);
            let direct = direct.build();
            let through_palette = store_of(&[palette]);
            assert_eq!(direct.to_array(0), through_palette.to_array(0), "{distinct} distinct");
            assert_eq!(direct.to_array(0), blocks, "{distinct} distinct");
            for local in (0..4096).step_by(131) {
                assert_eq!(direct.get(0, local), blocks[local]);
            }
        }
    }

    #[test]
    fn uniform_sections_hold_one_value_and_no_words() {
        let store = SectionStore::uniform(42, 24);
        assert_eq!(store.len(), 24);
        assert_eq!(store.get(23, 4095), 42);
        assert!(store.to_array(5).iter().all(|&value| value == 42));
        assert!(store.words_of(0).is_empty());
    }

    #[test]
    fn copying_a_section_into_a_new_store_keeps_it_intact() {
        let (first, first_blocks) = random_palette(1, 9);
        let (second, second_blocks) = random_palette(2, 300);
        let old = store_of(&[first, second]);
        let mut builder = SectionStoreBuilder::default();
        builder.push_section(&old, 1);
        builder.push_palette(&Palette::new(7));
        builder.push_section(&old, 0);
        let rebuilt = builder.build();
        assert_eq!(rebuilt.to_array(0), second_blocks);
        assert_eq!(rebuilt.get(1, 100), 7);
        assert_eq!(rebuilt.to_array(2), first_blocks);
    }

    #[test]
    fn cloning_shares_the_storage() {
        let store = SectionStore::uniform(1, 4);
        let copy = store.clone();
        assert!(Arc::ptr_eq(&store.0, &copy.0));
    }
}
