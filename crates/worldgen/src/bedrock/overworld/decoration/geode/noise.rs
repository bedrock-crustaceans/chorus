use super::super::ore::Draw;
use crate::shared::noise::lerp;

const PERMUTATION_SIZE: usize = 256;

const OFFSET_SCALE: f64 = 256.0;

const BANK_TOP_DOUBLES: usize = 3;

const BANK_SLOTS: usize = 4;

const FIRST_OCTAVE: i32 = -4;

const OUT_OF_RANGE_SLOT_WORDS: usize = 262;

const VALUE_FACTOR: f32 = 0.833_333_3;

const SECOND_BANK_SCALE: f32 = 1.0181268;

const INVERSE_SQRT_MAGIC: i32 = 0x5f3759df;

const INVERSE_SQRT_THREE_HALVES: f32 = 1.5;

pub(super) fn inverse_sqrt(number: f32) -> f32 {
    let half = number * 0.5;
    let bits = INVERSE_SQRT_MAGIC.wrapping_sub((number.to_bits() as i32) >> 1);
    let estimate = f32::from_bits(bits as u32);
    estimate * (INVERSE_SQRT_THREE_HALVES - half * estimate * estimate)
}

pub(super) fn threshold(points: f32, offset: f32) -> f32 {
    1.0 / (points + offset).max(0.0).sqrt()
}

fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn gradient(hash: usize, x: f32, y: f32, z: f32) -> f32 {
    const TABLE: [[f32; 3]; 16] = [
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
    let [gx, gy, gz] = TABLE[hash & 15];
    gx * x + gy * y + gz * z
}

#[derive(Clone)]
struct Level {
    offset: [f32; 3],
    permutation: [u8; PERMUTATION_SIZE],
}

fn shuffle<D: Draw>(random: &mut D) -> [u8; PERMUTATION_SIZE] {
    let mut permutation: [u8; PERMUTATION_SIZE] = std::array::from_fn(|index| index as u8);
    for index in 0..PERMUTATION_SIZE {
        let other = index + random.below((PERMUTATION_SIZE - index) as i32) as usize;
        permutation.swap(index, other);
    }
    permutation
}

impl Level {
    fn build<D: Draw>(random: &mut D) -> Self {
        let offset = std::array::from_fn(|_| (random.double() * OFFSET_SCALE) as f32);
        Self { offset, permutation: shuffle(random) }
    }

    fn at(&self, index: i32) -> i32 {
        self.permutation[(index & 255) as usize] as i32
    }

    fn sample(&self, x: f32, y: f32, z: f32) -> f32 {
        let frequency = 2f32.powi(FIRST_OCTAVE);
        let (fx, fy, fz) = (x * frequency + self.offset[0], y * frequency + self.offset[1], z * frequency + self.offset[2]);
        let (floor_x, floor_y, floor_z) = (fx.floor(), fy.floor(), fz.floor());
        let (cell_x, cell_y, cell_z) = (floor_x as i32, floor_y as i32, floor_z as i32);
        let (x, y, z) = (fx - floor_x, fy - floor_y, fz - floor_z);
        let (u, v, w) = (fade(x), fade(y), fade(z));
        let a = self.at(cell_x) + cell_y;
        let b = self.at(cell_x + 1) + cell_y;
        let (aa, ab) = (self.at(a) + cell_z, self.at(a + 1) + cell_z);
        let (ba, bb) = (self.at(b) + cell_z, self.at(b + 1) + cell_z);
        let g = |hash: i32, x: f32, y: f32, z: f32| gradient(hash as usize, x, y, z);
        let x1 = lerp(u, g(self.at(aa), x, y, z), g(self.at(ba), x - 1.0, y, z));
        let x2 = lerp(u, g(self.at(ab), x, y - 1.0, z), g(self.at(bb), x - 1.0, y - 1.0, z));
        let x3 = lerp(u, g(self.at(aa + 1), x, y, z - 1.0), g(self.at(ba + 1), x - 1.0, y, z - 1.0));
        let x4 = lerp(u, g(self.at(ab + 1), x, y - 1.0, z - 1.0), g(self.at(bb + 1), x - 1.0, y - 1.0, z - 1.0));
        lerp(w, lerp(v, x1, x2), lerp(v, x3, x4))
    }
}

pub struct GeodeNoise {
    banks: [Level; 2],
}

impl GeodeNoise {
    pub fn new<D: Draw>(random: &mut D) -> Self {
        Self {
            banks: [Self::bank(random), Self::bank(random)],
        }
    }

    fn bank<D: Draw>(random: &mut D) -> Level {
        for _ in 0..BANK_TOP_DOUBLES {
            random.double();
        }
        shuffle(random);
        let mut level = None;
        for slot in (0..BANK_SLOTS).rev() {
            if slot == 0 {
                level = Some(Level::build(random));
            } else {
                (0..OUT_OF_RANGE_SLOT_WORDS).for_each(|_| {
                    random.word();
                });
            }
        }
        level.expect("slot zero is built last")
    }

    pub fn sample(&self, x: f32, y: f32, z: f32) -> f32 {
        let first = self.banks[0].sample(x, y, z);
        let second = self.banks[1].sample(x * SECOND_BANK_SCALE, y * SECOND_BANK_SCALE, z * SECOND_BANK_SCALE);
        (first + second) * VALUE_FACTOR
    }
}
