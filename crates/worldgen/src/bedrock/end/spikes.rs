use crate::shared::math::sine;
use crate::shared::rng::mersenne::MersenneTwister;

pub const COUNT: usize = 10;
const ANGLE_STEP: f32 = 0.314_159_27;
const CIRCLE_RADIUS: f32 = 42.0;
const SIZES: i32 = COUNT as i32;
const RADIUS_DIVISOR: i32 = 3;
const RADIUS_BASE: i32 = 2;
const HEIGHT_FACTOR: i32 = 3;
const HEIGHT_BASE: i32 = 76;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Spike {
    pub x: i32,
    pub z: i32,
    pub radius: i32,
    pub height: i32,
    pub guarded: bool,
}

pub fn spikes(level_seed: u32) -> [Spike; COUNT] {
    let mut random = MersenneTwister::new(level_seed);
    let mut sizes: [i32; COUNT] = std::array::from_fn(|index| index as i32);
    for index in 1..SIZES as usize {
        let target = (random.next_u32() % (index as u32 + 1)) as usize;
        sizes.swap(index, target);
    }
    std::array::from_fn(|index| {
        let angle = index as f32 * ANGLE_STEP + -std::f32::consts::PI;
        let turn = angle + angle;
        let size = sizes[index];
        Spike {
            x: (sine::cos(turn) * CIRCLE_RADIUS) as i32,
            z: (sine::sin(turn) * CIRCLE_RADIUS) as i32,
            radius: size / RADIUS_DIVISOR + RADIUS_BASE,
            height: size * HEIGHT_FACTOR + HEIGHT_BASE,
            guarded: size == 1 || size == 2,
        }
    })
}
