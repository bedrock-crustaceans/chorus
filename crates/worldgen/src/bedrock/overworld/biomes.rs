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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::rng::xoroshiro::Xoroshiro;
    use crate::shared::terrain::{Checkerboard, OverworldSettings, Preset};
    use chorus_level::dimension_definition::DimensionDefinition;

    const SEED: i64 = 1234567;
    const STEP: i32 = 16;
    const STEPS: i32 = 600;

    fn terrain(settings: &OverworldSettings) -> BedrockTerrain {
        BedrockTerrain::new(&Xoroshiro::new(SEED).fork_positional(), settings, DimensionDefinition::OVERWORLD)
    }

    fn biomes_along_a_line(settings: &OverworldSettings) -> Vec<Biome> {
        let (terrain, sampler) = (terrain(settings), BiomeSampler::new(SEED as i32, settings.biome_source.clone()).expect("the biome tree parses"));
        (0..STEPS).map(|step| sampler.point(&terrain, step * STEP, 64, 0)).collect()
    }

    fn changes(biomes: &[Biome]) -> usize {
        biomes.windows(2).filter(|pair| pair[0] != pair[1]).count()
    }

    #[test]
    fn large_biomes_change_biomes_less_often() {
        let normal = changes(&biomes_along_a_line(&Preset::Default.settings()));
        let large = changes(&biomes_along_a_line(&Preset::LargeBiomes.settings()));
        assert!(normal > 20, "{normal}");
        assert!(large < normal, "{large} against {normal}");
    }

    #[test]
    fn large_biomes_stretch_the_erosion_noise() {
        let crossings = |settings: &OverworldSettings| {
            let terrain = terrain(settings);
            let signs: Vec<bool> = (0..STEPS).map(|step| terrain.erosion_and_offset(step * STEP, 0).0 > 0.0).collect();
            signs.windows(2).filter(|pair| pair[0] != pair[1]).count()
        };
        let normal = crossings(&Preset::Default.settings());
        let large = crossings(&Preset::LargeBiomes.settings());
        assert!(normal > 8, "{normal}");
        assert!(large * 3 < normal, "{large} against {normal}");
    }

    #[test]
    fn a_single_biome_fills_every_sample() {
        let settings = OverworldSettings::single_biome(Biome::DESERT);
        assert!(biomes_along_a_line(&settings).iter().all(|biome| *biome == Biome::DESERT));
        let (terrain, sampler) = (terrain(&settings), BiomeSampler::new(SEED as i32, settings.biome_source.clone()).expect("the biome tree parses"));
        assert!(sampler.samples(&terrain, 3, -7, -64, 384).iter().all(|&id| id == Biome::DESERT.bedrock_id()));
    }

    #[test]
    fn a_checkerboard_source_alternates_between_its_biomes() {
        let checkerboard = Checkerboard {
            biomes: vec![Biome::PLAINS, Biome::DESERT],
            scale: 2,
        };
        let settings = OverworldSettings {
            biome_source: BiomeSource::Checkerboard(checkerboard),
            ..OverworldSettings::default()
        };
        let biomes = biomes_along_a_line(&settings);
        assert_eq!(changes(&biomes), (STEPS as usize - 1) / 4);
        assert!(biomes.iter().all(|biome| *biome == Biome::PLAINS || *biome == Biome::DESERT));
        let (terrain, sampler) = (terrain(&settings), BiomeSampler::new(SEED as i32, settings.biome_source.clone()).expect("the biome tree parses"));
        let samples = sampler.samples(&terrain, 2, 2, -64, 384);
        assert!(samples.iter().all(|&id| id == Biome::PLAINS.bedrock_id()));
        assert!(sampler.samples(&terrain, 6, 2, -64, 384).iter().all(|&id| id == Biome::DESERT.bedrock_id()));
    }

    #[test]
    fn amplified_offsets_rise_higher() {
        let highest = |settings: &OverworldSettings| {
            let terrain = terrain(settings);
            (0..2000).map(|step| terrain.erosion_and_offset(step * 5, step * 3 - 1000).1).fold(f32::MIN, f32::max)
        };
        let normal = highest(&Preset::Default.settings());
        let amplified = highest(&Preset::Amplified.settings());
        assert!(amplified > normal * 1.5, "{amplified} against {normal}");
    }
}
