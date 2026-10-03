use glam::IVec3;

use super::super::blocks::{AIR, BlockId, BlockTable, state, text};
use super::super::random::RandomSource;
use super::super::tags::Tag;
use super::basic::simple;
use super::key::Key;
use super::placement::{Modifier, Placed, count, filter, offset, vertical_offset};
use super::predicate::{BlockPredicate, Direction, Fluid, can_survive, empty, fluid_at};
use super::provider::{Anchor, BOTTOM, HeightProvider, IntProvider, StateProvider, TOP};
use super::region::{Heightmap, Region};
use super::{Context, Feature, SEA_LEVEL};
use chorus_block::block_id::*;
use chorus_block::state::common::POTENT_SULFUR_STATE;

pub struct Spike {
    state: BlockId,
    can_place_on: BlockPredicate,
    can_replace: BlockPredicate,
}

impl Spike {
    pub fn place(&self, ctx: &mut Context, random: &mut dyn RandomSource, mut origin: IVec3) -> bool {
        let blocks = ctx.region.blocks;
        while blocks.is_air(ctx.region.get(origin)) && origin.y > ctx.region.min_y() + 2 {
            origin.y -= 1;
        }
        if !self.can_place_on.test(ctx.region, origin) {
            return false;
        }
        origin.y += random.next_int_bounded(4);
        let height = random.next_int_bounded(4) + 7;
        let width = height / 4 + random.next_int_bounded(2);
        if width > 1 && random.next_int_bounded(60) == 0 {
            origin.y += 10 + random.next_int_bounded(30);
        }
        for y_off in 0..height {
            let scale = (1.0 - y_off as f32 / height as f32) * width as f32;
            let new_width = scale.ceil() as i32;
            for xo in -new_width..=new_width {
                let dx = xo.abs() as f32 - 0.25;
                for zo in -new_width..=new_width {
                    let dz = zo.abs() as f32 - 0.25;
                    let inside = (xo == 0 && zo == 0) || dx * dx + dz * dz <= scale * scale;
                    let edge = xo == -new_width || xo == new_width || zo == -new_width || zo == new_width;
                    if inside && (!edge || random.next_float() <= 0.75) {
                        let up = origin + IVec3::new(xo, y_off, zo);
                        if blocks.is_air(ctx.region.get(up)) || self.can_replace.test(ctx.region, up) {
                            ctx.region.set(up, self.state);
                        }
                        if y_off != 0 && new_width > 1 {
                            let down = origin + IVec3::new(xo, -y_off, zo);
                            if blocks.is_air(ctx.region.get(down)) || self.can_replace.test(ctx.region, down) {
                                ctx.region.set(down, self.state);
                            }
                        }
                    }
                }
            }
        }
        let pillar = (width - 1).clamp(0, 1);
        for xo in -pillar..=pillar {
            for zo in -pillar..=pillar {
                let mut cursor = origin + IVec3::new(xo, -1, zo);
                let mut run = if xo.abs() == 1 && zo.abs() == 1 { random.next_int_bounded(5) } else { 50 };
                while cursor.y > 50 {
                    let state = ctx.region.get(cursor);
                    if !blocks.is_air(state) && !self.can_replace.test(ctx.region, cursor) && state != self.state {
                        break;
                    }
                    ctx.region.set(cursor, self.state);
                    cursor.y -= 1;
                    run -= 1;
                    if run <= 0 {
                        cursor.y -= random.next_int_bounded(5) + 1;
                        run = random.next_int_bounded(5);
                    }
                }
            }
        }
        true
    }
}

pub struct Disk {
    state: StateProvider,
    target: BlockPredicate,
    radius: IntProvider,
    half_height: i32,
}

impl Disk {
    pub fn place(&self, ctx: &mut Context, random: &mut dyn RandomSource, origin: IVec3) -> bool {
        let mut placed_any = false;
        let top = origin.y + self.half_height;
        let bottom = origin.y - self.half_height - 1;
        let r = self.radius.sample(random);
        for z in origin.z - r..=origin.z + r {
            for x in origin.x - r..=origin.x + r {
                let (xd, zd) = (x - origin.x, z - origin.z);
                if xd * xd + zd * zd > r * r {
                    continue;
                }
                let mut placed_above = false;
                let mut y = top;
                while y > bottom {
                    let pos = IVec3::new(x, y, z);
                    if self.target.test(ctx.region, pos) {
                        if let Some(state) = self.state.optional_state(ctx.region, random, pos) {
                            ctx.region.set(pos, state);
                            if !placed_above {
                                ctx.region.mark_above(pos);
                            }
                            placed_any = true;
                            placed_above = true;
                        }
                    } else {
                        placed_above = false;
                    }
                    y -= 1;
                }
            }
        }
        placed_any
    }
}

pub struct BlockBlob {
    state: BlockId,
    can_place_on: BlockPredicate,
}

impl BlockBlob {
    pub fn place(&self, ctx: &mut Context, random: &mut dyn RandomSource, mut origin: IVec3) -> bool {
        while origin.y > ctx.region.min_y() + 3 && !self.can_place_on.test(ctx.region, origin - IVec3::Y) {
            origin.y -= 1;
        }
        if origin.y <= ctx.region.min_y() + 3 {
            return false;
        }
        for _ in 0..3 {
            let xr = random.next_int_bounded(2);
            let yr = random.next_int_bounded(2);
            let zr = random.next_int_bounded(2);
            let tr = (xr + yr + zr) as f32 * 0.333 + 0.5;
            for x in -xr..=xr {
                for y in -yr..=yr {
                    for z in -zr..=zr {
                        if ((x * x + y * y + z * z) as f32) <= tr * tr {
                            ctx.region.set(origin + IVec3::new(x, y, z), self.state);
                        }
                    }
                }
            }
            origin += IVec3::new(-1 + random.next_int_bounded(2), -random.next_int_bounded(2), -1 + random.next_int_bounded(2));
        }
        true
    }
}

pub struct BlueIce {
    water: BlockId,
    packed_ice: BlockId,
    ice: BlockId,
    blue_ice: BlockId,
}

impl BlueIce {
    pub fn place(&self, ctx: &mut Context, random: &mut dyn RandomSource, origin: IVec3) -> bool {
        if origin.y > SEA_LEVEL - 1 {
            return false;
        }
        if ctx.region.get(origin) != self.water && ctx.region.get(origin - IVec3::Y) != self.water {
            return false;
        }
        if !Direction::ALL
            .iter()
            .any(|&direction| direction != Direction::Down && ctx.region.get(origin + direction.offset()) == self.packed_ice)
        {
            return false;
        }
        ctx.region.set(origin, self.blue_ice);
        for _ in 0..200 {
            let y_off = random.next_int_bounded(5) - random.next_int_bounded(6);
            let mut xz = 3;
            if y_off < 2 {
                xz += y_off / 2;
            }
            if xz < 1 {
                continue;
            }
            let pos = origin
                + IVec3::new(
                    random.next_int_bounded(xz) - random.next_int_bounded(xz),
                    y_off,
                    random.next_int_bounded(xz) - random.next_int_bounded(xz),
                );
            let state = ctx.region.get(pos);
            let replaceable = ctx.region.blocks.is_air(state) || state == self.water || state == self.packed_ice || state == self.ice;
            if replaceable && Direction::ALL.iter().any(|direction| ctx.region.get(pos + direction.offset()) == self.blue_ice) {
                ctx.region.set(pos, self.blue_ice);
            }
        }
        true
    }
}

pub struct Lake {
    fluid: BlockId,
    barrier: BlockId,
    can_place: BlockPredicate,
    can_replace_with_air_or_fluid: BlockPredicate,
    can_replace_with_barrier: BlockPredicate,
    ice: BlockId,
}

fn shell(grid: &[bool; 2048], x: usize, z: usize, y: usize) -> bool {
    let at = |x: usize, z: usize, y: usize| grid[(x * 16 + z) * 8 + y];
    !at(x, z, y) && ((x < 15 && at(x + 1, z, y)) || (x > 0 && at(x - 1, z, y)) || (z < 15 && at(x, z + 1, y)) || (z > 0 && at(x, z - 1, y)) || (y < 7 && at(x, z, y + 1)) || (y > 0 && at(x, z, y - 1)))
}

impl Lake {
    pub fn place(&self, ctx: &mut Context, random: &mut dyn RandomSource, origin: IVec3) -> bool {
        if origin.y <= ctx.region.min_y() + 4 {
            return false;
        }
        let origin = origin - IVec3::new(8, 4, 8);
        let mut grid = [false; 2048];
        let spots = random.next_int_bounded(4) + 4;
        for _ in 0..spots {
            let xr = random.next_double() * 6.0 + 3.0;
            let yr = random.next_double() * 4.0 + 2.0;
            let zr = random.next_double() * 6.0 + 3.0;
            let xp = random.next_double() * (16.0 - xr - 2.0) + 1.0 + xr / 2.0;
            let yp = random.next_double() * (8.0 - yr - 4.0) + 2.0 + yr / 2.0;
            let zp = random.next_double() * (16.0 - zr - 2.0) + 1.0 + zr / 2.0;
            for xx in 1..15 {
                for zz in 1..15 {
                    for yy in 1..7 {
                        let xd = (xx as f64 - xp) / (xr / 2.0);
                        let yd = (yy as f64 - yp) / (yr / 2.0);
                        let zd = (zz as f64 - zp) / (zr / 2.0);
                        if xd * xd + yd * yd + zd * zd < 1.0 {
                            grid[(xx * 16 + zz) * 8 + yy] = true;
                        }
                    }
                }
            }
        }
        let blocks = ctx.region.blocks;
        for xx in 0..16 {
            for zz in 0..16 {
                for yy in 0..8 {
                    if !shell(&grid, xx, zz, yy) {
                        continue;
                    }
                    let pos = origin + IVec3::new(xx as i32, yy as i32, zz as i32);
                    let state = ctx.region.get(pos);
                    if yy >= 4 && blocks.entry(state).liquid {
                        return false;
                    }
                    if yy < 4 && !blocks.entry(state).solid && state != self.fluid {
                        return false;
                    }
                    if !self.can_place.test(ctx.region, pos) {
                        return false;
                    }
                }
            }
        }
        for xx in 0..16 {
            for zz in 0..16 {
                for yy in 0..8 {
                    if grid[(xx * 16 + zz) * 8 + yy] {
                        let pos = origin + IVec3::new(xx as i32, yy as i32, zz as i32);
                        if self.can_replace_with_air_or_fluid.test(ctx.region, pos) {
                            let air = yy >= 4;
                            ctx.region.set(pos, if air { AIR } else { self.fluid });
                            if air {
                                ctx.region.mark_above(pos);
                            }
                        }
                    }
                }
            }
        }
        if !blocks.is_air(self.barrier) {
            for xx in 0..16 {
                for zz in 0..16 {
                    for yy in 0..8 {
                        if shell(&grid, xx, zz, yy) && (yy < 4 || random.next_int_bounded(2) != 0) {
                            let pos = origin + IVec3::new(xx as i32, yy as i32, zz as i32);
                            if blocks.entry(ctx.region.get(pos)).solid && self.can_replace_with_barrier.test(ctx.region, pos) {
                                ctx.region.set(pos, self.barrier);
                                ctx.region.mark_above(pos);
                            }
                        }
                    }
                }
            }
        }
        if fluid_at(blocks, self.fluid) == Some(Fluid::Water) {
            for xx in 0..16 {
                for zz in 0..16 {
                    let pos = origin + IVec3::new(xx, 4, zz);
                    if should_freeze(ctx.region, pos) && self.can_replace_with_air_or_fluid.test(ctx.region, pos) {
                        ctx.region.set(pos, self.ice);
                    }
                }
            }
        }
        true
    }
}

pub fn should_freeze(region: &Region, pos: IVec3) -> bool {
    let biome = region.biome(pos);
    if biome.warm_enough_to_rain(pos.x, pos.y, pos.z, SEA_LEVEL) || region.is_outside_build_height(pos.y) {
        return false;
    }
    region.blocks.name_of(region.get(pos)) == WATER
}

pub struct SnowAndFreeze {
    ice: BlockId,
    snow: BlockId,
}

impl SnowAndFreeze {
    pub fn place(&self, ctx: &mut Context, origin: IVec3) -> bool {
        let base = IVec3::new((origin.x >> 4) << 4, origin.y, (origin.z >> 4) << 4);
        for dx in 0..16 {
            for dz in 0..16 {
                let (x, z) = (base.x + dx, base.z + dz);
                let y = ctx.region.height(Heightmap::MotionBlocking, x, z);
                let top = IVec3::new(x, y, z);
                let below = top - IVec3::Y;
                if should_freeze(ctx.region, below) {
                    ctx.region.set(below, self.ice);
                }
                let biome = ctx.region.biome(top);
                let current = ctx.region.get(top);
                if biome.has_precipitation()
                    && biome.cold_enough_to_snow(top.x, top.y, top.z, SEA_LEVEL)
                    && !ctx.region.is_outside_build_height(top.y)
                    && (ctx.region.blocks.is_air(current) || current == self.snow)
                    && can_survive(ctx.region, self.snow, top)
                {
                    ctx.region.set(top, self.snow);
                }
            }
        }
        true
    }
}

pub struct Spring {
    fluid: BlockId,
    requires_block_below: bool,
    rock_count: i32,
    hole_count: i32,
    valid: Vec<u16>,
}

impl Spring {
    pub fn place(&self, ctx: &mut Context, origin: IVec3) -> bool {
        let blocks = ctx.region.blocks;
        let valid = |pos: IVec3| self.valid.contains(&blocks.entry(ctx.region.get(pos)).kind);
        if !valid(origin + IVec3::Y) || (self.requires_block_below && !valid(origin - IVec3::Y)) {
            return false;
        }
        let current = ctx.region.get(origin);
        if !blocks.is_air(current) && !self.valid.contains(&blocks.entry(current).kind) {
            return false;
        }
        let sides = [IVec3::NEG_X, IVec3::X, IVec3::NEG_Z, IVec3::Z, IVec3::NEG_Y];
        let rocks = sides.iter().filter(|&&side| valid(origin + side)).count() as i32;
        let holes = sides.iter().filter(|&&side| blocks.is_air(ctx.region.get(origin + side))).count() as i32;
        if rocks == self.rock_count && holes == self.hole_count {
            ctx.region.set(origin, self.fluid);
            return true;
        }
        false
    }
}

pub struct Iceberg {
    main: BlockId,
    snow_block: BlockId,
    ice: BlockId,
    water: BlockId,
    packed_ice: BlockId,
    blue_ice: BlockId,
    snow: BlockId,
}

impl Iceberg {
    fn is_iceberg(&self, state: BlockId) -> bool {
        state == self.packed_ice || state == self.snow_block || state == self.blue_ice
    }

    pub fn place(&self, ctx: &mut Context, random: &mut dyn RandomSource, origin: IVec3) -> bool {
        let origin = IVec3::new(origin.x, SEA_LEVEL, origin.z);
        let snow_on_top = random.next_double() > 0.7;
        let shape_angle = random.next_double() * 2.0 * std::f64::consts::PI;
        let ellipse_a = 11 - random.next_int_bounded(5);
        let ellipse_c = 3 + random.next_int_bounded(3);
        let is_ellipse = random.next_double() > 0.7;
        let mut over = if is_ellipse { random.next_int_bounded(6) + 6 } else { random.next_int_bounded(15) + 3 };
        if !is_ellipse && random.next_double() > 0.9 {
            over += random.next_int_bounded(19) + 7;
        }
        let under = (over + random.next_int_bounded(11)).min(18);
        let width = (over + random.next_int_bounded(7) - random.next_int_bounded(5)).min(11);
        let a = if is_ellipse { ellipse_a } else { 11 };
        for xo in -a..a {
            for zo in -a..a {
                for y_off in 0..over {
                    let radius = if is_ellipse {
                        radius_ellipse(y_off, over, width)
                    } else {
                        radius_round(random, y_off, over, width)
                    };
                    if is_ellipse || xo < radius {
                        self.block(ctx, random, origin, over, IVec3::new(xo, y_off, zo), radius, a, is_ellipse, ellipse_c, shape_angle, snow_on_top);
                    }
                }
            }
        }
        self.smooth(ctx, origin, width, over, is_ellipse, ellipse_a);
        for xo in -a..a {
            for zo in -a..a {
                let mut y_off = -1;
                while y_off > -under {
                    let new_a = if is_ellipse {
                        (a as f32 * (1.0 - (y_off as f64).powi(2) as f32 / (under as f32 * 8.0))).ceil() as i32
                    } else {
                        a
                    };
                    let radius = radius_steep(random, -y_off, under, width);
                    if xo < radius {
                        self.block(ctx, random, origin, under, IVec3::new(xo, y_off, zo), radius, new_a, is_ellipse, ellipse_c, shape_angle, snow_on_top);
                    }
                    y_off -= 1;
                }
            }
        }
        let cut_out = if is_ellipse { random.next_double() > 0.1 } else { random.next_double() > 0.7 };
        if cut_out {
            self.cut_out(ctx, random, width, over, origin, is_ellipse, ellipse_a, shape_angle, ellipse_c);
        }
        true
    }

    #[allow(clippy::too_many_arguments)]
    fn cut_out(&self, ctx: &mut Context, random: &mut dyn RandomSource, width: i32, height: i32, origin: IVec3, is_ellipse: bool, ellipse_a: i32, shape_angle: f64, ellipse_c: i32) {
        let sign_x = if random.next_boolean() { -1 } else { 1 };
        let sign_z = if random.next_boolean() { -1 } else { 1 };
        let mut x_off = random.next_int_bounded((width / 2 - 2).max(1));
        if random.next_boolean() {
            x_off = width / 2 + 1 - random.next_int_bounded((width - width / 2 - 1).max(1));
        }
        let mut z_off = random.next_int_bounded((width / 2 - 2).max(1));
        if random.next_boolean() {
            z_off = width / 2 + 1 - random.next_int_bounded((width - width / 2 - 1).max(1));
        }
        if is_ellipse {
            x_off = random.next_int_bounded((ellipse_a - 5).max(1));
            z_off = x_off;
        }
        let local = IVec3::new(sign_x * x_off, 0, sign_z * z_off);
        let angle = if is_ellipse {
            shape_angle + std::f64::consts::FRAC_PI_2
        } else {
            random.next_double() * 2.0 * std::f64::consts::PI
        };
        for y_off in 0..height - 3 {
            let radius = radius_round(random, y_off, height, width);
            self.carve(ctx, radius, y_off, origin, false, angle, local, ellipse_a, ellipse_c);
        }
        let mut y_off = -1;
        while y_off > -height + random.next_int_bounded(5) {
            let radius = radius_steep(random, -y_off, height, width);
            self.carve(ctx, radius, y_off, origin, true, angle, local, ellipse_a, ellipse_c);
            y_off -= 1;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn carve(&self, ctx: &mut Context, radius: i32, y_off: i32, origin: IVec3, under_water: bool, angle: f64, local: IVec3, ellipse_a: i32, ellipse_c: i32) {
        let a = radius + 1 + ellipse_a / 3;
        let c = (radius - 3).min(3) + ellipse_c / 2 - 1;
        for xo in -a..a {
            for zo in -a..a {
                if signed_distance_ellipse(xo, zo, local, a, c, angle) < 0.0 {
                    let pos = origin + IVec3::new(xo, y_off, zo);
                    let state = ctx.region.get(pos);
                    if self.is_iceberg(state) || state == self.snow_block {
                        if under_water {
                            ctx.region.set(pos, self.water);
                        } else {
                            ctx.region.set(pos, AIR);
                            if ctx.region.get(pos + IVec3::Y) == self.snow {
                                ctx.region.set(pos + IVec3::Y, AIR);
                            }
                        }
                    }
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn block(&self, ctx: &mut Context, random: &mut dyn RandomSource, origin: IVec3, height: i32, offset: IVec3, radius: i32, a: i32, is_ellipse: bool, ellipse_c: i32, angle: f64, snow_on_top: bool) {
        let (xo, y_off, zo) = (offset.x, offset.y, offset.z);
        let signed = if is_ellipse {
            let mut c = ellipse_c;
            if y_off > 0 && height - y_off <= 3 {
                c -= 4 - (height - y_off);
            }
            signed_distance_ellipse(xo, zo, IVec3::ZERO, a, c, angle)
        } else {
            let off = 10.0 * random.next_float().clamp(0.2, 0.8) / radius as f32;
            off as f64 + (xo as f64).powi(2) + (zo as f64).powi(2) - (radius as f64).powi(2)
        };
        if signed >= 0.0 {
            return;
        }
        let pos = origin + offset;
        let compare = if is_ellipse { -0.5 } else { (-6 - random.next_int_bounded(3)) as f64 };
        if signed > compare && random.next_double() > 0.9 {
            return;
        }
        let state = ctx.region.get(pos);
        if ctx.region.blocks.is_air(state) || state == self.snow_block || state == self.ice || state == self.water {
            let randomness = !is_ellipse || random.next_double() > 0.05;
            let divisor = if is_ellipse { 3 } else { 2 };
            let h_diff = height - y_off;
            if snow_on_top && state != self.water && (h_diff as f64) <= (random.next_int_bounded((height / divisor).max(1)) as f64 + height as f64 * 0.6) && randomness {
                ctx.region.set(pos, self.snow_block);
            } else {
                ctx.region.set(pos, self.main);
            }
        }
    }

    fn smooth(&self, ctx: &mut Context, origin: IVec3, width: i32, height: i32, is_ellipse: bool, ellipse_a: i32) {
        let a = if is_ellipse { ellipse_a } else { width / 2 };
        for x in -a..=a {
            for z in -a..=a {
                for y_off in 0..=height {
                    let pos = origin + IVec3::new(x, y_off, z);
                    let state = ctx.region.get(pos);
                    if !(self.is_iceberg(state) || state == self.snow) {
                        continue;
                    }
                    if ctx.region.blocks.is_air(ctx.region.get(pos - IVec3::Y)) {
                        ctx.region.set(pos, AIR);
                        ctx.region.set(pos + IVec3::Y, AIR);
                    } else if self.is_iceberg(state) {
                        let open = [IVec3::NEG_X, IVec3::X, IVec3::NEG_Z, IVec3::Z]
                            .iter()
                            .filter(|&&side| !self.is_iceberg(ctx.region.get(pos + side)))
                            .count();
                        if open >= 3 {
                            ctx.region.set(pos, AIR);
                        }
                    }
                }
            }
        }
    }
}

fn signed_distance_ellipse(xo: i32, zo: i32, origin: IVec3, a: i32, c: i32, angle: f64) -> f64 {
    let (dx, dz) = ((xo - origin.x) as f64, (zo - origin.z) as f64);
    ((dx * angle.cos() - dz * angle.sin()) / a as f64).powi(2) + ((dx * angle.sin() + dz * angle.cos()) / c as f64).powi(2) - 1.0
}

fn radius_round(random: &mut dyn RandomSource, y_off: i32, height: i32, width: i32) -> i32 {
    let k = 3.5 - random.next_float();
    let mut scale = (1.0 - (y_off as f64).powi(2) as f32 / (height as f32 * k)) * width as f32;
    if height > 15 + random.next_int_bounded(5) {
        let temp = if y_off < 3 + random.next_int_bounded(6) { y_off / 2 } else { y_off };
        scale = (1.0 - temp as f32 / (height as f32 * k * 0.4)) * width as f32;
    }
    (scale / 2.0).ceil() as i32
}

fn radius_ellipse(y_off: i32, height: i32, width: i32) -> i32 {
    let scale = (1.0 - (y_off as f64).powi(2) as f32 / height as f32) * width as f32;
    (scale / 2.0).ceil() as i32
}

fn radius_steep(random: &mut dyn RandomSource, y_off: i32, height: i32, width: i32) -> i32 {
    let k = 1.0 + random.next_float() / 2.0;
    let scale = (1.0 - y_off as f32 / (height as f32 * k)) * width as f32;
    (scale / 2.0).ceil() as i32
}

fn kinds(table: &mut BlockTable, names: &[&'static str]) -> Vec<u16> {
    names.iter().map(|name| table.kind(name)).collect()
}

fn matching(table: &mut BlockTable, names: &[&'static str]) -> BlockPredicate {
    BlockPredicate::MatchingBlocks(IVec3::ZERO, kinds(table, names))
}

fn disk(table: &mut BlockTable, state: impl DiskState, targets: &[&'static str], radius: IntProvider, half_height: i32) -> Feature {
    let state = state.provider(table);
    Feature::Disk(Disk {
        state,
        target: matching(table, targets),
        radius,
        half_height,
    })
}

fn spring(table: &mut BlockTable, fluid: &'static str, valid: &[&'static str]) -> Feature {
    Feature::Spring(Spring {
        fluid: table.name(fluid),
        requires_block_below: true,
        rock_count: 4,
        hole_count: 1,
        valid: kinds(table, valid),
    })
}

fn lake(table: &mut BlockTable, fluid: &'static str, barrier: &'static str, can_place: BlockPredicate) -> Feature {
    Feature::Lake(Lake {
        fluid: table.name(fluid),
        barrier: table.name(barrier),
        can_place,
        can_replace_with_air_or_fluid: BlockPredicate::MatchingTag(IVec3::ZERO, Tag::FeaturesCannotReplace).not(),
        can_replace_with_barrier: BlockPredicate::MatchingTag(IVec3::ZERO, Tag::LavaPoolStoneCannotReplace).not(),
        ice: table.name(ICE),
    })
}

fn iceberg(table: &mut BlockTable, main: &'static str) -> Feature {
    Feature::Iceberg(Iceberg {
        main: table.name(main),
        snow_block: table.name(SNOW),
        ice: table.name(ICE),
        water: table.name(WATER),
        packed_ice: table.name(PACKED_ICE),
        blue_ice: table.name(BLUE_ICE),
        snow: table.name(SNOW_LAYER),
    })
}

trait DiskState {
    fn provider(self, table: &mut BlockTable) -> StateProvider;
}

impl DiskState for &'static str {
    fn provider(self, table: &mut BlockTable) -> StateProvider {
        table.name(self).into()
    }
}

impl DiskState for StateProvider {
    fn provider(self, _table: &mut BlockTable) -> StateProvider {
        self
    }
}

const OVERWORLD_SPRING_ROCK: &[&str] = &[STONE, GRANITE, DIORITE, ANDESITE, DEEPSLATE, TUFF, CALCITE, DIRT];

pub fn define(key: Key, table: &mut BlockTable) -> Option<Placed> {
    let surface = Modifier::Heightmap(Heightmap::MotionBlocking);
    let top_solid = Modifier::Heightmap(Heightmap::OceanFloorWg);
    let in_water = filter(BlockPredicate::MatchingFluids(IVec3::ZERO, vec![Fluid::Water]));
    let spring_height = Modifier::HeightRange(HeightProvider::VeryBiasedToBottom(BOTTOM, Anchor::BelowTop(8), 8));
    let (feature, placement) = match key {
        Key::IceSpike => {
            let feature = Feature::Spike(Spike {
                state: table.name(PACKED_ICE),
                can_place_on: matching(table, &[SNOW]),
                can_replace: BlockPredicate::MatchingTag(IVec3::ZERO, Tag::IceSpikeReplaceable),
            });
            (feature, vec![count(3), Modifier::InSquare, surface, Modifier::Biome])
        }
        Key::IcePatch => {
            let feature = disk(table, PACKED_ICE, &[DIRT, GRASS_BLOCK, PODZOL, COARSE_DIRT, MYCELIUM, SNOW, ICE], IntProvider::Uniform(2, 3), 1);
            (
                feature,
                vec![count(2), Modifier::InSquare, surface, offset(0, -1, 0), filter(matching(table, &[SNOW])), Modifier::Biome],
            )
        }
        Key::ForestRock => {
            let feature = Feature::BlockBlob(BlockBlob {
                state: table.name(MOSSY_COBBLESTONE),
                can_place_on: BlockPredicate::MatchingTag(IVec3::ZERO, Tag::ForestRockCanPlaceOn),
            });
            (feature, vec![count(2), Modifier::InSquare, surface, Modifier::Biome])
        }
        Key::IcebergBlue => (iceberg(table, BLUE_ICE), vec![Modifier::Rarity(200), Modifier::InSquare, Modifier::Biome]),
        Key::IcebergPacked => (iceberg(table, PACKED_ICE), vec![Modifier::Rarity(16), Modifier::InSquare, Modifier::Biome]),
        Key::BlueIce => {
            let feature = Feature::BlueIce(BlueIce {
                water: table.name(WATER),
                packed_ice: table.name(PACKED_ICE),
                ice: table.name(ICE),
                blue_ice: table.name(BLUE_ICE),
            });
            (
                feature,
                vec![
                    Modifier::Count(IntProvider::Uniform(0, 19)),
                    Modifier::InSquare,
                    Modifier::HeightRange(HeightProvider::Uniform(Anchor::Absolute(30), Anchor::Absolute(61))),
                    Modifier::Biome,
                ],
            )
        }
        Key::LakeLavaUnderground => {
            let scan_target = BlockPredicate::AllOf(vec![empty().not(), BlockPredicate::InsideWorld(IVec3::new(0, -5, 0))]);
            let placement = vec![
                Modifier::Rarity(9),
                Modifier::InSquare,
                Modifier::HeightRange(HeightProvider::Uniform(Anchor::Absolute(0), TOP)),
                Modifier::EnvironmentScan {
                    direction: Direction::Down,
                    target: scan_target,
                    allowed: BlockPredicate::True,
                    max_steps: 32,
                },
                Modifier::SurfaceRelativeThreshold {
                    heightmap: Heightmap::OceanFloorWg,
                    min: i32::MIN,
                    max: -5,
                },
                Modifier::Biome,
            ];
            (lake(table, LAVA, STONE, BlockPredicate::True), placement)
        }
        Key::LakeLavaSurface => (
            lake(table, LAVA, STONE, BlockPredicate::True),
            vec![Modifier::Rarity(200), Modifier::InSquare, Modifier::Heightmap(Heightmap::WorldSurfaceWg), Modifier::Biome],
        ),
        Key::DiskClay => (
            disk(table, CLAY, &[DIRT, CLAY], IntProvider::Uniform(2, 3), 1),
            vec![Modifier::InSquare, top_solid, in_water, Modifier::Biome],
        ),
        Key::DiskGravel => (
            disk(table, GRAVEL, &[DIRT, GRASS_BLOCK], IntProvider::Uniform(2, 5), 2),
            vec![Modifier::InSquare, top_solid, in_water, Modifier::Biome],
        ),
        Key::DiskSand => {
            let air_below = BlockPredicate::MatchingBlocks(IVec3::NEG_Y, kinds(table, &[chorus_block::block_id::AIR]));
            let state = StateProvider::RuleBased {
                fallback: Some(Box::new(table.name(SAND).into())),
                rules: vec![(air_below, table.name(SANDSTONE).into())],
            };
            (
                disk(table, state, &[DIRT, GRASS_BLOCK], IntProvider::Uniform(2, 6), 2),
                vec![count(3), Modifier::InSquare, top_solid, in_water, Modifier::Biome],
            )
        }
        Key::DiskGrass => {
            let covered = BlockPredicate::AnyOf(vec![BlockPredicate::Solid(IVec3::Y), BlockPredicate::MatchingFluids(IVec3::Y, vec![Fluid::Water])]);
            let state = StateProvider::RuleBased {
                fallback: Some(Box::new(table.name(DIRT).into())),
                rules: vec![(covered.not(), table.name(GRASS_BLOCK).into())],
            };
            let feature = disk(table, state, &[DIRT, MUD], IntProvider::Uniform(2, 6), 2);
            (
                feature,
                vec![count(1), Modifier::InSquare, top_solid, offset(0, -1, 0), filter(matching(table, &[MUD])), Modifier::Biome],
            )
        }
        Key::FreezeTopLayer => (
            Feature::SnowAndFreeze(SnowAndFreeze {
                ice: table.name(ICE),
                snow: table.name(SNOW_LAYER),
            }),
            vec![Modifier::Biome],
        ),
        Key::SpringLava => (spring(table, LAVA, OVERWORLD_SPRING_ROCK), vec![count(20), Modifier::InSquare, spring_height, Modifier::Biome]),
        Key::SpringLavaFrozen => (
            spring(table, LAVA, &[SNOW, POWDER_SNOW, PACKED_ICE]),
            vec![count(20), Modifier::InSquare, spring_height, Modifier::Biome],
        ),
        Key::SpringWater => {
            let mut valid = OVERWORLD_SPRING_ROCK.to_vec();
            valid.extend([SNOW, POWDER_SNOW, PACKED_ICE]);
            (
                spring(table, WATER, &valid),
                vec![
                    count(25),
                    Modifier::InSquare,
                    Modifier::HeightRange(HeightProvider::Uniform(BOTTOM, Anchor::Absolute(192))),
                    Modifier::Biome,
                ],
            )
        }
        Key::SulfurPool => {
            let not_spike = matching(table, &[SULFUR_SPIKE]).not();
            let pool = Placed::new(lake(table, WATER, SULFUR, not_spike), Vec::new());
            let potent = table.get(POTENT_SULFUR, &[state(POTENT_SULFUR_STATE, text("wet"))]);
            let scan = Modifier::EnvironmentScan {
                direction: Direction::Down,
                target: BlockPredicate::AllOf(vec![BlockPredicate::Solid(IVec3::ZERO), BlockPredicate::MatchingFluids(IVec3::Y, vec![Fluid::Water])]),
                allowed: BlockPredicate::True,
                max_steps: 4,
            };
            let potent = Placed::new(simple(potent), vec![scan]);
            let placement = vec![
                count(256),
                Modifier::InSquare,
                Modifier::HeightRange(HeightProvider::Uniform(BOTTOM, Anchor::Absolute(256))),
                filter(BlockPredicate::Solid(IVec3::ZERO)),
                Modifier::EnvironmentScan {
                    direction: Direction::Up,
                    target: empty(),
                    allowed: BlockPredicate::True,
                    max_steps: 32,
                },
                vertical_offset((-1).into()),
                filter(matching(table, &[SULFUR])),
                Modifier::Biome,
            ];
            (Feature::Sequence(vec![pool, potent]), placement)
        }
        _ => return None,
    };
    Some(Placed::new(feature, placement))
}
