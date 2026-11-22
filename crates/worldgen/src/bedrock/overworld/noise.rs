use crate::shared::noise::lerp;
use crate::shared::rng::random::RandomSource;
use crate::shared::rng::xoroshiro::Xoroshiro;

const DOUBLE_UNIT: f64 = 1.1102230246251565e-16;
const ORIGIN_SCALE: f32 = 256.0;
const PERMUTATION_SIZE: usize = 256;
const SECOND_SAMPLE_FACTOR: f32 = 1.0181268;
const VALUE_FACTOR_NUMERATOR: f32 = 0.16666667;
const VALUE_FACTOR_SCALE: f32 = 0.1;
pub const GRADIENTS: [[f32; 3]; 16] = [
    [1.0, 1.0, 0.0],
    [-1.0, 1.0, 0.0],
    [1.0, -1.0, 0.0],
    [-1.0, -1.0, 0.0],
    [1.0, 0.0, 1.0],
    [-1.0, 0.0, 1.0],
    [1.0, 0.0, -1.0],
    [-1.0, 0.0, -1.0],
    [0.0, 1.0, 1.0],
    [0.0, -1.0, 1.0],
    [0.0, 1.0, -1.0],
    [0.0, -1.0, -1.0],
    [1.0, 1.0, 0.0],
    [0.0, -1.0, 1.0],
    [-1.0, 1.0, 0.0],
    [0.0, -1.0, -1.0],
];

pub fn next_double(random: &mut Xoroshiro) -> f64 {
    ((random.next_long() as u64) >> 11) as f64 * DOUBLE_UNIT
}

pub fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

pub fn gradient(hash: usize, x: f32, y: f32, z: f32) -> f32 {
    let direction = GRADIENTS[hash & 15];
    direction[0] * x + direction[1] * y + direction[2] * z
}

fn shuffled_permutation(random: &mut Xoroshiro) -> [u8; PERMUTATION_SIZE] {
    let mut permutation = [0u8; PERMUTATION_SIZE];
    for (i, slot) in permutation.iter_mut().enumerate() {
        *slot = i as u8;
    }
    for i in 0..PERMUTATION_SIZE {
        let j = random.next_int_bounded((PERMUTATION_SIZE - i) as i32) as usize + i;
        permutation.swap(i, j);
    }
    permutation
}

pub struct ImprovedNoise {
    origin: [f32; 3],
    permutation: [u8; PERMUTATION_SIZE],
}

impl ImprovedNoise {
    pub fn new(random: &mut Xoroshiro) -> Self {
        let origin = [
            (next_double(random) * ORIGIN_SCALE as f64) as f32,
            (next_double(random) * ORIGIN_SCALE as f64) as f32,
            (next_double(random) * ORIGIN_SCALE as f64) as f32,
        ];
        Self {
            origin,
            permutation: shuffled_permutation(random),
        }
    }

    fn p(&self, index: i32) -> i32 {
        self.permutation[(index & 255) as usize] as i32
    }

    fn sample_cell(&self, x: f32, y: f32, z: f32, cell_x: i32, cell_y: i32, cell_z: i32) -> f32 {
        let a = self.p(cell_x) + cell_y;
        let b = self.p(cell_x + 1) + cell_y;
        let aa = self.p(a) + cell_z;
        let ab = self.p(a + 1) + cell_z;
        let ba = self.p(b) + cell_z;
        let bb = self.p(b + 1) + cell_z;
        let u = fade(x);
        let v = fade(y);
        let w = fade(z);
        let hash = |index: i32| self.p(index) as usize;
        let x1 = lerp(u, gradient(hash(aa), x, y, z), gradient(hash(ba), x - 1.0, y, z));
        let x2 = lerp(u, gradient(hash(ab), x, y - 1.0, z), gradient(hash(bb), x - 1.0, y - 1.0, z));
        let x3 = lerp(u, gradient(hash(aa + 1), x, y, z - 1.0), gradient(hash(ba + 1), x - 1.0, y, z - 1.0));
        let x4 = lerp(u, gradient(hash(ab + 1), x, y - 1.0, z - 1.0), gradient(hash(bb + 1), x - 1.0, y - 1.0, z - 1.0));
        lerp(w, lerp(v, x1, x2), lerp(v, x3, x4))
    }
}

struct Octave {
    noise: ImprovedNoise,
    input_factor: f32,
    value_factor: f32,
}

pub struct MultiOctaveNoise {
    octaves: Vec<Octave>,
}

impl MultiOctaveNoise {
    pub fn new(random: &mut Xoroshiro, first_octave: i32, amplitudes: &[f64]) -> Self {
        let count = amplitudes.len();
        let factory = random.fork_positional();
        let denominator = ((1u32 << count) - 1) as f32;
        let octaves = amplitudes
            .iter()
            .enumerate()
            .filter(|(_, amplitude)| **amplitude != 0.0)
            .map(|(i, &amplitude)| {
                let octave = first_octave + i as i32;
                let value_factor = ((1u32 << (count - 1 - i)) as f32) / denominator;
                Octave {
                    noise: ImprovedNoise::new(&mut factory.hash_of(&format!("octave_{octave}"))),
                    input_factor: 2f64.powi(octave) as f32,
                    value_factor: value_factor * amplitude as f32,
                }
            })
            .collect();
        Self { octaves }
    }

    pub fn sample(&self, x: f32, y: f32, z: f32) -> f32 {
        let mut sum = 0.0f32;
        for octave in &self.octaves {
            let noise = &octave.noise;
            let fx = x * octave.input_factor + noise.origin[0];
            let fy = y * octave.input_factor + noise.origin[1];
            let fz = z * octave.input_factor + noise.origin[2];
            let (floor_x, floor_y, floor_z) = (fx.floor(), fy.floor(), fz.floor());
            sum += noise.sample_cell(fx - floor_x, fy - floor_y, fz - floor_z, floor_x as i32, floor_y as i32, floor_z as i32) * octave.value_factor;
        }
        sum
    }
}

pub struct NormalNoise {
    first: MultiOctaveNoise,
    second: MultiOctaveNoise,
    value_factor: f32,
}

impl NormalNoise {
    pub fn new(random: &mut Xoroshiro, first_octave: i32, amplitudes: &[f64]) -> Self {
        let first = MultiOctaveNoise::new(random, first_octave, amplitudes);
        let second = MultiOctaveNoise::new(random, first_octave, amplitudes);
        let mut used = amplitudes.iter().enumerate().filter(|(_, amplitude)| **amplitude != 0.0).map(|(i, _)| i);
        let span = match (used.clone().next(), used.next_back()) {
            (Some(min), Some(max)) => (max - min) as f32 + 1.0,
            _ => 2.0,
        };
        Self {
            first,
            second,
            value_factor: VALUE_FACTOR_NUMERATOR / ((1.0 / span + 1.0) * VALUE_FACTOR_SCALE),
        }
    }

    pub fn value(&self, x: f32, y: f32, z: f32) -> f32 {
        (self.first.sample(x, y, z) + self.second.sample(x * SECOND_SAMPLE_FACTOR, y * SECOND_SAMPLE_FACTOR, z * SECOND_SAMPLE_FACTOR)) * self.value_factor
    }
}
