use super::random::{Legacy, RandomSource, Xoroshiro};

const GRADIENT: [[i8; 3]; 16] = [
    [1, 1, 0],
    [-1, 1, 0],
    [1, -1, 0],
    [-1, -1, 0],
    [1, 0, 1],
    [-1, 0, 1],
    [1, 0, -1],
    [-1, 0, -1],
    [0, 1, 1],
    [0, -1, 1],
    [0, 1, -1],
    [0, -1, -1],
    [1, 1, 0],
    [0, -1, 1],
    [-1, 1, 0],
    [0, -1, -1],
];

const HALF_ROUND_OFF: f64 = 16777215.999999998;
const SECOND_SAMPLE_FACTOR: f64 = 1.0181268882175227;
const PERLIN_STANDARD_DEVIATION: f64 = 0.2702247831245211;

pub fn java_string_hash(value: &str) -> i32 {
    value.encode_utf16().fold(0i32, |hash, unit| hash.wrapping_mul(31).wrapping_add(unit as i32))
}

pub fn wrap(x: f64) -> f64 {
    if (-HALF_ROUND_OFF..HALF_ROUND_OFF).contains(&x) {
        x
    } else {
        x - (x / 3.3554432E7 + 0.5).floor() * 3.3554432E7
    }
}

pub fn smoothstep(x: f32) -> f32 {
    x * x * x * (x * (x * 6.0 - 15.0) + 10.0)
}

pub fn lerp(alpha: f32, p0: f32, p1: f32) -> f32 {
    p0 + alpha * (p1 - p0)
}

pub fn lerp2(a1: f32, a2: f32, x00: f32, x10: f32, x01: f32, x11: f32) -> f32 {
    lerp(a2, lerp(a1, x00, x10), lerp(a1, x01, x11))
}

#[allow(clippy::too_many_arguments)]
pub fn lerp3(a1: f32, a2: f32, a3: f32, x000: f32, x100: f32, x010: f32, x110: f32, x001: f32, x101: f32, x011: f32, x111: f32) -> f32 {
    lerp(a3, lerp2(a1, a2, x000, x100, x010, x110), lerp2(a1, a2, x001, x101, x011, x111))
}

const PERLIN_GRADIENT: [[f32; 3]; 16] = {
    let mut out = [[0.0; 3]; 16];
    let mut i = 0;
    while i < 16 {
        out[i] = [GRADIENT[i][0] as f32, GRADIENT[i][1] as f32, GRADIENT[i][2] as f32];
        i += 1;
    }
    out
};

fn grad_dot(hash: i32, x: f32, y: f32, z: f32) -> f32 {
    let [gx, gy, gz] = PERLIN_GRADIENT[(hash & 15) as usize];
    gx * x + gy * y + gz * z
}

pub struct PerlinNoise {
    perms: [u8; 256],
    offset_x: f64,
    offset_y: f64,
    offset_z: f64,
    smear_y: Option<f64>,
}

impl PerlinNoise {
    pub fn new(random: &mut impl RandomSource) -> Self {
        let offset_x = random.next_double() * 256.0;
        let offset_y = random.next_double() * 256.0;
        let offset_z = random.next_double() * 256.0;
        let mut perms = [0u8; 256];
        for (i, perm) in perms.iter_mut().enumerate() {
            *perm = i as u8;
        }
        for i in 0..256 {
            let offset = random.next_int_bounded(256 - i as i32) as usize;
            perms.swap(i, offset + i);
        }
        Self {
            perms,
            offset_x,
            offset_y,
            offset_z,
            smear_y: None,
        }
    }

    pub fn smeared(random: &mut impl RandomSource, smear_y: f64) -> Self {
        Self {
            smear_y: Some(smear_y),
            ..Self::new(random)
        }
    }

    fn permute(&self, x: i32) -> i32 {
        self.perms[(x & 0xFF) as usize] as i32
    }

    pub fn get(&self, raw_x: f64, raw_y: f64, raw_z: f64) -> f32 {
        let x = wrap(raw_x) + self.offset_x;
        let y = wrap(raw_y) + self.offset_y;
        let z = wrap(raw_z) + self.offset_z;
        let floor_x = x.floor() as i32;
        let floor_y = y.floor() as i32;
        let floor_z = z.floor() as i32;
        let relative_x = (x - floor_x as f64) as f32;
        let relative_y = y - floor_y as f64;
        let relative_z = (z - floor_z as f64) as f32;
        let sampled_y = match self.smear_y {
            None => relative_y as f32,
            Some(scale) => {
                let limit = if raw_y >= 0.0 && raw_y < relative_y { raw_y } else { relative_y };
                (relative_y - (limit / scale + 1.0E-7f32 as f64).floor() * scale) as f32
            }
        };
        self.sample_and_lerp(floor_x, floor_y, floor_z, relative_x, sampled_y, relative_z, relative_y as f32)
    }

    fn add_column(&self, raw_x: f64, raw_z: f64, raw_ys: &[f64], frequency: f64, amplitude: f32, out: &mut [f32]) {
        let x = wrap(raw_x * frequency) + self.offset_x;
        let z = wrap(raw_z * frequency) + self.offset_z;
        let floor_x = x.floor() as i32;
        let floor_z = z.floor() as i32;
        let rx = (x - floor_x as f64) as f32;
        let rz = (z - floor_z as f64) as f32;
        let (smooth_x, smooth_z) = (smoothstep(rx), smoothstep(rz));
        let (x0, x1) = (self.permute(floor_x), self.permute(floor_x + 1));
        let mut cell: Option<(i32, [i32; 8])> = None;
        for (&raw_y, out) in raw_ys.iter().zip(out) {
            let raw_y = raw_y * frequency;
            let y = wrap(raw_y) + self.offset_y;
            let floor_y = y.floor() as i32;
            let relative_y = y - floor_y as f64;
            let ry = match self.smear_y {
                None => relative_y as f32,
                Some(scale) => {
                    let limit = if raw_y >= 0.0 && raw_y < relative_y { raw_y } else { relative_y };
                    (relative_y - (limit / scale + 1.0E-7f32 as f64).floor() * scale) as f32
                }
            };
            let hashes = match cell {
                Some((cached, hashes)) if cached == floor_y => hashes,
                _ => {
                    let (xy00, xy01) = (self.permute(x0 + floor_y), self.permute(x0 + floor_y + 1));
                    let (xy10, xy11) = (self.permute(x1 + floor_y), self.permute(x1 + floor_y + 1));
                    let hashes = [
                        self.permute(xy00 + floor_z),
                        self.permute(xy10 + floor_z),
                        self.permute(xy01 + floor_z),
                        self.permute(xy11 + floor_z),
                        self.permute(xy00 + floor_z + 1),
                        self.permute(xy10 + floor_z + 1),
                        self.permute(xy01 + floor_z + 1),
                        self.permute(xy11 + floor_z + 1),
                    ];
                    cell = Some((floor_y, hashes));
                    hashes
                }
            };
            let d000 = grad_dot(hashes[0], rx, ry, rz);
            let d100 = grad_dot(hashes[1], rx - 1.0, ry, rz);
            let d010 = grad_dot(hashes[2], rx, ry - 1.0, rz);
            let d110 = grad_dot(hashes[3], rx - 1.0, ry - 1.0, rz);
            let d001 = grad_dot(hashes[4], rx, ry, rz - 1.0);
            let d101 = grad_dot(hashes[5], rx - 1.0, ry, rz - 1.0);
            let d011 = grad_dot(hashes[6], rx, ry - 1.0, rz - 1.0);
            let d111 = grad_dot(hashes[7], rx - 1.0, ry - 1.0, rz - 1.0);
            *out += amplitude * lerp3(smooth_x, smoothstep(relative_y as f32), smooth_z, d000, d100, d010, d110, d001, d101, d011, d111);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn sample_and_lerp(&self, x: i32, y: i32, z: i32, rx: f32, ry: f32, rz: f32, original_ry: f32) -> f32 {
        let x0 = self.permute(x);
        let x1 = self.permute(x + 1);
        let xy00 = self.permute(x0 + y);
        let xy01 = self.permute(x0 + y + 1);
        let xy10 = self.permute(x1 + y);
        let xy11 = self.permute(x1 + y + 1);
        let d000 = grad_dot(self.permute(xy00 + z), rx, ry, rz);
        let d100 = grad_dot(self.permute(xy10 + z), rx - 1.0, ry, rz);
        let d010 = grad_dot(self.permute(xy01 + z), rx, ry - 1.0, rz);
        let d110 = grad_dot(self.permute(xy11 + z), rx - 1.0, ry - 1.0, rz);
        let d001 = grad_dot(self.permute(xy00 + z + 1), rx, ry, rz - 1.0);
        let d101 = grad_dot(self.permute(xy10 + z + 1), rx - 1.0, ry, rz - 1.0);
        let d011 = grad_dot(self.permute(xy01 + z + 1), rx, ry - 1.0, rz - 1.0);
        let d111 = grad_dot(self.permute(xy11 + z + 1), rx - 1.0, ry - 1.0, rz - 1.0);
        lerp3(smoothstep(rx), smoothstep(original_ry), smoothstep(rz), d000, d100, d010, d110, d001, d101, d011, d111)
    }
}

struct Layer {
    noise: PerlinNoise,
    frequency: f64,
    amplitude: f32,
}

pub const MAX_COLUMN: usize = 64;

#[derive(Default)]
pub struct NoiseStack {
    layers: Vec<Layer>,
}

impl NoiseStack {
    fn add(&mut self, noise: PerlinNoise, frequency: f64, amplitude: f32) {
        self.layers.push(Layer { noise, frequency, amplitude });
    }

    pub fn get(&self, x: f64, y: f64, z: f64) -> f32 {
        let mut value = 0.0f32;
        for layer in &self.layers {
            value += layer.amplitude * layer.noise.get(x * layer.frequency, y * layer.frequency, z * layer.frequency);
        }
        value
    }

    pub fn column(&self, x: f64, z: f64, ys: &[f64], out: &mut [f32]) {
        out.fill(0.0);
        for layer in &self.layers {
            layer.noise.add_column(x, z, ys, layer.frequency, layer.amplitude, out);
        }
    }

    pub fn column_subset(&self, x: f64, z: f64, ys: &[f64], indices: &[usize], out: &mut [f32]) {
        if indices.is_empty() {
            return;
        }
        let mut picked = [0.0f64; MAX_COLUMN];
        let mut values = [0.0f32; MAX_COLUMN];
        let count = indices.len();
        for (slot, &i) in picked.iter_mut().zip(indices) {
            *slot = ys[i];
        }
        self.column(x, z, &picked[..count], &mut values[..count]);
        for (&i, &value) in indices.iter().zip(&values[..count]) {
            out[i] = value;
        }
    }
}

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

    pub fn create_legacy(&self, seed: i64) -> NoiseStack {
        let mut random = Legacy::new(seed);
        let first = random.next_long();
        let second = random.next_long();
        let legacy = |factory: i64, name: &str| Legacy::new(java_string_hash(name) as i64 ^ factory);
        self.build(|seed| PerlinNoise::new(&mut legacy(first, seed)), |seed| PerlinNoise::new(&mut legacy(second, seed)))
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

pub struct BlendedNoise {
    min_limit: NoiseStack,
    max_limit: NoiseStack,
    main: NoiseStack,
    xz_multiplier: f64,
    y_multiplier: f64,
    xz_factor: f64,
    y_factor: f64,
}

impl BlendedNoise {
    pub fn new(random: &mut Xoroshiro, xz_scale: f64, y_scale: f64, xz_factor: f64, y_factor: f64, smear_scale_multiplier: f64) -> Self {
        let xz_multiplier = 684.412 * xz_scale;
        let y_multiplier = 684.412 * y_scale;
        let limit_smear = y_multiplier * smear_scale_multiplier;
        let main_smear = limit_smear / y_factor;
        let min_limit = Self::fbm(random, -15, limit_smear, 0.99998474f32 as f64);
        let max_limit = Self::fbm(random, -15, limit_smear, 0.99998474f32 as f64);
        let main = Self::fbm(random, -7, main_smear, 12.75);
        Self {
            min_limit,
            max_limit,
            main,
            xz_multiplier,
            y_multiplier,
            xz_factor,
            y_factor,
        }
    }

    fn fbm(random: &mut Xoroshiro, first_octave: i32, smear_y: f64, value_factor: f64) -> NoiseStack {
        let octaves = -first_octave + 1;
        let mut factor = 1.0;
        let mut value_factor = value_factor / (2f64.powi(octaves) - 1.0);
        let mut stack = NoiseStack::default();
        for _ in 0..octaves {
            stack.add(PerlinNoise::smeared(random, smear_y * factor), factor, value_factor as f32);
            factor /= 2.0;
            value_factor *= 2.0;
        }
        stack
    }

    pub fn column(&self, x: i32, z: i32, ys: &[i32], indices: &[usize], out: &mut [f32]) {
        let (x, z) = (x as f64, z as f64);
        let main_ys: Vec<f64> = ys.iter().map(|&y| y as f64 * self.y_multiplier / self.y_factor).collect();
        let limit_ys: Vec<f64> = ys.iter().map(|&y| y as f64 * self.y_multiplier).collect();
        let mut main = vec![0.0; ys.len()];
        self.main
            .column_subset(x * self.xz_multiplier / self.xz_factor, z * self.xz_multiplier / self.xz_factor, &main_ys, indices, &mut main);
        let alpha: Vec<f32> = main.iter().map(|value| (value + 0.5).clamp(0.0, 1.0)).collect();
        let needs_min: Vec<usize> = indices.iter().copied().filter(|&i| alpha[i] != 1.0).collect();
        let needs_max: Vec<usize> = indices.iter().copied().filter(|&i| alpha[i] != 0.0).collect();
        let (limit_x, limit_z) = (x * self.xz_multiplier, z * self.xz_multiplier);
        let mut min = vec![0.0; ys.len()];
        let mut max = vec![0.0; ys.len()];
        self.min_limit.column_subset(limit_x, limit_z, &limit_ys, &needs_min, &mut min);
        self.max_limit.column_subset(limit_x, limit_z, &limit_ys, &needs_max, &mut max);
        for &i in indices {
            out[i] = if alpha[i] == 0.0 {
                min[i]
            } else if alpha[i] == 1.0 {
                max[i]
            } else {
                lerp(alpha[i], min[i], max[i])
            };
        }
    }
}

pub struct SimplexNoise {
    perms: [u8; 256],
}

impl SimplexNoise {
    const F2: f64 = 0.5 * (1.7320508075688772 - 1.0);
    const G2: f64 = (3.0 - 1.7320508075688772) / 6.0;

    pub fn without_offset(random: &mut impl RandomSource) -> Self {
        for _ in 0..3 {
            random.next_double();
        }
        let mut perms = [0u8; 256];
        for (i, perm) in perms.iter_mut().enumerate() {
            *perm = i as u8;
        }
        for i in 0..256 {
            let offset = random.next_int_bounded(256 - i as i32) as usize;
            perms.swap(i, offset + i);
        }
        Self { perms }
    }

    fn permute(&self, x: i32) -> i32 {
        self.perms[(x & 0xFF) as usize] as i32
    }

    fn corner(index: i32, x: f64, y: f64) -> f64 {
        let t = 0.5 - x * x - y * y;
        if t < 0.0 {
            return 0.0;
        }
        let [gx, gy, _] = GRADIENT[index as usize];
        t * t * t * t * (gx as f64 * x + gy as f64 * y)
    }

    pub fn get(&self, x: f64, y: f64) -> f32 {
        let (xin, yin) = (x, y);
        let s = (xin + yin) * Self::F2;
        let i = (xin + s).floor() as i32;
        let j = (yin + s).floor() as i32;
        let t = (i + j) as f64 * Self::G2;
        let x0 = xin - (i as f64 - t);
        let y0 = yin - (j as f64 - t);
        let (i1, j1) = if x0 > y0 { (1, 0) } else { (0, 1) };
        let x1 = x0 - i1 as f64 + Self::G2;
        let y1 = y0 - j1 as f64 + Self::G2;
        let x2 = x0 - 1.0 + 2.0 * Self::G2;
        let y2 = y0 - 1.0 + 2.0 * Self::G2;
        let (ii, jj) = (i & 0xFF, j & 0xFF);
        let gi0 = self.permute(ii + self.permute(jj)) % 12;
        let gi1 = self.permute(ii + i1 + self.permute(jj + j1)) % 12;
        let gi2 = self.permute(ii + 1 + self.permute(jj + 1)) % 12;
        (70.0 * (Self::corner(gi0, x0, y0) + Self::corner(gi1, x1, y1) + Self::corner(gi2, x2, y2))) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn column_matches_points() {
        let mut random = Xoroshiro::new(42);
        let blended = BlendedNoise::new(&mut random, 0.25, 0.125, 80.0, 160.0, 8.0);
        let stacks = [
            NormalNoise::parity(-8, &[0.5, 1.0, 2.0, 1.0, 2.0, 1.0, 0.0, 2.0, 0.0]).create(&mut random),
            NormalNoise::parity(-3, &[1.0]).create(&mut random),
            NormalNoise::parity(-16, &[1.0; 16]).create(&mut random),
        ];
        let ys: Vec<f64> = (0..49).map(|cy| (-64 + cy * 8) as f64 * 0.6666666666666666).collect();
        let smeared_ys: Vec<f64> = (0..49).map(|cy| (-64 + cy * 8) as f64 * blended.y_multiplier / blended.y_factor).collect();
        let mut out = vec![0.0f32; 49];
        let mut checked = 0;
        for (x, z) in [(0, 0), (-1234, 5678), (99999, -4), (30000000, 7)] {
            for stack in &stacks {
                stack.column(x as f64, z as f64, &ys, &mut out);
                for (y, value) in ys.iter().zip(&out) {
                    assert_eq!(value.to_bits(), stack.get(x as f64, *y, z as f64).to_bits());
                    checked += 1;
                }
            }
            let (bx, bz) = (x as f64 * blended.xz_multiplier / blended.xz_factor, z as f64 * blended.xz_multiplier / blended.xz_factor);
            blended.main.column(bx, bz, &smeared_ys, &mut out);
            for (y, value) in smeared_ys.iter().zip(&out) {
                assert_eq!(value.to_bits(), blended.main.get(bx, *y, bz).to_bits());
                checked += 1;
            }
        }
        println!("{checked} column samples matched");
        let start = std::time::Instant::now();
        for i in 0..2000 {
            for stack in &stacks {
                for y in &ys {
                    std::hint::black_box(stack.get(i as f64, *y, -(i as f64)));
                }
            }
        }
        let points = start.elapsed();
        let start = std::time::Instant::now();
        for i in 0..2000 {
            for stack in &stacks {
                stack.column(i as f64, -(i as f64), &ys, &mut out);
                std::hint::black_box(&out);
            }
        }
        println!("points {points:?}, columns {:?}", start.elapsed());
    }
}
