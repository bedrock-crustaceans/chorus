use super::perlin::PerlinNoise;
use super::stack::NoiseStack;
use crate::shared::rng::xoroshiro::Xoroshiro;

const SECOND_SAMPLE_FACTOR: f64 = 1.0181268882175227;
const PERLIN_STANDARD_DEVIATION: f64 = 0.2702247831245211;

struct OctaveInfo {
    index: i32,
    frequency: f64,
    amplitude: f64,
}

pub struct NormalNoise {
    octaves: Vec<OctaveInfo>,
    normalization: f64,
}

impl NormalNoise {
    pub fn parity(first_octave: i32, amplitudes: &[f64]) -> Self {
        let unit = Self::build_octaves(first_octave, 1.0, amplitudes);
        let unit_normalization = Self::normalization(&unit);
        let base_amplitude = if unit_normalization == 0.0 {
            1.0
        } else {
            Self::parity_normalization(amplitudes) / unit_normalization
        };
        let octaves = Self::build_octaves(first_octave, base_amplitude, amplitudes);
        let normalization = Self::normalization(&octaves);
        Self { octaves, normalization }
    }

    fn build_octaves(base_octave: i32, base_amplitude: f64, modifiers: &[f64]) -> Vec<OctaveInfo> {
        let count = modifiers.len() as i32;
        let mut frequency = 2f64.powi(base_octave);
        let mut amplitude = base_amplitude * 0.5f64.powi(-(count - 1)) / (0.5f64.powi(-count) - 1.0);
        let mut octaves = Vec::new();
        for (i, &modifier) in modifiers.iter().enumerate() {
            if modifier != 0.0 {
                octaves.push(OctaveInfo {
                    index: base_octave + i as i32,
                    frequency,
                    amplitude: amplitude * modifier,
                });
            }
            frequency *= 2.0;
            amplitude *= 0.5;
        }
        octaves
    }

    fn normalization(octaves: &[OctaveInfo]) -> f64 {
        let target_amplitude: f64 = octaves.iter().map(|octave| octave.amplitude.abs()).sum();
        let variance: f64 = octaves.iter().map(|octave| (PERLIN_STANDARD_DEVIATION * octave.amplitude.abs()).powi(2)).sum();
        let deviation = variance.sqrt();
        if deviation == 0.0 {
            0.0
        } else {
            target_amplitude * 0.3333333333333333 / (deviation * 2f64.sqrt())
        }
    }

    fn parity_normalization(modifiers: &[f64]) -> f64 {
        let used: Vec<usize> = modifiers.iter().enumerate().filter(|(_, m)| **m != 0.0).map(|(i, _)| i).collect();
        let span = (used.last().unwrap_or(&0) - used.first().unwrap_or(&0)) as f64;
        0.5 * 0.3333333333333333 / (0.1 * (1.0 + 1.0 / (span + 1.0)))
    }

    pub fn create(&self, random: &mut Xoroshiro) -> NoiseStack {
        let first = random.fork_positional();
        let second = random.fork_positional();
        self.build(|seed| PerlinNoise::new(&mut first.hash_of(seed)), |seed| PerlinNoise::new(&mut second.hash_of(seed)))
    }

    fn build(&self, first: impl Fn(&str) -> PerlinNoise, second: impl Fn(&str) -> PerlinNoise) -> NoiseStack {
        let mut stack = NoiseStack::default();
        for octave in &self.octaves {
            let seed = format!("octave_{}", octave.index);
            let value_factor = (self.normalization * octave.amplitude) as f32;
            stack.add(first(&seed), octave.frequency, value_factor);
            stack.add(second(&seed), octave.frequency * SECOND_SAMPLE_FACTOR, value_factor);
        }
        stack
    }
}
