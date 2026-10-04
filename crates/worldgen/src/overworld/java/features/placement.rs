use glam::IVec3;

use super::super::biome::BIOME_INFO_NOISE;
use super::super::random::RandomSource;
use super::predicate::{BlockPredicate, Direction};
use super::provider::{HeightProvider, IntProvider};
use super::region::Heightmap;
use super::{Context, Feature, Key};

#[derive(Clone)]
pub enum Modifier {
    Count(IntProvider),
    InSquare,
    Heightmap(Heightmap),
    HeightRange(HeightProvider),
    Biome,
    Rarity(i32),
    Filter(BlockPredicate),
    Offset(IntProvider, IntProvider, IntProvider),
    SurfaceWaterDepth(i32),
    EnvironmentScan {
        direction: Direction,
        target: BlockPredicate,
        allowed: BlockPredicate,
        max_steps: i32,
    },
    NoiseThresholdCount {
        level: f64,
        below: i32,
        above: i32,
    },
    NoiseBasedCount {
        ratio: i32,
        factor: f64,
        offset: f64,
    },
    SurfaceRelativeThreshold {
        heightmap: Heightmap,
        min: i32,
        max: i32,
    },
    RandomChance(f32),
    Cuboid {
        xz: IntProvider,
        y: IntProvider,
        edges: bool,
        interior: bool,
    },
}

pub fn count(value: i32) -> Modifier {
    Modifier::Count(IntProvider::Constant(value))
}

pub fn count_extra(count: i32, chance: f32, extra: i32) -> Modifier {
    let weight = (1.0 / chance) as i32;
    Modifier::Count(IntProvider::Weighted(vec![(IntProvider::Constant(count), weight - 1), (IntProvider::Constant(count + extra), 1)]))
}

pub fn offset(x: i32, y: i32, z: i32) -> Modifier {
    Modifier::Offset(x.into(), y.into(), z.into())
}

pub fn vertical_offset(y: IntProvider) -> Modifier {
    Modifier::Offset(0.into(), y, 0.into())
}

pub fn filter(predicate: BlockPredicate) -> Modifier {
    Modifier::Filter(predicate)
}

pub struct Placed {
    pub feature: Feature,
    pub placement: Vec<Modifier>,
}

impl Placed {
    pub fn new(feature: Feature, placement: Vec<Modifier>) -> Self {
        Self { feature, placement }
    }

    pub fn place(&self, ctx: &mut Context, random: &mut dyn RandomSource, origin: IVec3, top: Option<Key>) -> bool {
        if self.placement.is_empty() {
            return self.feature.place(ctx, random, origin);
        }
        let mut placed = false;
        let mut stack = vec![(origin, 0usize)];
        let mut modified = Vec::new();
        while let Some((pos, index)) = stack.pop() {
            modified.clear();
            self.apply(&self.placement[index], ctx, random, pos, top, &mut modified);
            let next = index + 1;
            if next < self.placement.len() {
                for &pos in modified.iter().rev() {
                    stack.push((pos, next));
                }
            } else {
                for &pos in &modified {
                    placed |= self.feature.place(ctx, random, pos);
                }
            }
        }
        placed
    }

    fn apply(&self, modifier: &Modifier, ctx: &mut Context, random: &mut dyn RandomSource, origin: IVec3, top: Option<Key>, out: &mut Vec<IVec3>) {
        match modifier {
            Modifier::Count(count) => {
                for _ in 0..count.sample(random) {
                    out.push(origin);
                }
            }
            Modifier::InSquare => {
                let x = random.next_int_bounded(16) + origin.x;
                let z = random.next_int_bounded(16) + origin.z;
                out.push(IVec3::new(x, origin.y, z));
            }
            Modifier::Heightmap(kind) => {
                let height = ctx.region.height(*kind, origin.x, origin.z);
                if height > ctx.region.min_y() {
                    out.push(IVec3::new(origin.x, height, origin.z));
                }
            }
            Modifier::HeightRange(height) => out.push(IVec3::new(origin.x, height.sample(random), origin.z)),
            Modifier::Biome => {
                let biome = ctx.region.biome(origin);
                if top.is_some_and(|key| ctx.catalog.biome_has(biome, key)) {
                    out.push(origin);
                }
            }
            Modifier::Rarity(chance) => {
                if random.next_float() < 1.0 / *chance as f32 {
                    out.push(origin);
                }
            }
            Modifier::Filter(predicate) => {
                if predicate.test(ctx.region, origin) {
                    out.push(origin);
                }
            }
            Modifier::Offset(x, y, z) => {
                let (dx, dy, dz) = (x.sample(random), y.sample(random), z.sample(random));
                out.push(origin + IVec3::new(dx, dy, dz));
            }
            Modifier::SurfaceWaterDepth(max_depth) => {
                let ocean_floor = ctx.region.height(Heightmap::OceanFloor, origin.x, origin.z);
                let surface = ctx.region.height(Heightmap::WorldSurface, origin.x, origin.z);
                if surface - ocean_floor <= *max_depth {
                    out.push(origin);
                }
            }
            Modifier::EnvironmentScan {
                direction,
                target,
                allowed,
                max_steps,
            } => {
                let mut pos = origin;
                if !allowed.test(ctx.region, pos) {
                    return;
                }
                for _ in 0..*max_steps {
                    if target.test(ctx.region, pos) {
                        out.push(pos);
                        return;
                    }
                    pos += direction.offset();
                    if ctx.region.is_outside_build_height(pos.y) {
                        return;
                    }
                    if !allowed.test(ctx.region, pos) {
                        break;
                    }
                }
                if target.test(ctx.region, pos) {
                    out.push(pos);
                }
            }
            Modifier::NoiseThresholdCount { level, below, above } => {
                let noise = BIOME_INFO_NOISE.get(origin.x as f64 / 200.0, origin.z as f64 / 200.0) as f64;
                for _ in 0..(if noise < *level { *below } else { *above }) {
                    out.push(origin);
                }
            }
            Modifier::NoiseBasedCount { ratio, factor, offset } => {
                let noise = BIOME_INFO_NOISE.get(origin.x as f64 / factor, origin.z as f64 / factor) as f64;
                for _ in 0..((noise + offset) * *ratio as f64).ceil() as i32 {
                    out.push(origin);
                }
            }
            Modifier::SurfaceRelativeThreshold { heightmap, min, max } => {
                let surface = ctx.region.height(*heightmap, origin.x, origin.z);
                if surface as i64 + *min as i64 <= origin.y as i64 && origin.y as i64 <= surface as i64 + *max as i64 {
                    out.push(origin);
                }
            }
            Modifier::RandomChance(chance) => {
                if random.next_float() < *chance {
                    out.push(origin);
                }
            }
            Modifier::Cuboid { xz, y, edges, interior } => {
                let height = y.sample(random);
                let width = xz.sample(random);
                let length = xz.sample(random);
                for x in 0..=width {
                    for y in 0..=height {
                        for z in 0..=length {
                            let (x_face, y_face, z_face) = (x == 0 || x == width, y == 0 || y == height, z == 0 || z == length);
                            let on_edge = x_face as u8 + y_face as u8 + z_face as u8 >= 2;
                            let keep = (*edges || !on_edge) && (*interior || x_face || y_face || z_face);
                            if keep {
                                out.push(origin + IVec3::new(x, y, z));
                            }
                        }
                    }
                }
            }
        }
    }
}
