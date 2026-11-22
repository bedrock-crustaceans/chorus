use crate::bedrock::overworld::noise::GRADIENTS;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::rng::random::RandomSource;
use crate::shared::rng::xoroshiro::Xoroshiro;

const PERMUTATION_SIZE: usize = 256;
const GRADIENT_COUNT: usize = 12;
const ORIGIN_SCALE: f32 = 256.0;
const SKEW_2D: f32 = 0.36602542;
const UNSKEW_2D: f32 = 0.21132487;
const SKEW_3D: f32 = 0.33333334;
const UNSKEW_3D: f32 = 0.16666667;
const SECOND_CORNER_3D: f32 = 0.33333334;
const LAST_CORNER_3D_OFFSET: f32 = 0.5;
const CORNER_FALLOFF_2D: f32 = 0.5;
const CORNER_FALLOFF_3D: f32 = 0.6;
const SCALE_2D: f32 = 70.0;
const SCALE_3D: f32 = 32.0;
const FREQUENCY_STEP: f32 = 0.5;
pub const REGION_WIDTH: usize = 16;
const REGION_STEP: f32 = 0.125;
const REGION_AMPLITUDE: f32 = 0.55;

pub trait SimplexRandom {
    fn float(&mut self) -> f32;
    fn bounded(&mut self, bound: i32) -> i32;
}

impl SimplexRandom for MersenneTwister {
    fn float(&mut self) -> f32 {
        self.next_float()
    }

    fn bounded(&mut self, bound: i32) -> i32 {
        self.next_int_bounded(bound)
    }
}

impl SimplexRandom for Xoroshiro {
    fn float(&mut self) -> f32 {
        self.next_float()
    }

    fn bounded(&mut self, bound: i32) -> i32 {
        self.next_int_bounded(bound)
    }
}

fn fast_floor(value: f32) -> i32 {
    value as i32 - (value <= 0.0) as i32
}

fn corner_2d(gradient: usize, x: f32, y: f32) -> f32 {
    let falloff = CORNER_FALLOFF_2D - x * x - y * y;
    if falloff < 0.0 {
        return 0.0;
    }
    let direction = GRADIENTS[gradient];
    falloff * falloff * falloff * falloff * (direction[0] * x + direction[1] * y)
}

fn corner_3d(gradient: usize, x: f32, y: f32, z: f32) -> f32 {
    let falloff = ((CORNER_FALLOFF_3D - x * x) - y * y) - z * z;
    if falloff < 0.0 {
        return 0.0;
    }
    let direction = GRADIENTS[gradient];
    ((direction[2] * z + direction[0] * x) + direction[1] * y) * falloff * falloff * falloff * falloff
}

pub struct SimplexNoise {
    origin: [f32; 3],
    permutation: [u8; PERMUTATION_SIZE * 2],
}

impl SimplexNoise {
    pub fn new<R: SimplexRandom>(random: &mut R, new_order: bool) -> Self {
        let first = random.float();
        let second = random.float();
        let third = random.float();
        let origin = if new_order {
            [first * ORIGIN_SCALE, second * ORIGIN_SCALE, third * ORIGIN_SCALE]
        } else {
            [third * ORIGIN_SCALE, second * ORIGIN_SCALE, first * ORIGIN_SCALE]
        };
        let mut permutation = [0u8; PERMUTATION_SIZE * 2];
        for (i, slot) in permutation.iter_mut().take(PERMUTATION_SIZE).enumerate() {
            *slot = i as u8;
        }
        for i in 0..PERMUTATION_SIZE {
            let j = random.bounded((PERMUTATION_SIZE - i) as i32) as usize + i;
            permutation.swap(i, j);
            permutation[i + PERMUTATION_SIZE] = permutation[i];
        }
        Self { origin, permutation }
    }

    fn p(&self, index: i32) -> usize {
        self.permutation[index as usize] as usize
    }

    fn corners_2d(&self, x: f32, y: f32) -> f32 {
        let skew = (x + y) * SKEW_2D;
        let i = fast_floor(x + skew);
        let j = fast_floor(y + skew);
        let unskew = (i + j) as f32 * UNSKEW_2D;
        let x0 = x - (i as f32 - unskew);
        let y0 = y - (j as f32 - unskew);
        let (i1, j1) = if x0 > y0 { (1, 0) } else { (0, 1) };
        let x1 = x0 - i1 as f32 + UNSKEW_2D;
        let y1 = y0 - j1 as f32 + UNSKEW_2D;
        let x2 = x0 - 1.0 + (UNSKEW_2D + UNSKEW_2D);
        let y2 = y0 - 1.0 + (UNSKEW_2D + UNSKEW_2D);
        let ii = i & 255;
        let jj = j & 255;
        let g0 = self.p(ii + self.p(jj) as i32) % GRADIENT_COUNT;
        let g1 = self.p(ii + i1 + self.p(jj + j1) as i32) % GRADIENT_COUNT;
        let g2 = self.p(ii + 1 + self.p(jj + 1) as i32) % GRADIENT_COUNT;
        let c0 = corner_2d(g0, x0, y0);
        let c1 = corner_2d(g1, x1, y1);
        let c2 = corner_2d(g2, x2, y2);
        (c0 + c1) + c2
    }

    pub fn sample_2d(&self, x: f32, y: f32) -> f32 {
        self.corners_2d(x, y) * SCALE_2D
    }

    pub fn sample_3d(&self, x: f32, y: f32, z: f32) -> f32 {
        let skew = (x + y + z) * SKEW_3D;
        let i = fast_floor(skew + x);
        let j = fast_floor(y + skew);
        let k = fast_floor(skew + z);
        let unskew = (i + j + k) as f32 * UNSKEW_3D;
        let x0 = x - (i as f32 - unskew);
        let y0 = y - (j as f32 - unskew);
        let z0 = z - (k as f32 - unskew);
        let ((i1, j1, k1), (i2, j2, k2)) = if y0 <= x0 {
            if z0 <= y0 {
                ((1, 0, 0), (1, 1, 0))
            } else if x0 >= z0 {
                ((1, 0, 0), (1, 0, 1))
            } else {
                ((0, 0, 1), (1, 0, 1))
            }
        } else if y0 < z0 {
            ((0, 0, 1), (0, 1, 1))
        } else if z0 <= x0 {
            ((0, 1, 0), (1, 1, 0))
        } else {
            ((0, 1, 0), (0, 1, 1))
        };
        let x1 = (x0 - i1 as f32) + UNSKEW_3D;
        let y1 = (y0 - j1 as f32) + UNSKEW_3D;
        let z1 = (z0 - k1 as f32) + UNSKEW_3D;
        let x2 = (x0 - i2 as f32) + SECOND_CORNER_3D;
        let y2 = (y0 - j2 as f32) + SECOND_CORNER_3D;
        let z2 = (z0 - k2 as f32) + SECOND_CORNER_3D;
        let x3 = x0 + -1.0 + LAST_CORNER_3D_OFFSET;
        let y3 = y0 + -1.0 + LAST_CORNER_3D_OFFSET;
        let z3 = z0 + -1.0 + LAST_CORNER_3D_OFFSET;
        let ii = i & 255;
        let jj = j & 255;
        let kk = k & 255;
        let gradient = |a: i32, b: i32, c: i32| self.p(ii + a + self.p(jj + b + self.p(kk + c) as i32) as i32) % GRADIENT_COUNT;
        let n0 = corner_3d(gradient(0, 0, 0), x0, y0, z0);
        let n1 = corner_3d(gradient(i1, j1, k1), x1, y1, z1);
        let n2 = corner_3d(gradient(i2, j2, k2), x2, y2, z2);
        let n3 = corner_3d(gradient(1, 1, 1), x3, y3, z3);
        (((n0 + n1) + n2) + n3) * SCALE_3D
    }

    fn add_region(&self, buffer: &mut [f32; REGION_WIDTH * REGION_WIDTH], origin_x: f32, origin_z: f32, step: f32, amplitude: f32) {
        for zi in 0..REGION_WIDTH {
            let y = (zi as f32 + origin_z) * step + self.origin[1];
            for xi in 0..REGION_WIDTH {
                let x = (xi as f32 + origin_x) * step + self.origin[0];
                buffer[zi * REGION_WIDTH + xi] += self.corners_2d(x, y) * SCALE_2D * amplitude;
            }
        }
    }
}

pub struct PerlinSimplex {
    levels: Vec<SimplexNoise>,
}

impl PerlinSimplex {
    pub fn new<R: SimplexRandom>(random: &mut R, levels: usize, new_order: bool) -> Self {
        Self {
            levels: (0..levels).map(|_| SimplexNoise::new(random, new_order)).collect(),
        }
    }

    pub fn sum_2d(&self, x: f32, z: f32) -> f32 {
        let mut sum = 0.0f32;
        let mut frequency = 1.0f32;
        for level in &self.levels {
            sum += level.sample_2d(frequency * x, frequency * z) / frequency;
            frequency *= FREQUENCY_STEP;
        }
        sum
    }

    pub fn sum_3d(&self, x: f32, y: f32, z: f32) -> f32 {
        let mut sum = 0.0f32;
        let mut frequency = 1.0f32;
        for level in &self.levels {
            sum += level.sample_3d(x * frequency, y * frequency, z * frequency) / frequency;
            frequency *= FREQUENCY_STEP;
        }
        sum
    }

    pub fn region(&self, chunk_x: i32, chunk_z: i32) -> [f32; REGION_WIDTH * REGION_WIDTH] {
        let mut buffer = [0.0f32; REGION_WIDTH * REGION_WIDTH];
        let mut frequency = 1.0f32;
        for level in &self.levels {
            let step = frequency * REGION_STEP;
            level.add_region(
                &mut buffer,
                (chunk_x * REGION_WIDTH as i32) as f32,
                (chunk_z * REGION_WIDTH as i32) as f32,
                step,
                REGION_AMPLITUDE / frequency,
            );
            frequency *= FREQUENCY_STEP;
        }
        buffer
    }
}
