use super::data::BiomeSurface;
use super::simplex::PerlinSimplex;
use crate::shared::biome::Biome;
use crate::shared::rng::mersenne::MersenneTwister;
use std::sync::OnceLock;

const FROZEN_MAJOR_SEED: u32 = 3456;
const FROZEN_MAJOR_LEVELS: usize = 3;
const FROZEN_MINOR_SEED: u32 = 2345;
const HEIGHT_SEED: u32 = 1234;
const SINGLE_LEVEL: usize = 1;
const FROZEN_MAJOR_SCALE: f32 = 0.05;
const FROZEN_MINOR_SCALE: f32 = 0.2;
const FROZEN_PATCH_SCALE: f32 = 0.09;
const HEIGHT_SCALE: f32 = 0.125;
const FROZEN_SUM_LIMIT: f32 = 0.3;
const FROZEN_PATCH_LIMIT: f32 = 0.8;
const FROZEN_TEMPERATURE: f32 = 0.2;
const COOLING_START_ABOVE_SEA: i32 = 17;
const COOLING_NOISE_STEPS_PER_UNIT: f32 = 64.0;
const COOLING_NOISE_STEP: f32 = 0.125;
const COOLING_RATE: f32 = -0.05;
const COOLING_DISTANCE: f32 = 40.0;

pub struct Temperature {
    frozen_major: PerlinSimplex,
    frozen_minor: PerlinSimplex,
    height: PerlinSimplex,
}

impl Temperature {
    pub fn new() -> Self {
        Self {
            frozen_major: PerlinSimplex::new(&mut MersenneTwister::new(FROZEN_MAJOR_SEED), FROZEN_MAJOR_LEVELS, false),
            frozen_minor: PerlinSimplex::new(&mut MersenneTwister::new(FROZEN_MINOR_SEED), SINGLE_LEVEL, false),
            height: PerlinSimplex::new(&mut MersenneTwister::new(HEIGHT_SEED), SINGLE_LEVEL, false),
        }
    }

    pub fn at(&self, surface: &BiomeSurface, x: i32, y: i32, z: i32, sea_level: i32) -> f32 {
        self.at_climate(surface.temperature, surface.partially_frozen, x, y, z, sea_level)
    }

    fn at_climate(&self, base: f32, partially_frozen: bool, x: i32, y: i32, z: i32, sea_level: i32) -> f32 {
        let (fx, fz) = (x as f32, z as f32);
        let mut temperature = base;
        if partially_frozen {
            let major = self.frozen_major.sum_2d(fx * FROZEN_MAJOR_SCALE, fz * FROZEN_MAJOR_SCALE);
            let minor = self.frozen_minor.sum_2d(fx * FROZEN_MINOR_SCALE, fz * FROZEN_MINOR_SCALE);
            if major + minor < FROZEN_SUM_LIMIT && self.frozen_minor.sum_2d(fx * FROZEN_PATCH_SCALE, fz * FROZEN_PATCH_SCALE) < FROZEN_PATCH_LIMIT {
                temperature = FROZEN_TEMPERATURE;
            }
        }
        let cooling_start = sea_level + COOLING_START_ABOVE_SEA;
        if y > cooling_start {
            let noise = self.height.sum_2d(fx * HEIGHT_SCALE, fz * HEIGHT_SCALE);
            let cached = ((noise * COOLING_NOISE_STEPS_PER_UNIT) as i32 as i8) as f32 * COOLING_NOISE_STEP;
            temperature += (y as f32 + cached - cooling_start as f32) * COOLING_RATE / COOLING_DISTANCE;
        }
        temperature
    }
}

pub fn temperature_at(biome: &Biome, x: i32, y: i32, z: i32, sea_level: i32) -> Option<f32> {
    static TEMPERATURE: OnceLock<Temperature> = OnceLock::new();
    let (base, partially_frozen) = super::data::climate(biome)?;
    Some(TEMPERATURE.get_or_init(Temperature::new).at_climate(base, partially_frozen, x, y, z, sea_level))
}

pub fn info_noise(x: f32, z: f32) -> f32 {
    static NOISE: OnceLock<PerlinSimplex> = OnceLock::new();
    NOISE.get_or_init(|| PerlinSimplex::new(&mut MersenneTwister::new(FROZEN_MINOR_SEED), SINGLE_LEVEL, false)).sum_2d(x, z)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cooling_follows_the_sea_level() {
        for biome in [Biome::PLAINS, Biome::TAIGA, Biome::OCEAN] {
            let at_vanilla = temperature_at(&biome, 100, 140, 100, 63);
            assert_eq!(temperature_at(&biome, 100, 140 + 37, 100, 63 + 37), at_vanilla);
            assert_ne!(temperature_at(&biome, 100, 140, 100, 63 + 37), at_vanilla);
        }
    }
}
