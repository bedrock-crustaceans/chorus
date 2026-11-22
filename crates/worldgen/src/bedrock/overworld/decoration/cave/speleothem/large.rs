use super::super::super::sine;
use super::super::super::world::read;
use super::super::CaveDraw;
use super::super::support::{is_lava, is_open_or_wet};
use super::Family;
use super::cluster::{Column, scan};
use crate::shared::blocks::BlockId;
use crate::shared::chunk::region::{Heightmap, Region};
use glam::IVec3;

const SEARCH_RANGE: i32 = 30;
const RADIUS: [i32; 2] = [3, 18];
const RADIUS_TO_HEIGHT: f32 = 0.33;
const STALACTITE_BLUNTNESS: [f32; 2] = [0.3, 0.9];
const STALAGMITE_BLUNTNESS: [f32; 2] = [0.4, 1.0];
const HEIGHT_SCALE: [f32; 2] = [0.4, 2.0];
const WIND_SPEED: [f32; 2] = [0.0, 0.3];
const MIN_RADIUS_FOR_WIND: i32 = 4;
const MIN_BLUNTNESS_FOR_WIND: f32 = 0.6;
const MIN_ROOM: i32 = 5;
const MOVE_BACK_LIMIT: i32 = 10;
const SHORTEN_CHANCE: f32 = 0.2;
const SHORTEN_SPAN: f32 = 0.199_999_99;
const SHORTEN_MIN: f32 = 0.8;
const SHAPE_EXPONENT: f64 = 0.384;
const SHAPE_HIGH: f64 = 1.333_333_333_333_333_3;
const SHAPE_LOW: f64 = 0.666_666_666_666_666_6;
const SHAPE_LOG: f64 = -0.333_333_333_333_333_3;
const FULL_TURN: f32 = 6.283_185_5;
const CIRCLE_STEP: f32 = 6.0;

fn uniform<D: CaveDraw>(random: &mut D, [min, max]: [f32; 2]) -> f32 {
    (max - min) * random.float() + min
}

fn dripstone_height(distance: f32, radius: i32, scale: f32, bluntness: f32) -> f32 {
    let ratio = (bluntness.max(distance) / radius as f32) as f64 * SHAPE_EXPONENT;
    let shape = ratio.powf(SHAPE_HIGH) * 0.75 - ratio.powf(SHAPE_LOW) + ratio.ln() * SHAPE_LOG;
    ((shape * scale as f64).max(0.0) / SHAPE_EXPONENT * radius as f64) as f32
}

fn open_to_fluid(region: &Region, block: BlockId) -> bool {
    is_open_or_wet(region.blocks, block) || is_lava(region.blocks, block)
}

#[derive(Clone, Copy)]
struct Wind(Option<(i32, f32, f32)>);

impl Wind {
    fn at(self, pos: IVec3) -> IVec3 {
        match self.0 {
            Some((origin, x, z)) => {
                let drop = (origin - pos.y) as f32;
                IVec3::new(pos.x + (drop * x) as i32, pos.y, pos.z + (drop * z) as i32)
            }
            None => pos,
        }
    }
}

struct Large {
    root: IVec3,
    up: bool,
    radius: i32,
    bluntness: f32,
    scale: f32,
}

impl Large {
    fn height(&self) -> i32 {
        dripstone_height(0.0, self.radius, self.scale, self.bluntness) as i32
    }

    fn embedded(region: &Region, pos: IVec3, radius: i32) -> bool {
        if open_to_fluid(region, read(region, pos)) {
            return false;
        }
        let radius = radius as f32;
        let mut angle = 0.0f32;
        while angle < FULL_TURN {
            let offset = IVec3::new((sine::cos(angle) * radius) as i32, 0, (sine::sin(angle) * radius) as i32);
            if open_to_fluid(region, read(region, pos + offset)) {
                return false;
            }
            angle += CIRCLE_STEP / radius;
        }
        true
    }

    fn move_back(&mut self, region: &Region, wind: Wind) -> bool {
        while self.radius > 1 {
            let steps = self.height().min(MOVE_BACK_LIMIT);
            for index in 0..steps {
                let pos = self.root + IVec3::Y * if self.up { -index } else { index };
                if is_lava(region.blocks, read(region, pos)) {
                    return false;
                }
                if Self::embedded(region, wind.at(pos), self.radius) {
                    self.root = pos;
                    return true;
                }
            }
            self.radius /= 2;
        }
        false
    }

    fn build<D: CaveDraw>(&self, family: &Family, region: &mut Region, random: &mut D, wind: Wind) {
        let radius = self.radius;
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                let distance = ((dx * dx + dz * dz) as f32).sqrt();
                if distance > radius as f32 {
                    continue;
                }
                let mut height = dripstone_height(distance, radius, self.scale, self.bluntness) as i32;
                if height <= 0 {
                    continue;
                }
                if random.float() < SHORTEN_CHANCE {
                    height = ((random.float() * SHORTEN_SPAN + SHORTEN_MIN) * height as f32) as i32;
                }
                let (x, z) = (self.root.x + dx, self.root.z + dz);
                let top = if self.up { region.height(Heightmap::WorldSurfaceWg, x, z) } else { i32::MAX };
                let mut y = self.root.y;
                let mut placed = false;
                for _ in 0..height {
                    if y >= top {
                        break;
                    }
                    let pos = wind.at(IVec3::new(x, y, z));
                    let block = read(region, pos);
                    if open_to_fluid(region, block) || family.is_spike(region, block) {
                        region.set(pos, family.base);
                        placed = true;
                    } else if placed {
                        break;
                    }
                    y += if self.up { 1 } else { -1 };
                }
            }
        }
    }
}

pub fn place<D: CaveDraw>(family: &Family, region: &mut Region, random: &mut D, at: IVec3) -> bool {
    if !is_open_or_wet(region.blocks, read(region, at)) {
        return false;
    }
    let blocks = region.blocks;
    let edge = |block: BlockId| family.replaceable.contains(blocks, block) || block == family.base || is_lava(blocks, block);
    let Some(Column {
        floor: Some(floor),
        ceiling: Some(ceiling),
    }) = scan(region, at, SEARCH_RANGE, |block| is_open_or_wet(blocks, block), edge)
    else {
        return false;
    };
    if ceiling - floor < MIN_ROOM {
        return false;
    }
    let cap = (((ceiling - floor - 1) as f32 * RADIUS_TO_HEIGHT) as i32).clamp(RADIUS[0], RADIUS[1]);
    let radius = RADIUS[0] + random.below(cap - RADIUS[0] + 1);
    let (blunt_up, scale_up) = (uniform(random, STALACTITE_BLUNTNESS), uniform(random, HEIGHT_SCALE));
    let (blunt_down, scale_down) = (uniform(random, STALAGMITE_BLUNTNESS), uniform(random, HEIGHT_SCALE));
    let mut stalactite = Large {
        root: IVec3::new(at.x, ceiling - 1, at.z),
        up: false,
        radius,
        bluntness: blunt_up,
        scale: scale_up,
    };
    let mut stalagmite = Large {
        root: IVec3::new(at.x, floor + 1, at.z),
        up: true,
        radius,
        bluntness: blunt_down,
        scale: scale_down,
    };
    let mut wind = Wind(None);
    if radius >= MIN_RADIUS_FOR_WIND && blunt_up >= MIN_BLUNTNESS_FOR_WIND && blunt_down >= MIN_BLUNTNESS_FOR_WIND {
        let speed = uniform(random, WIND_SPEED);
        let angle = random.float() * std::f32::consts::PI + 0.0;
        wind = Wind(Some((at.y, sine::cos(angle) * speed, sine::sin(angle) * speed)));
    }
    let hangs = stalactite.move_back(region, wind);
    let stands = stalagmite.move_back(region, wind);
    let (min, max) = (region.min_y(), region.max_y() + 1);
    let tip = |large: &Large| dripstone_height(0.0, large.radius, large.scale, large.bluntness);
    let mut placed = false;
    if hangs && ((stalactite.root.y as f32 - tip(&stalactite)) as i32) > min {
        stalactite.build(family, region, random, wind);
        placed = true;
    }
    if stands && ((stalagmite.root.y as f32 + tip(&stalagmite)) as i32) < max {
        stalagmite.build(family, region, random, wind);
        placed = true;
    }
    placed
}
