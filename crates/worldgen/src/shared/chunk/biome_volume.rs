use crate::shared::biome::Biome;
use chorus_level::palette::Palette;
use chorus_level::section_store::{SectionStore, SectionStoreBuilder};

pub const SAMPLE_COLUMNS: usize = 4;
pub const SAMPLE_SPACING: i32 = 8;
const SAMPLE_PADDING_LEVELS: i32 = 2;
const SECTION_SIZE: i32 = 16;
const SECTION_BLOCKS: usize = 4096;
const CELLS_PER_SECTION_AXIS: i32 = SECTION_SIZE / SAMPLE_SPACING;
const CELL_SCALE: f32 = 0.125;
const CELL_MASK: i32 = SAMPLE_SPACING - 1;
const SAMPLE_ORIGIN_SHIFT: i32 = SAMPLE_SPACING;
const CELL_AXIS_SHIFT: u32 = 3;
const HASH_X: u32 = 3129871;
const HASH_Z: i64 = 116129781;
const HASH_MULTIPLIER: i64 = 42317861;
const HASH_INCREMENT: i64 = 11;
const HASH_SHIFT: u32 = 16;
const RANDOM_MULTIPLIER: u64 = 0x5DEECE66D;
const RANDOM_INCREMENT: u64 = 0xB;
const RANDOM_SHIFT: u32 = 24;
const RANDOM_MASK: u64 = 0xFFFFFF;
const JITTER_UNIT: f32 = 5.9604645e-08;
const JITTER_CENTRE: f32 = 0.5;
const JITTER_SCALE: f32 = 0.9;
const CANDIDATE_STEPS: [(usize, usize, usize); 8] = [(0, 0, 0), (0, 0, 1), (0, 1, 0), (0, 1, 1), (1, 0, 0), (1, 0, 1), (1, 1, 0), (1, 1, 1)];

pub fn sample_levels(height: i32) -> usize {
    (height / SAMPLE_SPACING + SAMPLE_PADDING_LEVELS) as usize
}

pub fn sample_count(height: i32) -> usize {
    SAMPLE_COLUMNS * SAMPLE_COLUMNS * sample_levels(height)
}

pub fn sample_index(xi: usize, zi: usize, yi: usize, levels: usize) -> usize {
    (xi * SAMPLE_COLUMNS + zi) * levels + yi
}

pub fn sample_origin(chunk_x: i32, chunk_z: i32, min_y: i32) -> (i32, i32, i32) {
    ((chunk_x << 4) - SAMPLE_ORIGIN_SHIFT, min_y - SAMPLE_ORIGIN_SHIFT, (chunk_z << 4) - SAMPLE_ORIGIN_SHIFT)
}

pub fn jitter(level_seed: i32, x: i32, y: i32, z: i32) -> [f32; 3] {
    let (x8, y8, z8) = (x >> CELL_AXIS_SHIFT, y >> CELL_AXIS_SHIFT, z >> CELL_AXIS_SHIFT);
    let mixed = ((x8 as u32).wrapping_mul(HASH_X) ^ y8 as u32) as i32 as i64 ^ (z8 as i64).wrapping_mul(HASH_Z);
    let hashed = (mixed.wrapping_mul(HASH_MULTIPLIER).wrapping_add(HASH_INCREMENT).wrapping_mul(mixed) as u64) >> HASH_SHIFT;
    let mut state = hashed ^ level_seed as u32 as u64 ^ RANDOM_MULTIPLIER;
    std::array::from_fn(|_| {
        state = state.wrapping_mul(RANDOM_MULTIPLIER).wrapping_add(RANDOM_INCREMENT);
        (((state >> RANDOM_SHIFT) & RANDOM_MASK) as f32 * JITTER_UNIT - JITTER_CENTRE) * JITTER_SCALE
    })
}

#[derive(Clone, Copy)]
enum Cell {
    Uniform(i32),
    Mixed { corners: [i32; 8], jitters: [[f32; 3]; 8] },
}

#[derive(Clone)]
pub struct BiomeVolume {
    min_y: i32,
    sections: SectionStore,
}

impl BiomeVolume {
    pub fn resolve(samples: &[i32], level_seed: i32, chunk_x: i32, chunk_z: i32, min_y: i32, height: i32) -> Self {
        let levels = sample_levels(height);
        assert_eq!(samples.len(), sample_count(height));
        assert_eq!(min_y & CELL_MASK, 0);
        let origin = sample_origin(chunk_x, chunk_z, min_y);
        let cell_of = |xc: usize, zc: usize, yc: usize| {
            let corners = CANDIDATE_STEPS.map(|(sx, sy, sz)| samples[sample_index(xc + sx, zc + sz, yc + sy, levels)]);
            if corners.iter().all(|&corner| corner == corners[0]) {
                return Cell::Uniform(corners[0]);
            }
            let jitters = CANDIDATE_STEPS.map(|(sx, sy, sz)| {
                jitter(
                    level_seed,
                    origin.0 + (xc + sx) as i32 * SAMPLE_SPACING,
                    origin.1 + (yc + sy) as i32 * SAMPLE_SPACING,
                    origin.2 + (zc + sz) as i32 * SAMPLE_SPACING,
                )
            });
            Cell::Mixed { corners, jitters }
        };
        let mut builder = SectionStoreBuilder::default();
        let mut blocks = Box::new([0i32; SECTION_BLOCKS]);
        for section in 0..(height / SECTION_SIZE) as usize {
            let cells: [Cell; 8] = std::array::from_fn(|i| {
                let (hx, hy, hz) = ((i >> 2) & 1, i & 1, (i >> 1) & 1);
                cell_of(hx + 1, hz + 1, section * CELLS_PER_SECTION_AXIS as usize + hy + 1)
            });
            if let Cell::Uniform(id) = cells[0]
                && cells.iter().all(|cell| matches!(cell, Cell::Uniform(other) if *other == id))
            {
                builder.push_palette(&Palette::new(id));
                continue;
            }
            for (i, cell) in cells.iter().enumerate() {
                let (hx, hy, hz) = ((i >> 2) & 1, i & 1, (i >> 1) & 1);
                match cell {
                    Cell::Uniform(id) => fill_cell(&mut blocks, hx, hy, hz, |_| *id),
                    Cell::Mixed { corners, jitters } => fill_cell(&mut blocks, hx, hy, hz, |(fx, fy, fz)| nearest(corners, jitters, fx, fy, fz)),
                }
            }
            builder.push_blocks(&blocks);
        }
        Self { min_y, sections: builder.build() }
    }

    fn locate(&self, x: i32, y: i32, z: i32) -> (usize, usize) {
        let ry = (y - self.min_y).clamp(0, self.sections.len() as i32 * SECTION_SIZE - 1);
        ((ry >> 4) as usize, (((ry & 15) << 8) | (z.clamp(0, 15) << 4) | x.clamp(0, 15)) as usize)
    }

    pub fn id_at(&self, x: i32, y: i32, z: i32) -> i32 {
        let (section, local) = self.locate(x, y, z);
        self.sections.get(section, local)
    }

    pub fn get(&self, x: i32, y: i32, z: i32) -> Biome {
        Biome::generated(self.id_at(x, y, z))
    }

    pub fn section_ids(&self, section: usize) -> [i32; SECTION_BLOCKS] {
        self.sections.to_array(section)
    }

    pub fn uniform_id_at(&self, y: i32) -> Option<i32> {
        let ry = (y - self.min_y).clamp(0, self.sections.len() as i32 * SECTION_SIZE - 1);
        self.sections.uniform_value((ry >> 4) as usize)
    }

    pub fn uniform_biome_at(&self, y: i32) -> Option<Biome> {
        self.uniform_id_at(y).map(Biome::generated)
    }
}

fn fill_cell(blocks: &mut [i32; SECTION_BLOCKS], hx: usize, hy: usize, hz: usize, pick: impl Fn((f32, f32, f32)) -> i32) {
    let span = SAMPLE_SPACING as usize;
    for ly in 0..span {
        for lz in 0..span {
            for lx in 0..span {
                let fraction = (lx as f32 * CELL_SCALE, ly as f32 * CELL_SCALE, lz as f32 * CELL_SCALE);
                blocks[((hy * span + ly) << 8) | ((hz * span + lz) << 4) | (hx * span + lx)] = pick(fraction);
            }
        }
    }
}

fn nearest(corners: &[i32; 8], jitters: &[[f32; 3]; 8], fx: f32, fy: f32, fz: f32) -> i32 {
    let mut best = 0;
    let mut best_distance = f32::INFINITY;
    for (corner, &(sx, sy, sz)) in CANDIDATE_STEPS.iter().enumerate() {
        let [jx, jy, jz] = jitters[corner];
        let ax = (fx - sx as f32) + jx;
        let ay = jy + (fy - sy as f32);
        let az = jz + (fz - sz as f32);
        let distance = az * az + ay * ay + ax * ax;
        if distance < best_distance {
            best = corner;
            best_distance = distance;
        }
    }
    corners[best]
}

#[cfg(test)]
mod tests {
    use super::*;

    const GENERAL_1: i64 = 4802433367836389877;

    const HEIGHT: i32 = 384;
    const MIN_Y: i32 = -64;
    const SEED: i32 = GENERAL_1 as i32;
    const SECTIONS: usize = (HEIGHT / SECTION_SIZE) as usize;

    fn uniform_samples(biome: Biome) -> Vec<i32> {
        vec![biome.bedrock_id(); sample_count(HEIGHT)]
    }

    fn count_of(volume: &BiomeVolume, section: usize, biome: Biome) -> usize {
        let id = biome.bedrock_id();
        volume.section_ids(section).iter().filter(|&&candidate| candidate == id).count()
    }

    #[test]
    fn mixed_cells_match_the_uniform_shortcut_when_every_candidate_agrees() {
        let levels = sample_levels(HEIGHT);
        let mut samples = uniform_samples(Biome::PLAINS);
        samples[sample_index(1, 1, 20, levels)] = Biome::FOREST.bedrock_id();
        let first = BiomeVolume::resolve(&samples, SEED, 0, 0, MIN_Y, HEIGHT);
        let second = BiomeVolume::resolve(&samples, SEED, 0, 0, MIN_Y, HEIGHT);
        let mut forests = 0;
        let mut uniform = 0;
        for section in 0..SECTIONS {
            assert_eq!(first.section_ids(section), second.section_ids(section));
            match first.sections.uniform_value(section) {
                Some(id) => {
                    assert_eq!(id, Biome::PLAINS.bedrock_id());
                    uniform += 1;
                }
                None => forests += count_of(&first, section, Biome::FOREST),
            }
        }
        assert!(forests > 0);
        assert!(uniform > 20);
    }

    #[test]
    fn a_mixed_section_holds_only_the_biomes_that_were_sampled() {
        let levels = sample_levels(HEIGHT);
        let mut samples = uniform_samples(Biome::PLAINS);
        samples[sample_index(1, 1, 20, levels)] = Biome::FOREST.bedrock_id();
        samples[sample_index(2, 2, 3, levels)] = Biome::DESERT.bedrock_id();
        let volume = BiomeVolume::resolve(&samples, SEED, 0, 0, MIN_Y, HEIGHT);
        for section in (0..SECTIONS).filter(|&section| volume.sections.uniform_value(section).is_none()) {
            let total: usize = [Biome::PLAINS, Biome::FOREST, Biome::DESERT].into_iter().map(|biome| count_of(&volume, section, biome)).sum();
            assert_eq!(total, SECTION_BLOCKS);
        }
    }
}
