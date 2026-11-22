use super::spikes::{COUNT, Spike};
use super::{AIR, BlockId, CHUNK_MASK, EndBlocks, EndChunk};
use crate::shared::decoration_window::{SLOTS, Window, Writes};
use crate::shared::rng::mersenne::MersenneTwister;

const CHORUS_REACH: u32 = 8;
const CHORUS_DEPTH: i32 = 4;
const CHORUS_SIDES: [[i32; 2]; 4] = [[0, -1], [1, 0], [0, 1], [-1, 0]];
const GATEWAY_WIDTH: i32 = 1;
const GATEWAY_HEIGHT: i32 = 3;
const GATEWAY_HEIGHT_RANGE: u32 = 7;
const GATEWAY_CHANCE: u32 = 700;
const ISLAND_CHANCE: u32 = 14;
const ISLAND_TWIN_CHANCE: u32 = 3;
const ISLAND_COUNT: u32 = 5;
const ISLAND_BASE_Y: i32 = 55;
const ISLAND_OFFSET: i32 = 8;
const ORIGIN_DISTANCE: i64 = 4096;
const LOW_ISLAND: f32 = -20.0;
const HIGH_ISLAND: f32 = 40.0;
const SPIKE_CLEAR_FROM: i32 = 65;
const SPIKE_CLEAR_ABOVE: i32 = 10;
const CAGE: i32 = 2;
const CAGE_TOP: i32 = 3;

pub(super) struct View<'a> {
    window: Window<'a>,
    tops: &'a [[i32; 256]; SLOTS],
}

impl<'a> View<'a> {
    pub(super) fn new(chunks: &'a mut [EndChunk; SLOTS], tops: &'a [[i32; 256]; SLOTS], source: (i32, i32)) -> Self {
        Self {
            window: Window::new(chunks, source),
            tops,
        }
    }

    pub(super) fn get(&self, x: i32, y: i32, z: i32) -> BlockId {
        self.window.get(x, y, z)
    }

    pub(super) fn set(&mut self, x: i32, y: i32, z: i32, block: BlockId) {
        self.window.set(x, y, z, block);
    }

    pub(super) fn heightmap(&self, x: i32, z: i32) -> i32 {
        let Some(slot) = self.window.slot(x, z) else { return 0 };
        self.tops[slot][((z & CHUNK_MASK) * 16 + (x & CHUNK_MASK)) as usize]
    }

    pub(super) fn take_writes(&mut self) -> Writes {
        self.window.take_writes()
    }
}

pub(super) fn decorate(random: &mut MersenneTwister, source: (i32, i32), view: &mut View, spikes: &[Spike; COUNT], ids: &EndBlocks, island: impl Fn(i32, i32) -> f32) {
    let (origin_x, origin_z) = (source.0 * 16, source.1 * 16);
    for spike in spikes
        .iter()
        .filter(|spike| (spike.x - spike.radius) & !CHUNK_MASK == origin_x && (spike.z - spike.radius) & !CHUNK_MASK == origin_z)
    {
        place_spike(view, spike, ids);
    }
    if i64::from(source.0) * i64::from(source.0) + i64::from(source.1) * i64::from(source.1) <= ORIGIN_DISTANCE {
        return;
    }
    let height = island(source.0, source.1);
    if height < LOW_ISLAND && random.next_u32().is_multiple_of(ISLAND_CHANCE) {
        let x = (random.next_u32() & CHUNK_MASK as u32) as i32 + origin_x + ISLAND_OFFSET;
        let y = (random.next_u32() & CHUNK_MASK as u32) as i32 + ISLAND_BASE_Y;
        let z = (random.next_u32() & CHUNK_MASK as u32) as i32 + origin_z + ISLAND_OFFSET;
        end_island(view, random, [x, y, z], ids);
        if random.next_u32() & ISLAND_TWIN_CHANCE == 0 {
            end_island(view, random, [x, y, z], ids);
        }
    }
    if height <= HIGH_ISLAND {
        return;
    }
    let count = random.next_u32() % ISLAND_COUNT;
    let (origin_x, origin_z) = (origin_x | 8, origin_z | 8);
    let mut placed = 0;
    let gateway_draw = loop {
        let x_draw = random.next_u32();
        if placed >= count {
            break x_draw;
        }
        let z_draw = random.next_u32();
        let (x, z) = ((x_draw & CHUNK_MASK as u32) as i32 + origin_x, (z_draw & CHUNK_MASK as u32) as i32 + origin_z);
        let top = view.heightmap(x, z);
        if top > 0 && view.get(x, top, z) == AIR && view.get(x, top - 1, z) == ids.end_stone {
            view.set(x, top, z, ids.chorus_plant);
            grow_chorus(view, random, [x, top, z], [x, top, z], CHORUS_REACH, 0, ids);
        }
        placed += 1;
    };
    if gateway_draw.is_multiple_of(GATEWAY_CHANCE) {
        let x = (random.next_u32() & CHUNK_MASK as u32) as i32 + origin_x;
        let z = (random.next_u32() & CHUNK_MASK as u32) as i32 + origin_z;
        let top = view.heightmap(x, z);
        if top > 0 {
            let y = (top + GATEWAY_HEIGHT) as u16 as i32 + (random.next_u32() % GATEWAY_HEIGHT_RANGE) as i32;
            end_gateway(view, [x, y, z], ids);
        }
    }
}

fn place_spike(view: &mut View, spike: &Spike, ids: &EndBlocks) {
    let radius = spike.radius;
    for x in spike.x - radius..=spike.x + radius {
        for y in 0..=spike.height + SPIKE_CLEAR_ABOVE {
            for z in spike.z - radius..=spike.z + radius {
                let distance = (z - spike.z) * (z - spike.z) + (x - spike.x) * (x - spike.x);
                if distance <= radius * radius + 1 && y < spike.height {
                    view.set(x, y, z, ids.obsidian);
                } else if y > SPIKE_CLEAR_FROM {
                    view.set(x, y, z, AIR);
                }
            }
        }
    }
    if spike.guarded {
        for dx in -CAGE..=CAGE {
            for dz in -CAGE..=CAGE {
                for dy in 0..=CAGE_TOP {
                    if dx.abs() == CAGE || dz.abs() == CAGE || dy == CAGE_TOP {
                        view.set(spike.x + dx, spike.height + dy, spike.z + dz, ids.iron_bars);
                    }
                }
            }
        }
    }
}

fn end_island(view: &mut View, random: &mut MersenneTwister, [x, y, z]: [i32; 3], ids: &EndBlocks) {
    let mut radius = random.next_int_bounded(3) as f32 + 4.0;
    let mut drop = 0;
    while radius > 0.5 {
        let (low, high) = ((-radius).floor() as i32, radius.ceil() as i32);
        let limit = (radius + 1.0) * (radius + 1.0);
        for i in low..=high {
            for k in low..=high {
                if (i * i + k * k) as f32 <= limit {
                    view.set(x + i, y + drop, z + k, ids.end_stone);
                }
            }
        }
        radius -= random.next_int_bounded(2) as f32 + 0.5;
        drop -= 1;
    }
}

fn sides_empty(view: &View, [x, y, z]: [i32; 3], except: Option<usize>) -> bool {
    CHORUS_SIDES
        .iter()
        .enumerate()
        .filter(|&(index, _)| Some(index) != except)
        .all(|(_, [dx, dz])| view.get(x + dx, y, z + dz) == AIR)
}

fn grow_chorus(view: &mut View, random: &mut MersenneTwister, at: [i32; 3], root: [i32; 3], reach: u32, depth: i32, ids: &EndBlocks) {
    let first = i32::from(depth == 0);
    let stem = random.next_int_bounded(4) + 1 + first;
    for step in 1..=stem {
        let position = [at[0], at[1] + step, at[2]];
        if !sides_empty(view, position, None) {
            return;
        }
        view.set(position[0], position[1], position[2], ids.chorus_plant);
    }
    let top = [at[0], at[1] + stem, at[2]];
    let mut branched = false;
    if depth < CHORUS_DEPTH {
        let branches = random.next_int_bounded(4) + first;
        for _ in 0..branches {
            let side = random.next_int_bounded(4) as usize;
            let [dx, dz] = CHORUS_SIDES[side];
            let branch = [top[0] + dx, top[1], top[2] + dz];
            let near = branch[0].abs_diff(root[0]) < reach && branch[2].abs_diff(root[2]) < reach;
            if near && view.get(branch[0], branch[1], branch[2]) == AIR && view.get(branch[0], branch[1] - 1, branch[2]) == AIR && sides_empty(view, branch, Some((side + 2) % 4)) {
                branched = true;
                view.set(branch[0], branch[1], branch[2], ids.chorus_plant);
                grow_chorus(view, random, branch, root, reach, depth + 1, ids);
            }
        }
    }
    if !branched {
        view.set(top[0], top[1], top[2], ids.chorus_flower);
    }
}

fn end_gateway(view: &mut View, [x, y, z]: [i32; 3], ids: &EndBlocks) {
    for dx in -GATEWAY_WIDTH..=GATEWAY_WIDTH {
        for dy in -2i32..=2 {
            for dz in -GATEWAY_WIDTH..=GATEWAY_WIDTH {
                let centre = dx == 0 && dz == 0;
                let block = match dy.abs() {
                    0 if centre => ids.end_gateway,
                    2 if centre => ids.bedrock,
                    1 if dx == 0 || dz == 0 => ids.bedrock,
                    _ => AIR,
                };
                view.set(x + dx, y + dy, z + dz, block);
            }
        }
    }
}
