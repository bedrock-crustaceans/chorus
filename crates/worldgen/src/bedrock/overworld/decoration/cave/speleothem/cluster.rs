use super::super::super::world::read;
use super::super::CaveDraw;
use super::super::support::{is_lava, is_open_or_wet, is_wet};
use super::Family;
use crate::shared::blocks::BlockId;
use crate::shared::chunk::region::Region;
use glam::IVec3;

const SEARCH_RANGE: i32 = 12;
const RADIUS_BASE: i32 = 2;
const RADIUS_BOUND: i32 = 6;
const DENSITY_SPAN: f32 = 0.4;
const DENSITY_MIN: f32 = 0.3;
const EDGE_DIVISOR: f32 = 3.0;
const EDGE_SLOPE: f32 = 0.9;
const EDGE_FLOOR: f32 = 0.1;
const LAYER_BOUND: i32 = 2;
const LAYER_BASE: i32 = 2;
const HEIGHT_DEVIATION: f32 = 3.0;
const DISTANCE_FADE: f32 = 0.125;
const MERGE_SLACK: i32 = 3;

pub struct Column {
    pub floor: Option<i32>,
    pub ceiling: Option<i32>,
}

pub fn scan(region: &Region, at: IVec3, range: i32, inside: impl Fn(BlockId) -> bool, edge: impl Fn(BlockId) -> bool) -> Option<Column> {
    if !inside(read(region, at)) {
        return None;
    }
    let way = |dy: i32| {
        let mut y = at.y + dy;
        for _ in 0..range - 1 {
            if !inside(read(region, IVec3::new(at.x, y, at.z))) {
                break;
            }
            y += dy;
        }
        edge(read(region, IVec3::new(at.x, y, at.z))).then_some(y)
    };
    Some(Column { ceiling: way(1), floor: way(-1) })
}

struct Shape {
    height: i32,
    wetness: f32,
    density: f32,
}

pub fn place<D: CaveDraw>(family: &Family, region: &mut Region, random: &mut D, height: [i32; 2], wetness: [f32; 4], at: IVec3) -> bool {
    if !is_open_or_wet(region.blocks, read(region, at)) {
        return false;
    }
    let height = height[0] + random.below(height[1] - height[0] + 1);
    let [mean, deviation, min, max] = wetness;
    let wetness = (mean + deviation * random.gaussian()).max(min).min(max);
    let density = random.float() * DENSITY_SPAN + DENSITY_MIN;
    let radius_x = random.below(RADIUS_BOUND) + RADIUS_BASE;
    let radius_z = random.below(RADIUS_BOUND) + RADIUS_BASE;
    let shape = Shape { height, wetness, density };
    for dx in -radius_x..=radius_x {
        for dz in -radius_z..=radius_z {
            let edge = (radius_x - dx.abs()).min(radius_z - dz.abs()) as f32 / EDGE_DIVISOR;
            let chance = if edge < 0.0 {
                EDGE_FLOOR as f64
            } else if edge > 1.0 {
                1.0
            } else {
                (edge * EDGE_SLOPE + EDGE_FLOOR) as f64
            };
            column(family, region, random, &shape, at + IVec3::new(dx, 0, dz), dx.abs() + dz.abs(), chance);
        }
    }
    true
}

fn layer<D: CaveDraw>(family: &Family, region: &mut Region, random: &mut D, at: IVec3, up: bool) {
    let thickness = random.below(LAYER_BOUND) + LAYER_BASE;
    let step = if up { IVec3::Y } else { IVec3::NEG_Y };
    for index in 0..thickness {
        if !family.replace(region, at + step * index) {
            return;
        }
    }
}

fn spike_height<D: CaveDraw>(random: &mut D, density: f32, distance: i32, room: i32) -> i32 {
    if random.float() > density {
        return 0;
    }
    let (room, fade) = (room as f32, distance as f32 * DISTANCE_FADE);
    let mean = if fade > 1.0 { 0.0 } else { (0.0 - room * 0.5) * fade + room * 0.5 };
    let value = random.gaussian() * HEIGHT_DEVIATION + mean;
    (if room < value { room } else { value.max(0.0) }) as i32
}

fn column<D: CaveDraw>(family: &Family, region: &mut Region, random: &mut D, shape: &Shape, at: IVec3, distance: i32, chance: f64) {
    let blocks = region.blocks;
    let inside = |block: BlockId| is_open_or_wet(blocks, block);
    let edge = |block: BlockId| {
        let base = family.replaceable.contains(blocks, block) || block == family.base;
        base || is_lava(blocks, block)
    };
    let Some(Column { mut floor, ceiling }) = scan(region, at, SEARCH_RANGE, inside, edge) else {
        return;
    };
    if floor.is_none() && ceiling.is_none() {
        return;
    }
    if random.float() < shape.wetness
        && let Some(found) = floor
        && can_pool(family, region, IVec3::new(at.x, found, at.z))
    {
        let water = region.water();
        region.set(IVec3::new(at.x, found, at.z), water);
        floor = Some(found - 1);
    }
    let mut stalactite = 0;
    if random.double() < chance
        && let Some(top) = ceiling
        && !is_lava(region.blocks, read(region, IVec3::new(at.x, top, at.z)))
    {
        layer(family, region, random, IVec3::new(at.x, top, at.z), true);
        let room = floor.map_or(shape.height, |bottom| shape.height.min(top - bottom));
        stalactite = spike_height(random, shape.density, distance, room);
    }
    let mut stalagmite = 0;
    if random.double() < chance
        && let Some(bottom) = floor
        && !is_lava(region.blocks, read(region, IVec3::new(at.x, bottom, at.z)))
    {
        layer(family, region, random, IVec3::new(at.x, bottom, at.z), false);
        stalagmite = if ceiling.is_some() {
            (stalactite + random.below(MERGE_SLACK) - 1).max(0)
        } else {
            spike_height(random, shape.density, distance, shape.height)
        };
    }
    if let (Some(top), Some(bottom)) = (ceiling, floor)
        && top - stalactite <= bottom + stalagmite
    {
        let low = (top - stalactite).max(bottom + 1);
        let high = (bottom + stalagmite).min(top - 1);
        let meet = if low <= high + 1 { low + random.below(high - low + 2) } else { low };
        stalagmite = meet - bottom - 1;
        stalactite = top - meet;
    }
    let joined = random.boolean();
    let merge = match (ceiling, floor) {
        (Some(top), Some(bottom)) => joined && stalactite > 0 && stalagmite > 0 && stalactite + stalagmite == top - bottom - 1,
        _ => false,
    };
    if let Some(top) = ceiling {
        family.grow(region, IVec3::new(at.x, top - 1, at.z), false, stalactite, merge);
    }
    if let Some(bottom) = floor {
        family.grow(region, IVec3::new(at.x, bottom + 1, at.z), true, stalagmite, merge);
    }
}

fn can_pool(family: &Family, region: &Region, pos: IVec3) -> bool {
    let blocks = region.blocks;
    let block = read(region, pos);
    if block == family.base || is_wet(blocks, block) || family.is_spike(region, block) || is_wet(blocks, read(region, pos + IVec3::Y)) {
        return false;
    }
    let holds = |block: BlockId| matches!(blocks.name_of(block), chorus_block::block_id::STONE | chorus_block::block_id::DEEPSLATE | chorus_block::block_id::TUFF) || is_wet(blocks, block);
    [IVec3::NEG_Z, IVec3::X, IVec3::Z, IVec3::NEG_X, IVec3::NEG_Y].iter().all(|&offset| holds(read(region, pos + offset)))
}
