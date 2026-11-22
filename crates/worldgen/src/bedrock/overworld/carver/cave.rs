use super::trig::{cos, sin};
use crate::shared::CHUNK_WIDTH;
use crate::shared::rng::mersenne::MersenneTwister;
use std::f32::consts::{FRAC_PI_2, PI};

const ROOM_THICKNESS_SPAN: f32 = 6.0;
const ROOM_THICKNESS_BASE: f32 = 1.0;
const ROOM_RADIUS_BASE: f32 = 1.5;
const TUNNEL_RADIUS_BASE: f32 = 1.5;
const TUNNEL_LENGTH_BASE: i32 = 112;
const TUNNEL_LENGTH_JITTER: i32 = 28;
const PITCH_SPREAD: f32 = 0.125;
const PITCH_CHANGE_DECAY: f32 = 0.9;
const YAW_CHANGE_DECAY: f32 = 0.75;
const PITCH_CHANGE_GAIN: f32 = 0.1;
const YAW_CHANGE_GAIN: f32 = 0.1;
const YAW_DELTA_SCALE: f32 = 4.0;
const STEEP_PITCH_DECAY: f32 = 0.92;
const FLAT_PITCH_DECAY: f32 = 0.7;
const THICKNESS_BOOST_CHANCE: i32 = 10;
const THICKNESS_BOOST_SCALE: f32 = 3.0;
const BRANCH_THICKNESS_SCALE: f32 = 0.5;
const BRANCH_THICKNESS_BASE: f32 = 0.5;
const BRANCH_PITCH_DIVISOR: f32 = 3.0;
const BRANCH_THICKNESS_LIMIT: f32 = 1.0;
const FLOOR_Y_OFFSET: i32 = 8;

#[derive(Clone, Copy)]
pub struct Segment {
    pub center: [f32; 3],
    pub horizontal_radius: f32,
    pub vertical_radius: f32,
    pub floor_level: f32,
    pub step: i32,
    pub length: i32,
    pub thickness: f32,
}

#[derive(Clone, Copy)]
pub struct Range {
    pub min: f32,
    pub max: f32,
}

impl Range {
    const fn new(min: f32, max: f32) -> Self {
        Self { min, max }
    }

    fn draw(self, random: &mut MersenneTwister) -> f32 {
        if self.min == self.max { self.min } else { self.min + (self.max - self.min) * random.next_float() }
    }
}

pub struct CaveSettings {
    pub skip_carve_chance: i32,
    pub height_limit: i32,
    pub y_scale: Range,
    pub horizontal_radius_multiplier: Range,
    pub vertical_radius_multiplier: Range,
    pub floor_level: Range,
    pub width_modifier: f32,
    pub cave_bound: i32,
    pub room_chance: i32,
    pub extra_tunnels: i32,
    pub skip_segment_chance: i32,
    pub steep_chance: i32,
}

pub const UNDERGROUND_CAVE_CARVER: CaveSettings = CaveSettings {
    skip_carve_chance: 33,
    height_limit: 236,
    y_scale: Range::new(0.1, 0.9),
    horizontal_radius_multiplier: Range::new(0.7, 1.4),
    vertical_radius_multiplier: Range::new(0.8, 1.3),
    floor_level: Range::new(-1.0, -0.4),
    width_modifier: 0.0,
    cave_bound: 40,
    room_chance: 4,
    extra_tunnels: 4,
    skip_segment_chance: 4,
    steep_chance: 6,
};

pub const EXTRA_CAVE_CARVER: CaveSettings = CaveSettings {
    skip_carve_chance: 66,
    height_limit: 103,
    ..UNDERGROUND_CAVE_CARVER
};

#[derive(Clone, Copy)]
struct Cave {
    horizontal_multiplier: f32,
    vertical_multiplier: f32,
    floor_level: f32,
}

#[derive(Clone, Copy)]
struct Tunnel {
    thickness: f32,
    yaw: f32,
    pitch: f32,
    step: i32,
    length: i32,
}

fn unbounded_int(random: &mut MersenneTwister) -> u32 {
    random.next_u32() >> 1
}

fn tunnel_length(random: &mut MersenneTwister) -> i32 {
    TUNNEL_LENGTH_BASE - random.next_int_bounded(TUNNEL_LENGTH_JITTER)
}

fn tunnel_thickness(random: &mut MersenneTwister) -> f32 {
    let first = random.next_float();
    let doubled = first + first;
    let mut thickness = random.next_float() + doubled;
    if random.next_int_bounded(THICKNESS_BOOST_CHANCE) == 0 {
        let a = random.next_float();
        let b = random.next_float();
        thickness *= b * a * THICKNESS_BOOST_SCALE + 1.0;
    }
    thickness
}

impl CaveSettings {
    pub fn starts(&self, [first, second, third, fourth]: [u32; 4]) -> bool {
        let bound = (first % self.cave_bound as u32) as i32;
        let bound = (second % (bound + 1) as u32) as i32;
        let count = (third % (bound + 1) as u32) as i32;
        count != 0 && fourth % self.skip_carve_chance as u32 == 0
    }

    pub fn segments(&self, random: &mut MersenneTwister, chunk_x: i32, chunk_z: i32, out: &mut Vec<Segment>) {
        out.clear();
        let bound = random.next_int_bounded(self.cave_bound);
        let bound = random.next_int_bounded(bound + 1);
        let count = random.next_int_bounded(bound + 1);
        if count == 0 || random.next_int_bounded(self.skip_carve_chance) != 0 {
            return;
        }
        for _ in 0..count {
            let z = random.next_int_bounded(CHUNK_WIDTH) + chunk_z * CHUNK_WIDTH;
            let y = random.next_int_bounded(self.height_limit) + FLOOR_Y_OFFSET;
            let x = random.next_int_bounded(CHUNK_WIDTH) + chunk_x * CHUNK_WIDTH;
            let origin = [x as f32, y as f32, z as f32];
            let cave = Cave {
                horizontal_multiplier: self.horizontal_radius_multiplier.draw(random),
                vertical_multiplier: self.vertical_radius_multiplier.draw(random),
                floor_level: self.floor_level.draw(random),
            };
            let mut tunnels = 1;
            if random.next_int_bounded(self.room_chance) == 0 {
                self.room(random, origin, cave, out);
                tunnels += random.next_int_bounded(self.extra_tunnels);
            }
            for _ in 0..tunnels {
                let yaw = random.next_float() * PI;
                let yaw = yaw + yaw;
                let pitch = random.next_float() - 0.5;
                let pitch = (pitch + pitch) * PITCH_SPREAD;
                let thickness = tunnel_thickness(random);
                let length = tunnel_length(random);
                self.tunnel(
                    random,
                    origin,
                    Tunnel {
                        thickness,
                        yaw,
                        pitch,
                        step: 0,
                        length,
                    },
                    cave,
                    out,
                );
            }
        }
    }

    fn room(&self, parent: &mut MersenneTwister, origin: [f32; 3], cave: Cave, out: &mut Vec<Segment>) {
        let thickness = parent.next_float() * ROOM_THICKNESS_SPAN + ROOM_THICKNESS_BASE;
        let mut random = MersenneTwister::new(unbounded_int(parent));
        let y_scale = self.y_scale.draw(&mut random);
        let length = tunnel_length(&mut random);
        let step = length >> 1;
        let radius = sin(step as f32 * PI / length as f32) * (self.width_modifier + thickness) + ROOM_RADIUS_BASE;
        out.push(Segment {
            center: origin,
            horizontal_radius: radius * cave.horizontal_multiplier,
            vertical_radius: y_scale * radius * cave.vertical_multiplier,
            floor_level: cave.floor_level,
            step,
            length,
            thickness,
        });
    }

    fn tunnel(&self, parent: &mut MersenneTwister, mut at: [f32; 3], mut tunnel: Tunnel, cave: Cave, out: &mut Vec<Segment>) {
        let mut random = MersenneTwister::new(unbounded_int(parent));
        let mut branch_step = 0;
        if tunnel.length != 1 {
            branch_step = (random.next_u32() % (tunnel.length as u32 >> 1)) as i32;
        }
        let pitch_decay = if random.next_int_bounded(self.steep_chance) != 0 { FLAT_PITCH_DECAY } else { STEEP_PITCH_DECAY };
        if tunnel.step >= tunnel.length {
            return;
        }
        branch_step += tunnel.length >> 2;
        let (mut pitch_change, mut yaw_change) = (0.0f32, 0.0f32);
        loop {
            let sine = sin(tunnel.step as f32 * PI / tunnel.length as f32);
            let width = self.width_modifier + tunnel.thickness;
            let (cos_pitch, sin_pitch) = (cos(tunnel.pitch), sin(tunnel.pitch));
            at[0] += cos(tunnel.yaw) * cos_pitch;
            at[1] += sin_pitch;
            at[2] += sin(tunnel.yaw) * cos_pitch;
            let decayed_pitch = tunnel.pitch * pitch_decay;
            let decayed_pitch_change = PITCH_CHANGE_DECAY * pitch_change;
            let decayed_yaw_change = YAW_CHANGE_DECAY * yaw_change;
            let [pitch_a, pitch_b, pitch_scale, yaw_a, yaw_b, yaw_scale] = std::array::from_fn(|_| random.next_float());
            let radius = sine * width + TUNNEL_RADIUS_BASE;
            tunnel.pitch = pitch_change * PITCH_CHANGE_GAIN + decayed_pitch;
            tunnel.yaw += yaw_change * YAW_CHANGE_GAIN;
            let yaw_delta = yaw_scale * (yaw_a - yaw_b) * YAW_DELTA_SCALE;
            if tunnel.thickness > BRANCH_THICKNESS_LIMIT && branch_step == tunnel.step {
                for side in [-FRAC_PI_2, FRAC_PI_2] {
                    let thickness = random.next_float() * BRANCH_THICKNESS_SCALE + BRANCH_THICKNESS_BASE;
                    let branch = Tunnel {
                        thickness,
                        yaw: tunnel.yaw + side,
                        pitch: tunnel.pitch / BRANCH_PITCH_DIVISOR,
                        step: branch_step,
                        length: tunnel.length,
                    };
                    self.tunnel(&mut random, at, branch, cave, out);
                }
                return;
            }
            if random.next_int_bounded(self.skip_segment_chance) != 0 {
                out.push(Segment {
                    center: at,
                    horizontal_radius: radius,
                    vertical_radius: radius,
                    floor_level: cave.floor_level,
                    step: tunnel.step,
                    length: tunnel.length,
                    thickness: tunnel.thickness,
                });
            }
            if tunnel.step + 1 >= tunnel.length {
                return;
            }
            let pitch_delta = pitch_scale * (pitch_a - pitch_b);
            pitch_change = decayed_pitch_change + (pitch_delta + pitch_delta);
            yaw_change = decayed_yaw_change + yaw_delta;
            tunnel.step += 1;
        }
    }
}
