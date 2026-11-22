use super::biome_tree::{BiomeTree, BiomeTreeError};
use super::terrain::BedrockTerrain;
use crate::shared::biome::Biome;
use crate::shared::chunk::biome_volume::{BiomeVolume, SAMPLE_COLUMNS, SAMPLE_SPACING, sample_count, sample_index, sample_levels, sample_origin};
use crate::shared::terrain::BiomeSource;

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

    pub fn samples(&self, terrain: &BedrockTerrain, chunk_x: i32, chunk_z: i32, min_y: i32, height: i32) -> Vec<i32> {
        let levels = sample_levels(height);
        let (origin_x, origin_y, origin_z) = sample_origin(chunk_x, chunk_z, min_y);
        let mut samples = vec![Biome::PLAINS.bedrock_id(); sample_count(height)];
        for xi in 0..SAMPLE_COLUMNS {
            for zi in 0..SAMPLE_COLUMNS {
                let (x, z) = (origin_x + xi as i32 * SAMPLE_SPACING, origin_z + zi as i32 * SAMPLE_SPACING);
                match &self.source {
                    BiomeSource::MultiNoise => {
                        let column = terrain.climate_column(x, z);
                        let mut previous = None;
                        for yi in 0..levels {
                            let nearest = self.tree.find(&column.point(origin_y + yi as i32 * SAMPLE_SPACING), previous);
                            samples[sample_index(xi, zi, yi, levels)] = self.tree.biome_id(nearest);
                            previous = Some(nearest);
                        }
                    }
                    flat => {
                        let biome = Self::flat_biome(flat, x, z).bedrock_id();
                        for yi in 0..levels {
                            samples[sample_index(xi, zi, yi, levels)] = biome;
                        }
                    }
                }
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
