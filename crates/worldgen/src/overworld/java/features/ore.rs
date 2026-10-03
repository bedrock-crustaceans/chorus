use glam::IVec3;

use super::super::blocks::{BlockId, BlockTable};
use super::super::carver::sin;
use super::super::random::RandomSource;
use super::super::tags::Tag;
use super::key::Key;
use super::placement::{Modifier, Placed, count};
use super::predicate::Direction;
use super::provider::{Anchor, BOTTOM, HeightProvider, IntProvider, TOP};
use super::region::Heightmap;
use super::{Context, Feature};
use chorus_block::block_id::*;

#[derive(Clone)]
pub enum RuleTest {
    Tag(Tag),
    Height(i32, i32),
    Either(Box<RuleTest>, Box<RuleTest>, Box<RuleTest>),
}

impl RuleTest {
    fn test(&self, ctx: &Context, block: BlockId, pos: IVec3) -> bool {
        match self {
            Self::Tag(tag) => ctx.region.blocks.is(block, *tag),
            Self::Height(min, max) => (*min..=*max).contains(&pos.y),
            Self::Either(condition, if_true, if_false) => {
                if condition.test(ctx, block, pos) {
                    if_true.test(ctx, block, pos)
                } else {
                    if_false.test(ctx, block, pos)
                }
            }
        }
    }
}

pub struct Ore {
    targets: Vec<(RuleTest, BlockId)>,
    size: i32,
    discard_chance_on_air_exposure: f32,
}

impl Ore {
    pub fn place(&self, ctx: &mut Context, random: &mut dyn RandomSource, origin: IVec3) -> bool {
        let dir = random.next_float() * std::f32::consts::PI;
        let spread = self.size as f32 / 8.0;
        let max_radius = ((self.size as f32 / 16.0 * 2.0 + 1.0) / 2.0).ceil() as i32;
        let x0 = origin.x as f64 + (dir as f64).sin() * spread as f64;
        let x1 = origin.x as f64 - (dir as f64).sin() * spread as f64;
        let z0 = origin.z as f64 + (dir as f64).cos() * spread as f64;
        let z1 = origin.z as f64 - (dir as f64).cos() * spread as f64;
        let y0 = (origin.y + random.next_int_bounded(3) - 2) as f64;
        let y1 = (origin.y + random.next_int_bounded(3) - 2) as f64;
        let x_start = origin.x - spread.ceil() as i32 - max_radius;
        let y_start = origin.y - 2 - max_radius;
        let z_start = origin.z - spread.ceil() as i32 - max_radius;
        let size_xz = 2 * (spread.ceil() as i32 + max_radius);
        let size_y = 2 * (2 + max_radius);
        for x in x_start..=x_start + size_xz {
            for z in z_start..=z_start + size_xz {
                if y_start <= ctx.region.height(Heightmap::OceanFloorWg, x, z) {
                    return self.do_place(ctx, random, [x0, x1, z0, z1, y0, y1], IVec3::new(x_start, y_start, z_start), size_xz, size_y);
                }
            }
        }
        false
    }

    fn do_place(&self, ctx: &mut Context, random: &mut dyn RandomSource, [x0, x1, z0, z1, y0, y1]: [f64; 6], start: IVec3, size_xz: i32, size_y: i32) -> bool {
        let size = self.size as usize;
        let mut data = vec![[0.0f64; 4]; size];
        for (i, entry) in data.iter_mut().enumerate() {
            let step = i as f32 / self.size as f32;
            let lerp = |a: f64, b: f64| a + step as f64 * (b - a);
            let ss = random.next_double() * self.size as f64 / 16.0;
            let r = ((sin((std::f32::consts::PI * step) as f64) + 1.0) as f64 * ss + 1.0) / 2.0;
            *entry = [lerp(x0, x1), lerp(y0, y1), lerp(z0, z1), r];
        }
        for i1 in 0..size.saturating_sub(1) {
            if data[i1][3] <= 0.0 {
                continue;
            }
            for i2 in i1 + 1..size {
                if data[i2][3] <= 0.0 {
                    continue;
                }
                let dx = data[i1][0] - data[i2][0];
                let dy = data[i1][1] - data[i2][1];
                let dz = data[i1][2] - data[i2][2];
                let dr = data[i1][3] - data[i2][3];
                if dr * dr > dx * dx + dy * dy + dz * dz {
                    if dr > 0.0 {
                        data[i2][3] = -1.0;
                    } else {
                        data[i1][3] = -1.0;
                    }
                }
            }
        }

        assert!(size_xz <= 64, "ore blob wider than a tested-row mask");
        let stride = size_xz * size_y;
        let mut tested = vec![0u64; stride as usize];
        let mut placed = 0;
        for &[xx, yy, zz, r] in &data {
            if r < 0.0 {
                continue;
            }
            let x_min = ((xx - r).floor() as i32).max(start.x);
            let y_min = ((yy - r).floor() as i32).max(start.y);
            let z_min = ((zz - r).floor() as i32).max(start.z);
            let x_max = ((xx + r).floor() as i32).max(x_min);
            let y_max = ((yy + r).floor() as i32).max(y_min);
            let z_max = ((zz + r).floor() as i32).max(z_min);
            let mut y_scaled = [0.0f64; 64];
            for (scaled, y) in y_scaled.iter_mut().zip(y_min..=y_max) {
                *scaled = (y as f64 + 0.5 - yy) / r;
            }
            for x in x_min..=x_max {
                let xd = (x as f64 + 0.5 - xx) / r;
                if xd * xd >= 1.0 {
                    continue;
                }
                for (y, &yd) in (y_min..=y_max).zip(&y_scaled) {
                    let xy = xd * xd + yd * yd;
                    if xy >= 1.0 || ctx.region.is_outside_build_height(y) {
                        continue;
                    }
                    let base = x - start.x + (y - start.y) * size_xz;
                    let lift = base / stride - start.z;
                    let row = (base % stride) as usize;
                    let chord = r * (1.0 - xy).sqrt();
                    let first = ((zz - 0.5 - chord).ceil() as i32 - 1).max(z_min);
                    let last = ((zz - 0.5 + chord).floor() as i32 + 1).min(z_max);
                    let (low, high) = (first + lift, (last + lift).min(size_xz - 1));
                    if low > high {
                        continue;
                    }
                    let mut candidates = (u64::MAX >> (63 - high)) & (u64::MAX << low) & !tested[row];
                    while candidates != 0 {
                        let bit = candidates.trailing_zeros();
                        candidates &= candidates - 1;
                        let z = bit as i32 - lift;
                        let zd = (z as f64 + 0.5 - zz) / r;
                        if xy + zd * zd >= 1.0 {
                            continue;
                        }
                        tested[row] |= 1 << bit;
                        let pos = IVec3::new(x, y, z);
                        let Some(current) = ctx.region.get_contained(pos) else { continue };
                        for (rule, state) in &self.targets {
                            if self.can_place(ctx, random, rule, current, pos) {
                                ctx.region.set(pos, *state);
                                placed += 1;
                                break;
                            }
                        }
                    }
                }
            }
        }
        placed > 0
    }

    fn can_place(&self, ctx: &Context, random: &mut dyn RandomSource, rule: &RuleTest, block: BlockId, pos: IVec3) -> bool {
        if !rule.test(ctx, block, pos) {
            return false;
        }
        let skip_air_check = if self.discard_chance_on_air_exposure <= 0.0 {
            true
        } else if self.discard_chance_on_air_exposure >= 1.0 {
            false
        } else {
            random.next_float() >= self.discard_chance_on_air_exposure
        };
        skip_air_check || !Direction::ALL.iter().any(|direction| ctx.region.blocks.is_air(ctx.region.get(pos + direction.offset())))
    }
}

pub struct OreFeatures {
    natural_stone: RuleTest,
    stone_ore: RuleTest,
    deepslate_ore: RuleTest,
}

impl OreFeatures {
    pub fn new() -> Self {
        let height_specific = Box::new(RuleTest::Tag(Tag::HeightSpecificOreReplaceables));
        Self {
            natural_stone: RuleTest::Tag(Tag::BaseStoneOverworld),
            stone_ore: RuleTest::Either(height_specific.clone(), Box::new(RuleTest::Height(0, i32::MAX)), Box::new(RuleTest::Tag(Tag::StoneOreReplaceables))),
            deepslate_ore: RuleTest::Either(height_specific, Box::new(RuleTest::Height(i32::MIN, 8)), Box::new(RuleTest::Tag(Tag::DeepslateOreReplaceables))),
        }
    }

    fn stone(&self, table: &mut BlockTable, block: &'static str, size: i32) -> Feature {
        Feature::Ore(Ore {
            targets: vec![(self.natural_stone.clone(), table.name(block))],
            size,
            discard_chance_on_air_exposure: 0.0,
        })
    }

    fn ore(&self, table: &mut BlockTable, (stone, deepslate): (&'static str, &'static str), size: i32, discard: f32) -> Feature {
        Feature::Ore(Ore {
            targets: vec![(self.stone_ore.clone(), table.name(stone)), (self.deepslate_ore.clone(), table.name(deepslate))],
            size,
            discard_chance_on_air_exposure: discard,
        })
    }
}

fn ore_placement(frequency: Modifier, height: HeightProvider) -> Vec<Modifier> {
    vec![frequency, Modifier::InSquare, Modifier::HeightRange(height), Modifier::Biome]
}

fn common(count_per_chunk: i32, height: HeightProvider) -> Vec<Modifier> {
    ore_placement(count(count_per_chunk), height)
}

fn rare(rarity: i32, height: HeightProvider) -> Vec<Modifier> {
    ore_placement(Modifier::Rarity(rarity), height)
}

fn uniform(min: Anchor, max: Anchor) -> HeightProvider {
    HeightProvider::Uniform(min, max)
}

fn triangle(min: Anchor, max: Anchor) -> HeightProvider {
    HeightProvider::Trapezoid(min, max, 0)
}

use Anchor::{AboveBottom, Absolute};

pub fn define(key: Key, table: &mut BlockTable) -> Option<Placed> {
    let o = OreFeatures::new();
    let (feature, placement) = match key {
        Key::OreDirt => (o.stone(table, DIRT, 33), common(7, uniform(Absolute(0), Absolute(160)))),
        Key::OreGravel => (o.stone(table, GRAVEL, 33), common(14, uniform(BOTTOM, TOP))),
        Key::OreGraniteUpper => (o.stone(table, GRANITE, 64), rare(6, uniform(Absolute(64), Absolute(128)))),
        Key::OreGraniteLower => (o.stone(table, GRANITE, 64), common(2, uniform(Absolute(0), Absolute(60)))),
        Key::OreDioriteUpper => (o.stone(table, DIORITE, 64), rare(6, uniform(Absolute(64), Absolute(128)))),
        Key::OreDioriteLower => (o.stone(table, DIORITE, 64), common(2, uniform(Absolute(0), Absolute(60)))),
        Key::OreAndesiteUpper => (o.stone(table, ANDESITE, 64), rare(6, uniform(Absolute(64), Absolute(128)))),
        Key::OreAndesiteLower => (o.stone(table, ANDESITE, 64), common(2, uniform(Absolute(0), Absolute(60)))),
        Key::OreTuff => (o.stone(table, TUFF, 64), common(2, uniform(BOTTOM, Absolute(0)))),
        Key::OreClay => (o.stone(table, CLAY, 33), common(46, uniform(BOTTOM, Absolute(256)))),
        Key::OreCoalUpper => (o.ore(table, (COAL_ORE, DEEPSLATE_COAL_ORE), 17, 0.0), common(30, uniform(Absolute(136), TOP))),
        Key::OreCoalLower => (o.ore(table, (COAL_ORE, DEEPSLATE_COAL_ORE), 17, 0.5), common(20, triangle(Absolute(0), Absolute(192)))),
        Key::OreIronUpper => (o.ore(table, (IRON_ORE, DEEPSLATE_IRON_ORE), 9, 0.0), common(90, triangle(Absolute(80), Absolute(384)))),
        Key::OreIronMiddle => (o.ore(table, (IRON_ORE, DEEPSLATE_IRON_ORE), 9, 0.0), common(10, triangle(Absolute(-24), Absolute(56)))),
        Key::OreIronSmall => (o.ore(table, (IRON_ORE, DEEPSLATE_IRON_ORE), 4, 0.0), common(10, uniform(BOTTOM, Absolute(72)))),
        Key::OreGoldExtra => (o.ore(table, (GOLD_ORE, DEEPSLATE_GOLD_ORE), 9, 0.0), common(50, uniform(Absolute(32), Absolute(256)))),
        Key::OreGold => (o.ore(table, (GOLD_ORE, DEEPSLATE_GOLD_ORE), 9, 0.5), common(4, triangle(Absolute(-64), Absolute(32)))),
        Key::OreGoldLower => (
            o.ore(table, (GOLD_ORE, DEEPSLATE_GOLD_ORE), 9, 0.5),
            ore_placement(Modifier::Count(IntProvider::Uniform(0, 1)), uniform(Absolute(-64), Absolute(-48))),
        ),
        Key::OreRedstone => (o.ore(table, (REDSTONE_ORE, DEEPSLATE_REDSTONE_ORE), 8, 0.0), common(4, uniform(BOTTOM, Absolute(15)))),
        Key::OreRedstoneLower => (o.ore(table, (REDSTONE_ORE, DEEPSLATE_REDSTONE_ORE), 8, 0.0), common(8, triangle(AboveBottom(-32), AboveBottom(32)))),
        Key::OreDiamond => (o.ore(table, (DIAMOND_ORE, DEEPSLATE_DIAMOND_ORE), 4, 0.5), common(7, triangle(AboveBottom(-80), AboveBottom(80)))),
        Key::OreDiamondMedium => (o.ore(table, (DIAMOND_ORE, DEEPSLATE_DIAMOND_ORE), 8, 0.5), common(2, uniform(Absolute(-64), Absolute(-4)))),
        Key::OreDiamondLarge => (o.ore(table, (DIAMOND_ORE, DEEPSLATE_DIAMOND_ORE), 12, 0.7), rare(9, triangle(AboveBottom(-80), AboveBottom(80)))),
        Key::OreDiamondBuried => (o.ore(table, (DIAMOND_ORE, DEEPSLATE_DIAMOND_ORE), 8, 1.0), common(4, triangle(AboveBottom(-80), AboveBottom(80)))),
        Key::OreLapis => (o.ore(table, (LAPIS_ORE, DEEPSLATE_LAPIS_ORE), 7, 0.0), common(2, triangle(Absolute(-32), Absolute(32)))),
        Key::OreLapisBuried => (o.ore(table, (LAPIS_ORE, DEEPSLATE_LAPIS_ORE), 7, 1.0), common(4, uniform(BOTTOM, Absolute(64)))),
        Key::OreInfested => (o.ore(table, (INFESTED_STONE, INFESTED_DEEPSLATE), 9, 0.0), common(14, uniform(BOTTOM, Absolute(63)))),
        Key::OreEmerald => (o.ore(table, (EMERALD_ORE, DEEPSLATE_EMERALD_ORE), 3, 0.0), common(100, triangle(Absolute(-16), Absolute(480)))),
        Key::OreCopper => (o.ore(table, (COPPER_ORE, DEEPSLATE_COPPER_ORE), 10, 0.0), common(16, triangle(Absolute(-16), Absolute(112)))),
        Key::OreCopperLarge => (o.ore(table, (COPPER_ORE, DEEPSLATE_COPPER_ORE), 20, 0.0), common(16, triangle(Absolute(-16), Absolute(112)))),
        _ => return None,
    };
    Some(Placed::new(feature, placement))
}
