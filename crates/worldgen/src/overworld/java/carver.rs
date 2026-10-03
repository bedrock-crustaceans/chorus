use std::f32::consts::PI;
use std::sync::LazyLock;

use super::random::{Legacy, RandomSource, WorldgenRandom};
use super::terrain::{HEIGHT, MIN_Y};

const RANGE: i32 = 4;
const PROTECTED_BLOCKS_ON_TOP: i32 = 7;
pub const CARVER_RADIUS: i32 = 8;

static SIN: LazyLock<Vec<f32>> = LazyLock::new(|| (0..65536).map(|i| (i as f64 / 10430.378350470453).sin() as f32).collect());

pub fn sin(value: f64) -> f32 {
    SIN[((value * 10430.378350470453) as i64 & 65535) as usize]
}

pub fn cos(value: f64) -> f32 {
    SIN[((value * 10430.378350470453 + 16384.0) as i64 & 65535) as usize]
}

pub struct CarvingMask {
    min_y: i32,
    max_y: i32,
    height: i32,
    bits: Vec<bool>,
}

impl CarvingMask {
    pub fn new() -> Self {
        let (min_y, max_y) = (MIN_Y + 1, MIN_Y + HEIGHT - 1 - PROTECTED_BLOCKS_ON_TOP);
        let height = max_y - min_y + 1;
        Self {
            min_y,
            max_y,
            height,
            bits: vec![false; (256 * height) as usize],
        }
    }

    fn index(&self, x: i32, y: i32, z: i32) -> usize {
        (y - self.min_y + (z + (x << 4)) * self.height) as usize
    }

    fn carve(&mut self, x: i32, y: i32, z: i32) {
        let index = self.index(x, y, z);
        self.bits[index] = true;
    }

    pub fn visit(&self, mut visitor: impl FnMut(i32, i32, i32, i32)) {
        for column in 0..256 {
            let base = (column * self.height) as usize;
            let (x, z) = (column >> 4 & 15, column & 15);
            let mut y = 0;
            while y < self.height {
                if !self.bits[base + y as usize] {
                    y += 1;
                    continue;
                }
                let bottom = y;
                while y < self.height && self.bits[base + y as usize] {
                    y += 1;
                }
                visitor(x, z, bottom + self.min_y, y - 1 + self.min_y);
            }
        }
    }
}

struct Target {
    chunk_x: i32,
    chunk_z: i32,
}

impl Target {
    fn middle_x(&self) -> f64 {
        ((self.chunk_x << 4) + 8) as f64
    }

    fn middle_z(&self) -> f64 {
        ((self.chunk_z << 4) + 8) as f64
    }

    #[allow(clippy::too_many_arguments)]
    fn carve_ellipsoid(&self, mask: &mut CarvingMask, x: f64, y: f64, z: f64, horizontal_radius: f64, vertical_radius: f64, skip: &dyn Fn(f64, f64, f64, i32) -> bool) {
        let max_delta = 16.0 + horizontal_radius * 2.0;
        if (x - self.middle_x()).abs() > max_delta || (z - self.middle_z()).abs() > max_delta {
            return;
        }
        let (min_x, min_z) = (self.chunk_x << 4, self.chunk_z << 4);
        let min_xi = ((x - horizontal_radius).floor() as i32 - min_x - 1).max(0);
        let max_xi = ((x + horizontal_radius).floor() as i32 - min_x).min(15);
        let min_y = ((y - vertical_radius).floor() as i32 - 1).max(mask.min_y);
        let max_y = ((y + vertical_radius).floor() as i32 + 1).min(mask.max_y);
        let min_zi = ((z - horizontal_radius).floor() as i32 - min_z - 1).max(0);
        let max_zi = ((z + horizontal_radius).floor() as i32 - min_z).min(15);
        for xi in min_xi..=max_xi {
            let xd = ((min_x + xi) as f64 + 0.5 - x) / horizontal_radius;
            for zi in min_zi..=max_zi {
                let zd = ((min_z + zi) as f64 + 0.5 - z) / horizontal_radius;
                if xd * xd + zd * zd >= 1.0 {
                    continue;
                }
                let mut world_y = max_y;
                while world_y > min_y {
                    let yd = (world_y as f64 - 0.5 - y) / vertical_radius;
                    if !skip(xd, yd, zd, world_y) {
                        mask.carve(xi, world_y, zi);
                    }
                    world_y -= 1;
                }
            }
        }
    }

    fn can_reach(&self, x: f64, z: f64, step: i32, total: i32, thickness: f32) -> bool {
        let (xd, zd) = (x - self.middle_x(), z - self.middle_z());
        let remaining = (total - step) as f64;
        let rr = (thickness + 2.0 + 16.0) as f64;
        xd * xd + zd * zd - remaining * remaining <= rr * rr
    }
}

fn uniform(random: &mut impl RandomSource, min: f32, max: f32) -> f32 {
    random.next_float() * (max - min) + min
}

fn trapezoid(random: &mut impl RandomSource, min: f32, max: f32, plateau: f32) -> f32 {
    let range = max - min;
    let plateau_start = (range - plateau) / 2.0;
    let plateau_end = range - plateau_start;
    min + random.next_float() * plateau_end + random.next_float() * plateau_start
}

fn very_biased_to_bottom(random: &mut impl RandomSource, min: i32, max: i32) -> i32 {
    let a = random.next_int_bounded(max - min + 1) + 1;
    let b = random.next_int_bounded(a) + 1;
    min + random.next_int_bounded(b)
}

struct Cave {
    probability: f32,
    max_y: i32,
}

impl Cave {
    fn carve(&self, random: &mut impl RandomSource, target: &Target, source_x: i32, source_z: i32, mask: &mut CarvingMask) {
        let max_distance = (RANGE * 2 - 1) << 4;
        let count = very_biased_to_bottom(random, 0, 14);
        for _ in 0..count {
            let x = ((source_x << 4) + random.next_int_bounded(16)) as f64;
            let y = random.next_int_between_inclusive(MIN_Y + 8, self.max_y) as f64;
            let z = ((source_z << 4) + random.next_int_bounded(16)) as f64;
            let horizontal_multiplier = uniform(random, 0.7, 1.4) as f64;
            let vertical_multiplier = uniform(random, 0.8, 1.3) as f64;
            let start_vertical_multiplier = 1.0;
            let floor_level = uniform(random, -1.0, -0.4) as f64;
            let skip = move |xd: f64, yd: f64, zd: f64, _y: i32| yd <= floor_level || xd * xd + yd * yd + zd * zd >= 1.0;
            let mut tunnels = 1;
            if random.next_int_bounded(4) == 0 {
                let y_scale = uniform(random, 0.1, 0.9) as f64;
                let thickness = 1.0 + random.next_float() * 6.0;
                let horizontal_radius = 1.5 + sin((PI / 2.0) as f64) as f64 * thickness as f64;
                target.carve_ellipsoid(mask, x + 1.0, y, z, horizontal_radius, horizontal_radius * y_scale, &skip);
                tunnels += random.next_int_bounded(4);
            }
            for _ in 0..tunnels {
                let horizontal_rotation = random.next_float() * (PI * 2.0);
                let vertical_rotation = (random.next_float() - 0.5) / 4.0;
                let mut thickness = trapezoid(random, 0.0, 3.0, 1.0);
                if random.next_int_bounded(10) == 0 {
                    thickness *= random.next_float() * random.next_float() * 3.0 + 1.0;
                }
                let distance = max_distance - random.next_int_bounded(max_distance / 4);
                let tunnel = Tunnel {
                    horizontal_multiplier,
                    vertical_multiplier,
                    thickness,
                    y_scale: start_vertical_multiplier,
                };
                tunnel.carve(target, random.next_long(), x, y, z, horizontal_rotation, vertical_rotation, 0, distance, mask, &skip);
            }
        }
    }
}

struct Tunnel {
    horizontal_multiplier: f64,
    vertical_multiplier: f64,
    thickness: f32,
    y_scale: f64,
}

impl Tunnel {
    #[allow(clippy::too_many_arguments)]
    fn carve(
        &self,
        target: &Target,
        seed: i64,
        mut x: f64,
        mut y: f64,
        mut z: f64,
        mut horizontal_rotation: f32,
        mut vertical_rotation: f32,
        step: i32,
        distance: i32,
        mask: &mut CarvingMask,
        skip: &dyn Fn(f64, f64, f64, i32) -> bool,
    ) {
        let mut random = Legacy::new(seed);
        let split_point = random.next_int_bounded(distance / 2) + distance / 4;
        let steep = random.next_int_bounded(6) == 0;
        let (mut y_rota, mut x_rota) = (0.0f32, 0.0f32);
        for current in step..distance {
            let horizontal_radius = 1.5 + sin((PI * current as f32 / distance as f32) as f64) as f64 * self.thickness as f64;
            let vertical_radius = horizontal_radius * self.y_scale;
            let cos_x = cos(vertical_rotation as f64);
            x += (cos(horizontal_rotation as f64) * cos_x) as f64;
            y += sin(vertical_rotation as f64) as f64;
            z += (sin(horizontal_rotation as f64) * cos_x) as f64;
            vertical_rotation *= if steep { 0.92 } else { 0.7 };
            vertical_rotation += x_rota * 0.1;
            horizontal_rotation += y_rota * 0.1;
            x_rota *= 0.9;
            y_rota *= 0.75;
            x_rota += (random.next_float() - random.next_float()) * random.next_float() * 2.0;
            y_rota += (random.next_float() - random.next_float()) * random.next_float() * 4.0;
            if current == split_point && self.thickness > 1.0 {
                for side in [-1.0f32, 1.0] {
                    let seed = random.next_long();
                    let branch = Tunnel {
                        thickness: random.next_float() * 0.5 + 0.5,
                        y_scale: 1.0,
                        ..*self
                    };
                    branch.carve(target, seed, x, y, z, horizontal_rotation + side * (PI / 2.0), vertical_rotation / 3.0, current, distance, mask, skip);
                }
                return;
            }
            if random.next_int_bounded(4) != 0 {
                if !target.can_reach(x, z, current, distance, self.thickness) {
                    return;
                }
                target.carve_ellipsoid(mask, x, y, z, horizontal_radius * self.horizontal_multiplier, vertical_radius * self.vertical_multiplier, skip);
            }
        }
    }
}

fn carve_canyon(random: &mut impl RandomSource, target: &Target, source_x: i32, source_z: i32, mask: &mut CarvingMask) {
    let max_distance = (RANGE * 2 - 1) * 16;
    let mut x = ((source_x << 4) + random.next_int_bounded(16)) as f64;
    let mut y = random.next_int_between_inclusive(10, 67) as f64;
    let mut z = ((source_z << 4) + random.next_int_bounded(16)) as f64;
    let mut horizontal_rotation = random.next_float() * (PI * 2.0);
    let mut vertical_rotation = uniform(random, -0.125, 0.125);
    let y_scale = 3.0f64;
    let thickness = trapezoid(random, 0.0, 6.0, 2.0);
    let distance = (max_distance as f32 * uniform(random, 0.75, 1.0)) as i32;

    let mut random = Legacy::new(random.next_long());
    let mut width_factors = vec![0.0f32; HEIGHT as usize];
    let mut width_factor = 1.0f32;
    for (index, factor) in width_factors.iter_mut().enumerate() {
        if index == 0 || random.next_int_bounded(3) == 0 {
            width_factor = 1.0 + random.next_float() * random.next_float();
        }
        *factor = width_factor * width_factor;
    }
    let skip = |xd: f64, yd: f64, zd: f64, world_y: i32| (xd * xd + zd * zd) * width_factors[(world_y - MIN_Y - 1) as usize] as f64 + yd * yd / 6.0 >= 1.0;

    let (mut y_rota, mut x_rota) = (0.0f32, 0.0f32);
    for current in 0..distance {
        let mut horizontal_radius = 1.5 + sin((current as f32 * PI / distance as f32) as f64) as f64 * thickness as f64;
        let mut vertical_radius = horizontal_radius * y_scale;
        horizontal_radius *= uniform(&mut random, 0.75, 1.0) as f64;
        let vertical_multiplier = 1.0 - (0.5 - current as f32 / distance as f32).abs() * 2.0;
        let factor = 1.0 + 0.0 * vertical_multiplier;
        vertical_radius = factor as f64 * vertical_radius * (random.next_float() * (1.0 - 0.75) + 0.75) as f64;
        let xc = cos(vertical_rotation as f64);
        let xs = sin(vertical_rotation as f64);
        x += (cos(horizontal_rotation as f64) * xc) as f64;
        y += xs as f64;
        z += (sin(horizontal_rotation as f64) * xc) as f64;
        vertical_rotation *= 0.7;
        vertical_rotation += x_rota * 0.05;
        horizontal_rotation += y_rota * 0.05;
        x_rota *= 0.8;
        y_rota *= 0.5;
        x_rota += (random.next_float() - random.next_float()) * random.next_float() * 2.0;
        y_rota += (random.next_float() - random.next_float()) * random.next_float() * 4.0;
        if random.next_int_bounded(4) != 0 {
            if !target.can_reach(x, z, current, distance, thickness) {
                return;
            }
            target.carve_ellipsoid(mask, x, y, z, horizontal_radius, vertical_radius, &skip);
        }
    }
}

const CAVE: Cave = Cave { probability: 0.15, max_y: 180 };
const CAVE_EXTRA_UNDERGROUND: Cave = Cave { probability: 0.07, max_y: 47 };
const CANYON_PROBABILITY: f32 = 0.01;

pub fn carve(seed: i64, chunk_x: i32, chunk_z: i32) -> CarvingMask {
    let mut mask = CarvingMask::new();
    let target = Target { chunk_x, chunk_z };
    let mut random = WorldgenRandom::new(Legacy::new(0));
    for dx in -CARVER_RADIUS..=CARVER_RADIUS {
        for dz in -CARVER_RADIUS..=CARVER_RADIUS {
            let (source_x, source_z) = (chunk_x + dx, chunk_z + dz);
            for (index, cave) in [CAVE, CAVE_EXTRA_UNDERGROUND].iter().enumerate() {
                random.set_large_feature_seed(seed.wrapping_add(index as i64), source_x, source_z);
                if random.next_float() <= cave.probability {
                    cave.carve(&mut random, &target, source_x, source_z, &mut mask);
                }
            }
            random.set_large_feature_seed(seed.wrapping_add(2), source_x, source_z);
            if random.next_float() <= CANYON_PROBABILITY {
                carve_canyon(&mut random, &target, source_x, source_z, &mut mask);
            }
        }
    }
    mask
}
