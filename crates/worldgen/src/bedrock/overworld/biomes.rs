use super::biome_tree::{BiomeTree, BiomeTreeError};
use super::terrain::BedrockTerrain;
use crate::shared::biome::Biome;
use crate::shared::chunk::biome_volume::{BiomeVolume, SAMPLE_COLUMNS, SAMPLE_SPACING, sample_count, sample_index, sample_levels};
use crate::shared::terrain::BiomeSource;
use std::sync::Arc;

const OWN_COLUMNS: usize = 2;
const SAMPLE_OWNERS: [(i32, usize); SAMPLE_COLUMNS] = [(-1, 1), (0, 0), (0, 1), (1, 0)];

pub struct BiomeSampler {
    tree: BiomeTree,
    level_seed: i32,
    source: BiomeSource,
}

impl BiomeSampler {
    pub fn new(level_seed: i32, source: BiomeSource) -> Result<Self, BiomeTreeError> {
        Ok(Self {
            tree: BiomeTree::parse()?,
            level_seed,
            source,
        })
    }

    fn fill_column(&self, terrain: &BedrockTerrain, x: i32, z: i32, origin_y: i32, out: &mut [i32]) {
        match &self.source {
            BiomeSource::MultiNoise => {
                let column = terrain.climate_column(x, z);
                let mut previous = None;
                for (yi, slot) in out.iter_mut().enumerate() {
                    let nearest = self.tree.find(&column.point(origin_y + yi as i32 * SAMPLE_SPACING), previous);
                    *slot = self.tree.biome_id(nearest);
                    previous = Some(nearest);
                }
            }
            flat => out.fill(Self::flat_biome(flat, x, z).bedrock_id()),
        }
    }

    pub fn own_columns(&self, terrain: &BedrockTerrain, chunk_x: i32, chunk_z: i32, min_y: i32, height: i32) -> Vec<i32> {
        let levels = sample_levels(height);
        let origin_y = min_y - SAMPLE_SPACING;
        let mut columns = vec![0; OWN_COLUMNS * OWN_COLUMNS * levels];
        for a in 0..OWN_COLUMNS {
            for b in 0..OWN_COLUMNS {
                let (x, z) = ((chunk_x << 4) + a as i32 * SAMPLE_SPACING, (chunk_z << 4) + b as i32 * SAMPLE_SPACING);
                self.fill_column(terrain, x, z, origin_y, &mut columns[(a * OWN_COLUMNS + b) * levels..][..levels]);
            }
        }
        columns
    }

    pub fn assemble_samples(neighbours: impl Fn(i32, i32) -> Arc<Vec<i32>>, height: i32) -> Vec<i32> {
        let levels = sample_levels(height);
        let mut samples = vec![0; sample_count(height)];
        for (xi, &(dx, a)) in SAMPLE_OWNERS.iter().enumerate() {
            for (zi, &(dz, b)) in SAMPLE_OWNERS.iter().enumerate() {
                let owner = neighbours(dx, dz);
                let source = &owner[(a * OWN_COLUMNS + b) * levels..][..levels];
                samples[sample_index(xi, zi, 0, levels)..][..levels].copy_from_slice(source);
            }
        }
        samples
    }

    pub fn point(&self, terrain: &BedrockTerrain, x: i32, y: i32, z: i32) -> Biome {
        match &self.source {
            BiomeSource::MultiNoise => {
                let target = terrain.climate_column(x, z).point(y);
                self.tree.biome(self.tree.find(&target, None))
            }
            flat => Self::flat_biome(flat, x, z),
        }
    }

    fn flat_biome(source: &BiomeSource, x: i32, z: i32) -> Biome {
        match source {
            BiomeSource::Fixed(biome) => biome.clone(),
            BiomeSource::Checkerboard(checkerboard) => checkerboard.at(x, z),
            BiomeSource::MultiNoise => unreachable!("climate sampling does not use a flat biome"),
        }
    }

    pub fn volume(&self, samples: &[i32], chunk_x: i32, chunk_z: i32, min_y: i32, height: i32) -> BiomeVolume {
        BiomeVolume::resolve(samples, self.level_seed, chunk_x, chunk_z, min_y, height)
    }
}
