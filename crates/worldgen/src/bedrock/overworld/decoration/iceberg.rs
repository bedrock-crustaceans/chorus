use super::aquatic::is_water;
use super::world::read;
use crate::shared::blocks::{AIR, BlockId, BlockTable};
use crate::shared::chunk::region::Region;
use crate::shared::math::sine::{cos, sin};
use crate::shared::rng::mersenne::MersenneTwister;
use chorus_block::block_id::{BLUE_ICE, ICE, PACKED_ICE, SNOW, SNOW_LAYER, WATER};
use glam::IVec3;
use std::f32::consts::{FRAC_PI_2, PI};

const BERG_ONE_IN: i32 = 8;
const BERG_OFFSET: i32 = 12;
const BERG_SPREAD: i32 = 8;
const PATCH_TRIES: i32 = 20;
const PATCH_OFFSET: i32 = 8;
const PATCH_SPREAD: i32 = 16;
const PATCH_DEPTH_BELOW_SEA: i32 = 33;
const PATCH_Y_SPREAD: i32 = 32;
const PATCH_GROWTH_TRIES: i32 = 200;
const SIDES: [[i32; 3]; 6] = [[0, -1, 0], [0, 1, 0], [0, 0, -1], [0, 0, 1], [-1, 0, 0], [1, 0, 0]];
const HORIZONTAL: [[i32; 2]; 4] = [[0, 1], [-1, 0], [0, -1], [1, 0]];
const LONG_RADIUS_MAX: i32 = 11;
const LONG_RADIUS_SPREAD: i32 = 5;
const SHORT_RADIUS_BASE: i32 = 3;
const SHORT_RADIUS_SPREAD: i32 = 3;
const TALL_HEIGHT_BASE: i32 = 6;
const TALL_HEIGHT_SPREAD: i32 = 6;
const SHORT_HEIGHT_BASE: i32 = 3;
const SHORT_HEIGHT_SPREAD: i32 = 15;
const TALL_HEIGHT_CHANCE: f32 = 0.9;
const SPIRE_HEIGHT_BASE: i32 = 10;
const SPIRE_HEIGHT_SPREAD: i32 = 19;
const ELLIPSE_CHANCE: f32 = 0.8;
const SNOW_CHANCE: f32 = 0.7;
const BLUE_BODY_CHANCE: f32 = 0.01;
const GROWTH_SPREAD: i32 = 7;
const RADIUS_SPREAD: i32 = 5;
const RADIUS_CAP: i32 = 11;
const DEEP_SPREAD: i32 = 11;
const DEEP_CAP: i32 = 18;
const STEEP_SQUASH: f32 = 8.0;
const ROUNDNESS_KNEE: f32 = 0.8;
const ROUNDNESS_HIGH: f32 = 8.0;
const ROUNDNESS_SCALE: f32 = 10.0;
const ROUNDNESS_FLOOR: f32 = 0.2;
const RAGGED_EDGE_BASE: f32 = -6.0;
const RAGGED_EDGE_SPREAD: i32 = 3;
const SMOOTH_EDGE: f32 = -0.5;
const EDGE_SKIP_CHANCE: f32 = 0.9;
const ELLIPSE_KEEP_CHANCE: f32 = 0.05;
const ELLIPSE_DIVISOR: i32 = 3;
const ROUND_DIVISOR: i32 = 2;
const SNOW_BODY_SHARE: f32 = 0.6;
const CUT_CHANCE: f32 = 0.7;
const ELLIPSE_CUT_CHANCE: f32 = 0.1;
const HOLE_MIN_SPAN: i32 = 3;
const CARVE_SHORT_CAP: i32 = 6;
const OPEN_SIDES_LIMIT: usize = 2;
const PATCH_DROP_BASE: i32 = 5;
const PATCH_DROP_SPREAD: i32 = 6;
const PATCH_GROW_BASE: i32 = 3;
const PATCH_GROW_LIMIT: i32 = 3;
const PATCH_GROW_TALL: i32 = 2;
const PATCH_NEIGHBOURS_MIN: usize = 1;
const PATCH_NEIGHBOURS_MAX: usize = 4;

fn ceil_to(value: f32) -> i32 {
    let truncated = value as i32;
    truncated + ((truncated as f32) < value) as i32
}

struct Layer {
    left: i32,
    height: i32,
    radius: i32,
    reach: i32,
}

struct Cut {
    angle: f32,
    centre: [i32; 2],
}

struct Slice {
    radius: i32,
    y: i32,
    water: bool,
}

struct Berg {
    long_radius: i32,
    short_radius: i32,
    ellipse: bool,
    span: i32,
    height: i32,
    snowy: bool,
    angle: f32,
    radius: i32,
    body: BlockId,
}

#[derive(Debug)]
pub struct IcebergFeature {
    air: BlockId,
    water: BlockId,
    packed_ice: BlockId,
    blue_ice: BlockId,
    ice: BlockId,
    snow: BlockId,
    snow_layer: BlockId,
}

impl IcebergFeature {
    pub fn new(table: &mut BlockTable) -> Self {
        Self {
            air: AIR,
            water: table.name(WATER),
            packed_ice: table.name(PACKED_ICE),
            blue_ice: table.name(BLUE_ICE),
            ice: table.name(ICE),
            snow: table.name(SNOW),
            snow_layer: table.name(SNOW_LAYER),
        }
    }

    fn berg(&self, random: &mut MersenneTwister) -> Berg {
        let long_radius = LONG_RADIUS_MAX - random.next_int_bounded(LONG_RADIUS_SPREAD);
        let short_radius = random.next_int_bounded(SHORT_RADIUS_SPREAD) + SHORT_RADIUS_BASE;
        let tall = random.next_int_bounded(TALL_HEIGHT_SPREAD);
        let low = random.next_int_bounded(SHORT_HEIGHT_SPREAD);
        let ellipse = random.next_float() > ELLIPSE_CHANCE;
        let (span, height) = if ellipse {
            (long_radius, tall + TALL_HEIGHT_BASE)
        } else if random.next_float() <= TALL_HEIGHT_CHANCE {
            (LONG_RADIUS_MAX, low + SHORT_HEIGHT_BASE)
        } else {
            (LONG_RADIUS_MAX, low + random.next_int_bounded(SPIRE_HEIGHT_SPREAD) + SPIRE_HEIGHT_BASE)
        };
        let snowy = random.next_float() > SNOW_CHANCE;
        let turn = random.next_float();
        let angle = (turn + turn) * PI;
        let grow = random.next_int_bounded(GROWTH_SPREAD);
        let radius = (grow + height - random.next_int_bounded(RADIUS_SPREAD)).min(RADIUS_CAP);
        let body = if random.next_float() <= BLUE_BODY_CHANCE { self.blue_ice } else { self.packed_ice };
        Berg {
            long_radius,
            short_radius,
            ellipse,
            span,
            height,
            snowy,
            angle,
            radius,
            body,
        }
    }

    fn is_iceberg_block(&self, block: BlockId) -> bool {
        block == self.packed_ice || block == self.blue_ice || block == self.snow
    }

    fn distance(&self, angle: f32, dx: f32, dz: f32, long: f32, short: f32) -> f32 {
        let (cosine, sine) = (cos(angle), sin(angle));
        let u = (cosine * dx - sine * dz) / long;
        let v = (cosine * dz + sine * dx) / short;
        ((v as f64) * (v as f64) + (u as f64) * (u as f64) + -1.0) as f32
    }

    fn round_radius(&self, berg: &Berg, random: &mut MersenneTwister, y: i32, height: i32) -> i32 {
        let scale = (3.5 - random.next_float()) * height as f32;
        let radius = if random.next_int_bounded(5) + 15 < height {
            let below = random.next_int_bounded(6) + 3;
            let level = if y < below { y >> 1 } else { y };
            (level as f32 / (scale * -0.4) + 1.0) * berg.radius as f32
        } else {
            let level = y as f32;
            (1.0 - (level * level) / scale) * berg.radius as f32
        };
        ceil_to(radius * 0.5)
    }

    fn steep_radius(&self, berg: &Berg, random: &mut MersenneTwister, down: i32, height: i32) -> i32 {
        let factor = random.next_float();
        ceil_to((1.0 - down as f32 / ((factor * 0.5 + 1.0) * height as f32)) * berg.radius as f32 * 0.5)
    }

    fn place_block(&self, region: &mut Region, random: &mut MersenneTwister, origin: IVec3, offset: IVec3, berg: &Berg, layer: &Layer) {
        let (p, q) = (offset.x, offset.z);
        let (distance, ragged) = if berg.ellipse {
            (self.distance(berg.angle, p as f32, q as f32, layer.reach as f32, berg.short_radius as f32), false)
        } else {
            let factor = random.next_float();
            let roundness = if factor > ROUNDNESS_KNEE { ROUNDNESS_HIGH } else { factor.max(ROUNDNESS_FLOOR) * ROUNDNESS_SCALE };
            let square = ((p * p + q * q) as f64 - (layer.radius as f64) * (layer.radius as f64)) as f32;
            (square + roundness / layer.radius as f32, true)
        };
        if distance >= 0.0 || distance.is_nan() {
            return;
        }
        let pos = origin + offset;
        let threshold = if ragged {
            RAGGED_EDGE_BASE - random.next_int_bounded(RAGGED_EDGE_SPREAD) as f32
        } else {
            SMOOTH_EDGE
        };
        if threshold < distance && random.next_float() > EDGE_SKIP_CHANCE {
            return;
        }
        let here = read(region, pos);
        let water = is_water(region, pos);
        if here != self.air && here != self.snow && here != self.ice && !water {
            return;
        }
        let (divisor, keep) = if berg.ellipse {
            (ELLIPSE_DIVISOR, random.next_float() <= ELLIPSE_KEEP_CHANCE)
        } else {
            (ROUND_DIVISOR, false)
        };
        let mut block = berg.body;
        if berg.snowy && !water {
            let span = layer.height / divisor;
            if layer.left as f32 <= random.next_int_bounded(span + (span == 0) as i32) as f32 + layer.height as f32 * SNOW_BODY_SHARE {
                block = self.snow;
            }
            if keep {
                block = berg.body;
            }
        }
        region.set(pos, block);
    }

    fn raise(&self, region: &mut Region, random: &mut MersenneTwister, origin: IVec3, berg: &Berg) {
        let (span, height) = (berg.span, berg.height);
        if span > 0 && height > 0 {
            for p in -span..span {
                for q in -span..span {
                    for y in 0..height {
                        let radius = if berg.ellipse { span } else { self.round_radius(berg, random, y, height) };
                        if berg.ellipse || p < radius {
                            let layer = Layer {
                                left: height - y,
                                height,
                                radius,
                                reach: span,
                            };
                            self.place_block(region, random, origin, IVec3::new(p, y, q), berg, &layer);
                        }
                    }
                }
            }
        }
        self.smooth(region, origin, berg);
        let depth = random.next_int_bounded(DEEP_SPREAD) + height;
        let deep = depth.min(DEEP_CAP);
        if span > 0 && depth > 1 {
            for p in -span..span {
                for q in -span..span {
                    let (mut y, mut down) = (-1, 1);
                    while y > -deep {
                        let reach = if berg.ellipse {
                            ceil_to(((1.0 - (y * y) as f64 / (deep as f32 * STEEP_SQUASH) as f64) * span as f64) as f32)
                        } else {
                            span
                        };
                        let radius = self.steep_radius(berg, random, down, deep);
                        if p < radius {
                            let layer = Layer {
                                left: deep + down,
                                height: deep,
                                radius,
                                reach,
                            };
                            self.place_block(region, random, origin, IVec3::new(p, y, q), berg, &layer);
                        }
                        y -= 1;
                        down += 1;
                    }
                }
            }
        }
        self.cut(region, random, origin, berg);
    }

    fn smooth(&self, region: &mut Region, origin: IVec3, berg: &Berg) {
        let radius = if berg.ellipse { berg.long_radius } else { berg.radius / 2 };
        for x in -radius..=radius {
            for z in -radius..=radius {
                for y in 0..=berg.height {
                    let pos = origin + IVec3::new(x, y, z);
                    let here = read(region, pos);
                    if !self.is_iceberg_block(here) && here != self.snow_layer {
                        continue;
                    }
                    if read(region, pos - IVec3::Y) == self.air {
                        region.set(pos, self.air);
                        region.set(pos + IVec3::Y, self.air);
                    } else if self.is_iceberg_block(here) {
                        let open = HORIZONTAL.iter().filter(|[dx, dz]| !self.is_iceberg_block(read(region, pos + IVec3::new(*dx, 0, *dz)))).count();
                        if open > OPEN_SIDES_LIMIT {
                            region.set(pos, self.air);
                        }
                    }
                }
            }
        }
    }

    fn cut(&self, region: &mut Region, random: &mut MersenneTwister, origin: IVec3, berg: &Berg) {
        let first = random.next_float();
        let go = if berg.ellipse { first > ELLIPSE_CUT_CHANCE } else { random.next_float() > CUT_CHANCE };
        if !go {
            return;
        }
        let (mut k, mut l) = (self.hole_offset(random, berg), self.hole_offset(random, berg));
        if berg.ellipse {
            k = random.next_int_bounded((berg.long_radius - 5).max(1));
            l = k;
        }
        let kx = if random.next_boolean() { -k } else { k };
        let lz = if random.next_boolean() { -l } else { l };
        let angle = if berg.ellipse {
            berg.angle + FRAC_PI_2
        } else {
            let turn = random.next_float();
            (turn + turn) * PI
        };
        let centre = [kx, lz];
        let cut = Cut { angle, centre };
        if berg.height > 3 {
            for y in 0..berg.height - 3 {
                let radius = self.round_radius(berg, random, y, berg.height);
                self.carve(region, origin, berg, &Slice { radius, y, water: false }, &cut);
            }
        }
        let (mut y, mut down) = (-1, 1);
        while random.next_int_bounded(5) - berg.height < y {
            let factor = random.next_float();
            let radius = ceil_to((1.0 - down as f32 / ((factor * 0.5 + 1.0) * berg.height as f32)) * berg.radius as f32 * 0.5);
            self.carve(region, origin, berg, &Slice { radius, y, water: true }, &cut);
            y -= 1;
            down += 1;
        }
    }

    fn hole_offset(&self, random: &mut MersenneTwister, berg: &Berg) -> i32 {
        let radius = berg.radius;
        let shallow = random.next_int_bounded((radius / 2).max(HOLE_MIN_SPAN) - 2);
        if random.next_boolean() {
            radius / 2 - random.next_int_bounded((radius - radius / 2 - 1).max(1)) + 1
        } else {
            shallow
        }
    }

    fn carve(&self, region: &mut Region, origin: IVec3, berg: &Berg, slice: &Slice, cut: &Cut) {
        let long = slice.radius + berg.long_radius / 3 + 1;
        let short = slice.radius.min(CARVE_SHORT_CAP) + berg.short_radius / 2 - 4;
        for k in -long..long {
            for l in -long..long {
                let distance = self.distance(cut.angle, (k - cut.centre[0]) as f32, (l - cut.centre[1]) as f32, long as f32, short as f32);
                if distance.is_nan() || distance >= 0.0 {
                    continue;
                }
                let pos = origin + IVec3::new(k, slice.y, l);
                let here = read(region, pos);
                if !self.is_iceberg_block(here) && here != self.snow_layer {
                    continue;
                }
                if slice.water {
                    region.set(pos, self.water);
                } else {
                    region.set(pos, self.air);
                    if read(region, pos + IVec3::Y) == self.snow_layer {
                        region.set(pos + IVec3::Y, self.air);
                    }
                }
            }
        }
    }

    fn blue_patch(&self, region: &mut Region, random: &mut MersenneTwister, origin: IVec3) {
        if origin.y >= region.sea_level() || (!is_water(region, origin) && !is_water(region, origin - IVec3::Y)) {
            return;
        }
        if !SIDES[1..].iter().any(|side| read(region, origin + IVec3::from(*side)) == self.packed_ice) {
            return;
        }
        region.set(origin, self.blue_ice);
        for _ in 0..PATCH_GROWTH_TRIES {
            let dy = random.next_int_bounded(PATCH_DROP_BASE) - random.next_int_bounded(PATCH_DROP_SPREAD);
            if dy <= -PATCH_DROP_SPREAD {
                continue;
            }
            let span = if dy < PATCH_GROW_TALL { dy / 2 + PATCH_GROW_BASE } else { PATCH_GROW_LIMIT };
            let (a, b, c, d) = (
                random.next_int_bounded(span),
                random.next_int_bounded(span),
                random.next_int_bounded(span),
                random.next_int_bounded(span),
            );
            let pos = origin + IVec3::new(a - b, dy, c - d);
            let there = read(region, pos);
            if there != self.air && there != self.packed_ice && there != self.ice && !is_water(region, pos) {
                continue;
            }
            let neighbours = SIDES.iter().filter(|side| read(region, pos + IVec3::from(**side)) == self.blue_ice).count();
            if (PATCH_NEIGHBOURS_MIN..=PATCH_NEIGHBOURS_MAX).contains(&neighbours) {
                region.set(pos, self.blue_ice);
            }
        }
    }

    pub fn place(&self, region: &mut Region, random: &mut MersenneTwister, origin: IVec3) -> bool {
        if random.next_int_bounded(BERG_ONE_IN) == 0 {
            let dx = random.next_int_bounded(BERG_SPREAD);
            let dz = random.next_int_bounded(BERG_SPREAD);
            let berg = self.berg(random);
            self.raise(region, random, IVec3::new(origin.x + dx + BERG_OFFSET, region.sea_level(), origin.z + dz + BERG_OFFSET), &berg);
        }
        for _ in 0..random.next_int_bounded(PATCH_TRIES) {
            let dx = random.next_int_bounded(PATCH_SPREAD);
            let y = random.next_int_bounded(PATCH_Y_SPREAD) + region.sea_level() - PATCH_DEPTH_BELOW_SEA;
            let dz = random.next_int_bounded(PATCH_SPREAD);
            self.blue_patch(region, random, IVec3::new(origin.x + dx + PATCH_OFFSET, y, origin.z + dz + PATCH_OFFSET));
        }
        true
    }
}
