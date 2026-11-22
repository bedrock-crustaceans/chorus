use crate::shared::noise::lerp;
use crate::shared::rng::mersenne::MersenneTwister;

const GRADIENTS: [[f32; 3]; 16] = [
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

const SIMPLEX_GRADIENTS: [[f32; 2]; 12] = [
    [1.0, 1.0],
    [-1.0, 1.0],
    [1.0, -1.0],
    [-1.0, -1.0],
    [1.0, 0.0],
    [-1.0, 0.0],
    [1.0, 0.0],
    [-1.0, 0.0],
    [0.0, 1.0],
    [0.0, -1.0],
    [0.0, 1.0],
    [0.0, -1.0],
];

const SIMPLEX_F2: f32 = f32::from_bits(0x3ebb_67ae);
const SIMPLEX_G2: f32 = 0.211_324_87;
const SIMPLEX_FACTOR: f32 = 70.0;
const SIMPLEX_G3: f32 = 0.166_666_67;
const SIMPLEX_F3: f32 = 0.333_333_34;
const TABLE_SIZE: usize = 256;

pub struct PerlinOctave {
    x_offset: f32,
    y_offset: f32,
    z_offset: f32,
    table: [u8; TABLE_SIZE],
}

impl PerlinOctave {
    fn new(random: &mut MersenneTwister) -> Self {
        let x_offset = random.next_float() * 256.0;
        let y_offset = random.next_float() * 256.0;
        let z_offset = random.next_float() * 256.0;
        let mut table: [u8; TABLE_SIZE] = std::array::from_fn(|i| i as u8);
        for i in 0..TABLE_SIZE {
            let j = random.next_int_bounded(TABLE_SIZE as i32 - i as i32) as usize;
            table.swap(i, i + j);
        }
        Self { x_offset, y_offset, z_offset, table }
    }

    fn hash(&self, value: i32) -> i32 {
        self.table[(value & 0xFF) as usize] as i32
    }

    fn x_hashes(&self, x: i32, y: i32) -> [i32; 4] {
        let low = self.hash(x) + y;
        let high = self.hash(x + 1) + y;
        [self.hash(low), self.hash(low + 1), self.hash(high), self.hash(high + 1)]
    }

    fn blend_x(&self, hashes: [i32; 4], z: i32, [fx, fy, fz]: [f32; 3], sx: f32) -> [f32; 4] {
        let blend = |first: i32, second: i32, y: f32, depth: f32| {
            let low = gradient_dot(self.hash(first), fx, y, depth);
            let high = gradient_dot(self.hash(second), fx - 1.0, y, depth);
            (high - low) * sx + low
        };
        [
            blend(hashes[0] + z, hashes[2] + z, fy, fz),
            blend(hashes[1] + z, hashes[3] + z, fy - 1.0, fz),
            blend(hashes[0] + z + 1, hashes[2] + z + 1, fy, fz - 1.0),
            blend(hashes[1] + z + 1, hashes[3] + z + 1, fy - 1.0, fz - 1.0),
        ]
    }

    pub fn sample_offset(&self, x: f32, y: f32, z: f32) -> f32 {
        let (ix, iy, iz) = (floor(x) as i32, floor(y) as i32, floor(z) as i32);
        let fractions = [x - ix as f32, y - iy as f32, z - iz as f32];
        let hashes = self.x_hashes(ix, iy);
        let corners = self.blend_x(hashes, iz, fractions, smoothstep(fractions[0]));
        lerp3(smoothstep(fractions[1]), smoothstep(fractions[2]), corners)
    }
}

fn gradient_dot(hash: i32, x: f32, y: f32, z: f32) -> f32 {
    let gradient = GRADIENTS[(hash & 15) as usize];
    gradient[0] * x + gradient[1] * y + gradient[2] * z
}

fn smoothstep(t: f32) -> f32 {
    let cube = t * t * t;
    ((t * 6.0 - 15.0) * t + 10.0) * cube
}

fn lerp3(ty: f32, tz: f32, d: [f32; 4]) -> f32 {
    let z0 = lerp(ty, d[0], d[1]);
    let z1 = lerp(ty, d[2], d[3]);
    lerp(tz, z0, z1)
}

fn floor(value: f32) -> f32 {
    if value.abs() < 8_388_608.0 {
        let truncated = (value as i32 as f32).copysign(value);
        if truncated > value { truncated - 1.0 } else { truncated }
    } else {
        value.floor()
    }
}

fn split(value: f32, scale: f32, offset: f32) -> (i32, f32, f32) {
    let world = value * scale + offset;
    let ground = floor(world);
    let fraction = world - ground;
    (ground as i32 & 0xFF, fraction, smoothstep(fraction))
}

pub fn perlin_octaves(random: &mut MersenneTwister, count: usize) -> Vec<PerlinOctave> {
    (0..count).map(|_| PerlinOctave::new(random)).collect()
}

pub fn fill_cells<const SIDE: usize, const ROWS: usize>(octaves: &[PerlinOctave], origin: [f32; 3], [xz_scale, y_scale]: [f32; 2]) -> Vec<f32> {
    let columns = SIDE * SIDE;
    let mut sums = vec![0.0f32; columns * ROWS];
    let mut factor = 1.0f32;
    for octave in octaves {
        fill_octave::<SIDE, ROWS>(octave, origin, xz_scale * factor, y_scale * factor, 1.0 / factor, &mut sums);
        factor *= 0.5;
    }
    sums
}

fn fill_octave<const SIDE: usize, const ROWS: usize>(octave: &PerlinOctave, origin: [f32; 3], xz_scale: f32, y_scale: f32, inverse_factor: f32, sums: &mut [f32]) {
    let columns = SIDE * SIDE;
    let fudge_y = y_scale * 0.0 + octave.y_offset;
    let fudge_index = fudge_y as i32;
    let rows = OctaveRows {
        octave,
        y_scale,
        fudge_index,
        fudge_fraction: fudge_y - fudge_index as f32,
        inverse_factor,
    };
    let x_parts: [(i32, f32, f32); 25] = std::array::from_fn(|index| split((index / SIDE) as f32 + origin[0], xz_scale, octave.x_offset));
    let z_parts: [(i32, f32, f32); 25] = std::array::from_fn(|index| split((index % SIDE) as f32 + origin[2], xz_scale, octave.z_offset));
    let x_parts = &x_parts[..columns];
    let z_parts = &z_parts[..columns];
    let y_values: [f32; 41] = std::array::from_fn(|row| (row as f32 + origin[1]) * y_scale + octave.y_offset);
    let y_values = &y_values[..ROWS];
    let last_row = rows.row(y_values[ROWS - 1], -1).0;
    if rows.row(y_values[0], last_row).2.is_none() {
        fill_columns::<SIDE, ROWS>(&rows, x_parts, z_parts, y_values, sums);
        return;
    }
    let mut corners = [[0.0f32; 25]; 4];
    let mut last_iy = -1;
    for (row, &y) in y_values.iter().enumerate() {
        let (iy, fraction, gradient) = rows.row(y, last_iy);
        if let Some(gradient_fraction) = gradient {
            for column_x in 0..SIDE {
                let hashes = octave.x_hashes(x_parts[column_x * SIDE].0, iy);
                for index in column_x * SIDE..(column_x + 1) * SIDE {
                    let (_, fx, sx) = x_parts[index];
                    let (iz, fz, _) = z_parts[index];
                    let computed = octave.blend_x(hashes, iz, [fx, gradient_fraction, fz], sx);
                    for (target, value) in corners.iter_mut().zip(computed) {
                        target[index] = value;
                    }
                }
            }
            last_iy = iy;
        }
        let sy = smoothstep(fraction);
        for index in 0..columns {
            let near = (corners[1][index] - corners[0][index]) * sy + corners[0][index];
            let far = (corners[3][index] - corners[2][index]) * sy + corners[2][index];
            sums[index * ROWS + row] += ((far - near) * z_parts[index].2 + near) * inverse_factor;
        }
    }
}

struct OctaveRows<'a> {
    octave: &'a PerlinOctave,
    y_scale: f32,
    fudge_index: i32,
    fudge_fraction: f32,
    inverse_factor: f32,
}

impl OctaveRows<'_> {
    fn row(&self, y: f32, last_iy: i32) -> (i32, f32, Option<f32>) {
        let ground = floor(y);
        let iy = ground as i32 & 0xFF;
        let fraction = y - ground;
        let gradient = if y as i32 == self.fudge_index {
            Some(self.fudge_fraction)
        } else if iy == last_iy {
            None
        } else {
            Some(fraction - floor(fraction / self.y_scale) * self.y_scale)
        };
        (iy, fraction, gradient)
    }
}

fn fill_columns<const SIDE: usize, const ROWS: usize>(rows: &OctaveRows, x_parts: &[(i32, f32, f32)], z_parts: &[(i32, f32, f32)], y_values: &[f32], sums: &mut [f32]) {
    let columns = SIDE * SIDE;
    for index in 0..columns {
        let (ix, fx, sx) = x_parts[index];
        let (iz, fz, sz) = z_parts[index];
        let mut last_iy = -1;
        let mut corners = [0.0f32; 4];
        for (row, &y) in y_values.iter().enumerate() {
            let (iy, fraction, gradient) = rows.row(y, last_iy);
            if let Some(gradient_fraction) = gradient {
                corners = rows.octave.blend_x(rows.octave.x_hashes(ix, iy), iz, [fx, gradient_fraction, fz], sx);
                last_iy = iy;
            }
            let sy = smoothstep(fraction);
            let near = (corners[1] - corners[0]) * sy + corners[0];
            let far = (corners[3] - corners[2]) * sy + corners[2];
            sums[index * ROWS + row] += ((far - near) * sz + near) * rows.inverse_factor;
        }
    }
}

pub struct SimplexNoise {
    pub origin: [f32; 2],
    permutation: [u8; TABLE_SIZE],
}

impl SimplexNoise {
    fn new(offsets: [f32; 3], random: &mut MersenneTwister) -> Self {
        let mut permutation: [u8; TABLE_SIZE] = std::array::from_fn(|i| i as u8);
        for i in 0..TABLE_SIZE {
            let j = i + random.next_int_bounded(TABLE_SIZE as i32 - i as i32) as usize;
            permutation.swap(i, j);
        }
        Self {
            origin: [offsets[2] * 256.0, offsets[1] * 256.0],
            permutation,
        }
    }

    fn at(&self, index: i32) -> i32 {
        self.permutation[(index & 255) as usize] as i32
    }

    fn corner(&self, gradient: i32, x: f32, z: f32) -> f32 {
        let falloff = 0.5 - x * x - z * z;
        if 0.0 > falloff {
            return 0.0;
        }
        let [gx, gz] = SIMPLEX_GRADIENTS[(gradient % 12) as usize];
        (z * gz + x * gx) * (falloff * falloff * falloff * falloff)
    }

    pub fn value(&self, x: f32, z: f32) -> f32 {
        let skew = (x + z) * SIMPLEX_F2;
        let (i, j) = (simplex_floor(x + skew), simplex_floor(skew + z));
        let unskew = (i + j) as f32 * SIMPLEX_G2;
        let (x0, z0) = (x - (i as f32 - unskew), z - (j as f32 - unskew));
        let (i1, j1) = if x0 > z0 { (1, 0) } else { (0, 1) };
        let (x1, z1) = (x0 - i1 as f32 + SIMPLEX_G2, z0 - j1 as f32 + SIMPLEX_G2);
        let doubled = SIMPLEX_G2 + SIMPLEX_G2;
        let (x2, z2) = (x0 + -1.0 + doubled, z0 + -1.0 + doubled);
        let (ii, jj) = (i & 255, j & 255);
        let first = self.corner(self.at(ii + self.at(jj)), x0, z0);
        let second = self.corner(self.at(ii + i1 + self.at(jj + j1)), x1, z1);
        let third = self.corner(self.at(ii + 1 + self.at(jj + 1)), x2, z2);
        (first + second + third) * SIMPLEX_FACTOR
    }

    fn corner3(&self, gradient: i32, x: f32, y: f32, z: f32) -> f32 {
        let falloff = 0.6 - x * x - y * y - z * z;
        if 0.0 > falloff {
            return 0.0;
        }
        let [gx, gy, gz] = GRADIENTS[(gradient % 12) as usize];
        (y * gy + x * gx + z * gz) * (falloff * falloff * falloff * falloff)
    }

    pub fn value3(&self, x: f32, y: f32, z: f32) -> f32 {
        let skew = (x + y + z) * SIMPLEX_F3;
        let (i, j, k) = (simplex_floor(x + skew), simplex_floor(y + skew), simplex_floor(skew + z));
        let unskew = (i + j + k) as f32 * SIMPLEX_G3;
        let (x0, y0, z0) = (x - (i as f32 - unskew), y - (j as f32 - unskew), z - (k as f32 - unskew));
        let ([i1, j1, k1], [i2, j2, k2]) = if x0 >= y0 {
            if y0 >= z0 {
                ([1, 0, 0], [1, 1, 0])
            } else if x0 >= z0 {
                ([1, 0, 0], [1, 0, 1])
            } else {
                ([0, 0, 1], [1, 0, 1])
            }
        } else if z0 > y0 {
            ([0, 0, 1], [0, 1, 1])
        } else if z0 > x0 {
            ([0, 1, 0], [0, 1, 1])
        } else {
            ([0, 1, 0], [1, 1, 0])
        };
        let offset = |value: f32, step: i32, add: f32| value - step as f32 + add;
        let first_offset = [offset(x0, i1, SIMPLEX_G3), offset(y0, j1, SIMPLEX_G3), offset(z0, k1, SIMPLEX_G3)];
        let second_offset = [offset(x0, i2, SIMPLEX_F3), offset(y0, j2, SIMPLEX_F3), offset(z0, k2, SIMPLEX_F3)];
        let third_offset = [x0 + -1.0 + 0.5, y0 + -1.0 + 0.5, z0 + -1.0 + 0.5];
        let (ii, jj, kk) = (i & 255, j & 255, k & 255);
        let hash = |a: i32, b: i32, c: i32| self.at(ii + a + self.at(jj + b + self.at(kk + c)));
        let first = self.corner3(hash(0, 0, 0), x0, y0, z0);
        let second = self.corner3(hash(i1, j1, k1), first_offset[0], first_offset[1], first_offset[2]);
        let third = self.corner3(hash(i2, j2, k2), second_offset[0], second_offset[1], second_offset[2]);
        let fourth = self.corner3(hash(1, 1, 1), third_offset[0], third_offset[1], third_offset[2]);
        (first + second + third + fourth) * 32.0
    }
}

fn simplex_floor(value: f32) -> i32 {
    value as i32 - (value <= 0.0) as i32
}

pub fn simplex_octaves(random: &mut MersenneTwister, count: usize) -> Vec<SimplexNoise> {
    (0..count)
        .map(|_| {
            let offsets = [random.next_float(), random.next_float(), random.next_float()];
            SimplexNoise::new(offsets, random)
        })
        .collect()
}

pub struct LegacyPerlin {
    octaves: Vec<PerlinOctave>,
    pub normalisation: f32,
}

impl LegacyPerlin {
    pub fn new(random: &mut MersenneTwister, count: usize) -> Self {
        let mut sum = 0.0f32;
        let mut factor = 1.0f32;
        for _ in 0..count {
            sum += 1.0 / factor;
            factor *= 0.5;
        }
        Self {
            octaves: perlin_octaves(random, count),
            normalisation: 1.0 / sum,
        }
    }

    pub fn value(&self, [x, y, z]: [f32; 3]) -> f32 {
        let mut total = 0.0f32;
        let mut factor = 1.0f32;
        for octave in &self.octaves {
            total += octave.sample_offset(x * factor + octave.x_offset, y * factor + octave.y_offset, z * factor + octave.z_offset) / factor;
            factor *= 0.5;
        }
        total
    }
}
