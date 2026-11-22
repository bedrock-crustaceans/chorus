use std::f32::consts::{FRAC_PI_2, PI};

use super::{NetherBlocks, NetherChunk, TERRAIN_HEIGHT};
use crate::bedrock::overworld::decoration_seed;
use crate::shared::CHUNK_WIDTH;
use crate::shared::math::sine;
use crate::shared::rng::mersenne::MersenneTwister;

const RANGE: i32 = 8;
const CAVE_BOUND: i32 = 10;
const SKIP_CARVE_CHANCE: i32 = 5;
const ROOM_CHANCE: i32 = 4;
const EXTRA_TUNNELS: i32 = 4;
const STEEP_CHANCE: u32 = 6;
const SKIP_SEGMENT_CHANCE: u32 = 4;
const FLOOR_LEVEL: f32 = -0.7;
const MIN_Y: i32 = 2;
const MAX_Y: i32 = 119;
const SOURCE_CHUNKS: i32 = RANGE;
const LENGTH_BASE: i32 = 112;
const LENGTH_SPREAD: u32 = 28;
const ROOM_THICKNESS: f32 = 6.0;
const ROOM_THICKNESS_BASE: f32 = 1.0;
const ROOM_Y_SCALE: f32 = 0.5;
const TUNNEL_Y_SCALE: f32 = 0.5;
const PITCH_SCALE: f32 = 0.125;
const THICKNESS_SCALE: f32 = 0.5;
const REACH_EXTRA: f32 = 16.0;
const ROOM_BRANCH_Y_SCALE: f32 = 1.0;
const BRANCH_PITCH_DIVISOR: f32 = 3.0;
const RIM_GROWTH: i32 = 2;

pub struct NetherCarver {
    seed: u32,
}

#[derive(Clone, Copy)]
struct Tunnel {
    thickness: f32,
    yaw: f32,
    pitch: f32,
    step: i32,
    length: i32,
    y_scale: f32,
}

struct Target<'a> {
    chunk: &'a mut NetherChunk,
    position: (i32, i32),
    ids: &'a NetherBlocks,
}

impl NetherCarver {
    pub fn new(seed: i64) -> Self {
        Self { seed: seed as u32 }
    }

    pub fn carve(&self, chunk: &mut NetherChunk, chunk_x: i32, chunk_z: i32, ids: &NetherBlocks) {
        let mut target = Target {
            chunk,
            position: (chunk_x, chunk_z),
            ids,
        };
        for x in chunk_x - SOURCE_CHUNKS..=chunk_x + SOURCE_CHUNKS {
            for z in chunk_z - SOURCE_CHUNKS..=chunk_z + SOURCE_CHUNKS {
                let mut random = MersenneTwister::new(decoration_seed(self.seed, x, z));
                add_feature(&mut random, (x, z), &mut target);
            }
        }
    }
}

fn add_feature(random: &mut MersenneTwister, source: (i32, i32), target: &mut Target) {
    let bound = random.next_int_bounded(CAVE_BOUND);
    let bound = random.next_int_bounded(bound + 1);
    let count = random.next_int_bounded(bound + 1);
    if random.next_int_bounded(SKIP_CARVE_CHANCE) != 0 || count <= 0 {
        return;
    }
    for _ in 0..count {
        let z = random.next_int_bounded(CHUNK_WIDTH) + source.1 * CHUNK_WIDTH;
        let y = random.next_int_bounded(TERRAIN_HEIGHT);
        let x = random.next_int_bounded(CHUNK_WIDTH) + source.0 * CHUNK_WIDTH;
        let origin = [x as f32, y as f32, z as f32];
        let mut tunnels = 1;
        if random.next_int_bounded(ROOM_CHANCE) == 0 {
            let room = Tunnel {
                thickness: random.next_float() * ROOM_THICKNESS + ROOM_THICKNESS_BASE,
                yaw: 0.0,
                pitch: 0.0,
                step: -1,
                length: -1,
                y_scale: ROOM_Y_SCALE,
            };
            add_tunnel(random, origin, room, target);
            tunnels = random.next_int_bounded(EXTRA_TUNNELS) + 1;
        }
        for _ in 0..tunnels {
            let yaw = random.next_float() * PI;
            let pitch = random.next_float() + -0.5;
            let first = random.next_float();
            let thickness = first + first + random.next_float();
            let tunnel = Tunnel {
                thickness: thickness + thickness,
                yaw: yaw + yaw,
                pitch: (pitch + pitch) * PITCH_SCALE,
                step: 0,
                length: 0,
                y_scale: TUNNEL_Y_SCALE,
            };
            add_tunnel(random, origin, tunnel, target);
        }
    }
}

fn add_tunnel(parent: &mut MersenneTwister, mut at: [f32; 3], mut tunnel: Tunnel, target: &mut Target) {
    let mut random = MersenneTwister::new(parent.next_int() as u32);
    if tunnel.length <= 0 {
        tunnel.length = LENGTH_BASE - (random.next_u32() % LENGTH_SPREAD) as i32;
    }
    let room = tunnel.step == -1;
    if room {
        tunnel.step = tunnel.length >> 1;
    }
    let mut branch = if tunnel.length != 1 { (random.next_u32() % (tunnel.length as u32 >> 1)) as i32 } else { 0 };
    let decay = if !random.next_u32().is_multiple_of(STEEP_CHANCE) { 0.7 } else { 0.92 };
    if tunnel.step >= tunnel.length {
        return;
    }
    branch += tunnel.length >> 2;
    let reach = (2.0 + tunnel.thickness) + REACH_EXTRA;
    let reach = reach * reach;
    let (chunk_x, chunk_z) = target.position;
    let centre = [((chunk_x * 16) | 8) as f32, ((chunk_z * 16) | 8) as f32];
    let (mut pitch_change, mut yaw_change) = (0.0f32, 0.0f32);
    while tunnel.step < tunnel.length {
        let sine = sine::sin(tunnel.step as f32 * PI / tunnel.length as f32);
        let radius = sine * (0.0 + tunnel.thickness) + 1.5;
        let (cos_pitch, sin_pitch) = (sine::cos(tunnel.pitch), sine::sin(tunnel.pitch));
        at[0] += sine::cos(tunnel.yaw) * cos_pitch;
        at[1] += sin_pitch;
        at[2] += cos_pitch * sine::sin(tunnel.yaw);
        tunnel.pitch = pitch_change * 0.1 + tunnel.pitch * decay;
        tunnel.yaw += yaw_change * 0.1;
        let (decayed_pitch_change, decayed_yaw_change) = (pitch_change * 0.9, yaw_change * 0.75);
        let draws: [f32; 6] = std::array::from_fn(|_| random.next_float());
        let pitch_delta = draws[2] * (draws[0] - draws[1]);
        pitch_change = pitch_delta + pitch_delta + decayed_pitch_change;
        yaw_change = draws[5] * (draws[3] - draws[4]) * 4.0 + decayed_yaw_change;
        if !room {
            if tunnel.thickness > 1.0 && tunnel.step == branch {
                for side in [-FRAC_PI_2, FRAC_PI_2] {
                    let thickness = random.next_float() * THICKNESS_SCALE + THICKNESS_SCALE;
                    let child = Tunnel {
                        thickness,
                        yaw: side + tunnel.yaw,
                        pitch: tunnel.pitch / BRANCH_PITCH_DIVISOR,
                        step: branch,
                        length: tunnel.length,
                        y_scale: ROOM_BRANCH_Y_SCALE,
                    };
                    add_tunnel(&mut random, at, child, target);
                }
                return;
            }
            if random.next_u32().is_multiple_of(SKIP_SEGMENT_CHANCE) {
                tunnel.step += 1;
                continue;
            }
        }
        let (dx, dz) = (at[0] - centre[0], at[2] - centre[1]);
        let left = (tunnel.length - tunnel.step) as f32;
        if dz * dz + dx * dx - left * left > reach {
            return;
        }
        let width = radius + radius;
        let outside = (centre[0] - 16.0) - width > at[0] || (centre[1] - 16.0) - width > at[2] || at[0] > (centre[0] + 16.0) + width || at[2] > width + (centre[1] + 16.0);
        if !outside && carve(target, at, radius, radius * tunnel.y_scale) && room {
            return;
        }
        tunnel.step += 1;
    }
}

fn floor(value: f32) -> i32 {
    let truncated = value as i32;
    truncated - i32::from(truncated as f32 > value)
}

fn carve(target: &mut Target, [x, y, z]: [f32; 3], radius: f32, vertical: f32) -> bool {
    let (chunk_x, chunk_z) = (target.position.0 * 16, target.position.1 * 16);
    let x0 = (floor(x - radius) - chunk_x - 1).max(0);
    let x1 = (floor(radius + x) - chunk_x).min(15);
    let z0 = (floor(z - radius) - chunk_z - 1).max(0);
    let z1 = (floor(radius + z) - chunk_z).min(15);
    let y0 = floor(y - vertical).max(MIN_Y);
    let y1 = floor(vertical + y).min(MAX_Y);
    if x0 > x1 {
        return true;
    }
    if lava_near(target, [x0, x1], [z0, z1], [y0, y1]) {
        return false;
    }
    for local_x in x0..=x1 {
        let dx = ((chunk_x + local_x) as f32 + 0.5 - x) / radius;
        let dx = dx * dx;
        for local_z in z0..=z1 {
            if y1 < y0 - 1 {
                continue;
            }
            let dz = ((chunk_z + local_z) as f32 + 0.5 - z) / radius;
            let dz = dz * dz;
            for cell in (y0 - 1..=y1).rev() {
                let dy = (cell as f32 + 0.5 - y) / vertical;
                if dy * dy + dx + dz < 1.0 && FLOOR_LEVEL < dy {
                    let block = target.chunk.get(local_x, cell + 1, local_z);
                    if block == target.ids.netherrack || block == target.ids.dirt || block == target.ids.coarse_dirt || block == target.ids.grass_block {
                        target.chunk.set(local_x, cell + 1, local_z, target.ids.air);
                    }
                }
            }
        }
    }
    true
}

fn lava_near(target: &Target, [x0, x1]: [i32; 2], [z0, z1]: [i32; 2], [y0, y1]: [i32; 2]) -> bool {
    for x in x0..=x1 {
        for z in z0..=z1 {
            let rim = x == x0 || x == x1 || z == z0 || z == z1;
            let mut y = y1 + RIM_GROWTH;
            while y >= y0 - RIM_GROWTH {
                if target.chunk.contains_y(y) && target.chunk.get(x, y, z) == target.ids.lava {
                    return true;
                }
                y = if rim || y == y0 - RIM_GROWTH { y - 1 } else { y0 - RIM_GROWTH };
            }
        }
    }
    false
}
