use super::trig::{cos, sin};
use crate::shared::CHUNK_WIDTH;
use crate::shared::rng::mersenne::MersenneTwister;
use std::f32::consts::PI;

const START_CHANCE: i32 = 100;
const START_Y_SPAN: i32 = 58;
const START_Y_BASE: i32 = 10;
const START_DRAWS_SKIPPED_AFTER_Y: usize = 1;
const START_DRAWS_SKIPPED_AFTER_GIANT_CHECK: usize = 1;
const YAW_SCALE: f32 = 2.0;
const PITCH_SCALE: f32 = 0.25;
const THICKNESS_SCALE: f32 = 3.0;
const Y_STRETCH: f32 = 4.0;
const LENGTH_BASE: i32 = 120;
const LENGTH_JITTER: u32 = 30;
const RADIUS_BASE: f32 = 1.5;
const RADIUS_JITTER_BASE: f32 = 0.75;
const RADIUS_JITTER_SPAN: f32 = 0.25;
const PITCH_DECAY: f32 = 0.7;
const PITCH_CHANGE_DECAY: f32 = 0.8;
const YAW_CHANGE_DECAY: f32 = 0.5;
const PITCH_CHANGE_GAIN: f32 = 0.05;
const YAW_CHANGE_GAIN: f32 = 0.05;
const YAW_DELTA_SCALE: f32 = 4.0;
const SKIP_STEP_MASK: u32 = 3;
const WIDTH_PROFILE_LENGTH: usize = 320;
const WIDTH_PROFILE_FIRST_PART: usize = 128;
const WIDTH_REPEAT_CHANCE: i32 = 3;
const WIDTH_BASE: f32 = 1.0;
pub const VERTICAL_DIVISOR: f32 = 6.0;
const REACH_MARGIN: f32 = 2.0;
const NEIGHBOUR_MARGIN: f32 = 16.0;
const TOP_MARGIN: i32 = 72;

pub const fn max_y(height: i32) -> i32 {
    height - TOP_MARGIN
}

#[derive(Clone, Copy)]
pub struct Step {
    pub center: [f32; 3],
    pub horizontal_radius: f32,
    pub vertical_radius: f32,
    pub remaining: i32,
}

pub struct Canyon {
    pub thickness: f32,
    pub widths: [f32; WIDTH_PROFILE_LENGTH],
}

fn width_profile(first: &mut MersenneTwister, second: &mut MersenneTwister) -> [f32; WIDTH_PROFILE_LENGTH] {
    let mut widths = [0.0; WIDTH_PROFILE_LENGTH];
    let a = first.next_float();
    let mut value = first.next_float() * a + WIDTH_BASE;
    widths[0] = value * value;
    for (index, width) in widths.iter_mut().enumerate().skip(1) {
        let random = if index < WIDTH_PROFILE_FIRST_PART { &mut *first } else { &mut *second };
        if random.next_int_bounded(WIDTH_REPEAT_CHANCE) == 0 {
            let a = random.next_float();
            value = random.next_float() * a + WIDTH_BASE;
        }
        *width = value * value;
    }
    widths
}

pub fn starts(first: u32) -> bool {
    first.is_multiple_of(START_CHANCE as u32)
}

pub fn canyon(random: &mut MersenneTwister, chunk_x: i32, chunk_z: i32, min_y: i32, steps: &mut Vec<Step>) -> Option<Canyon> {
    steps.clear();
    if random.next_int_bounded(START_CHANCE) != 0 {
        return None;
    }
    let x = random.next_int_bounded(CHUNK_WIDTH) + chunk_x * CHUNK_WIDTH;
    let y = random.next_int_bounded(START_Y_SPAN) - min_y + START_Y_BASE;
    random.skip(START_DRAWS_SKIPPED_AFTER_Y);
    random.skip(START_DRAWS_SKIPPED_AFTER_GIANT_CHECK);
    let z = random.next_int_bounded(CHUNK_WIDTH) + chunk_z * CHUNK_WIDTH;
    let yaw = random.next_float() * PI;
    let pitch = random.next_float() - 0.5;
    let first = random.next_float() * THICKNESS_SCALE;
    let thickness = random.next_float() * THICKNESS_SCALE + first;
    random.next_float();
    let seed = random.next_u32() >> 1;
    let mut walk = MersenneTwister::new(seed);
    let length = LENGTH_BASE - (walk.next_u32() % LENGTH_JITTER) as i32;
    let widths = width_profile(&mut walk, &mut MersenneTwister::new(seed));
    let mut at = [x as f32, y as f32, z as f32];
    let (mut yaw, mut pitch) = (yaw * YAW_SCALE, pitch * PITCH_SCALE);
    let (mut pitch_change, mut yaw_change) = (0.0f32, 0.0f32);
    for step in 0..length {
        let sine = sin(step as f32 * PI / length as f32);
        let base_radius = sine * thickness + RADIUS_BASE;
        let (horizontal_draw, vertical_draw) = (walk.next_float(), walk.next_float());
        let (cos_pitch, sin_pitch) = (cos(pitch), sin(pitch));
        at[0] += cos(yaw) * cos_pitch;
        at[1] += sin_pitch;
        at[2] += sin(yaw) * cos_pitch;
        let decayed_pitch = pitch * PITCH_DECAY;
        let decayed_pitch_change = pitch_change * PITCH_CHANGE_DECAY;
        let decayed_yaw_change = yaw_change * YAW_CHANGE_DECAY;
        let [pitch_a, pitch_b, pitch_scale] = [walk.next_float(), walk.next_float(), walk.next_float()];
        let yaw_difference = walk.next_float() - walk.next_float();
        let yaw_delta = walk.next_float() * yaw_difference * YAW_DELTA_SCALE;
        if walk.next_u32() & SKIP_STEP_MASK != 0 {
            let horizontal_radius = (horizontal_draw * RADIUS_JITTER_SPAN + RADIUS_JITTER_BASE) * base_radius;
            let vertical_radius = (vertical_draw * RADIUS_JITTER_SPAN + RADIUS_JITTER_BASE) * (base_radius * Y_STRETCH);
            steps.push(Step {
                center: at,
                horizontal_radius,
                vertical_radius,
                remaining: length - step,
            });
        }
        pitch = decayed_pitch + pitch_change * PITCH_CHANGE_GAIN;
        yaw += yaw_change * YAW_CHANGE_GAIN;
        let pitch_delta = pitch_scale * (pitch_a - pitch_b);
        pitch_change = decayed_pitch_change + (pitch_delta + pitch_delta);
        yaw_change = decayed_yaw_change + yaw_delta;
    }
    Some(Canyon { thickness, widths })
}

impl Canyon {
    pub fn reach(&self) -> f32 {
        REACH_MARGIN + self.thickness + NEIGHBOUR_MARGIN
    }
}
