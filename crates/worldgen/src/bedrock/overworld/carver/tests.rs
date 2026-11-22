use super::canyon::{self, max_y};
use super::cave::{EXTRA_CAVE_CARVER, Segment, UNDERGROUND_CAVE_CARVER};
use super::trig;
use crate::bedrock::overworld::decoration::seed::large_feature_multipliers;
use crate::shared::rng::mersenne::MersenneTwister;
use std::f32::consts::{FRAC_PI_2, PI};

const GENERAL_1: i64 = 4802433367836389877;
const LEVEL_SEED: u32 = GENERAL_1 as u32;
const CANYON_DRAWS_BEFORE_WALK: usize = 12;
const SOURCE_SAMPLE: i32 = 40;
const MAX_CAVE_LENGTH: i32 = 112;
const MAX_CANYON_LENGTH: i32 = 120;
const CHUNK_SIZE: f32 = 16.0;

fn source_seed(x: i32, z: i32) -> u32 {
    let (x_multiplier, z_multiplier) = large_feature_multipliers(LEVEL_SEED);
    LEVEL_SEED ^ (z as u32).wrapping_mul(z_multiplier).wrapping_add((x as u32).wrapping_mul(x_multiplier))
}

fn sources() -> impl Iterator<Item = (i32, i32)> {
    (-SOURCE_SAMPLE..SOURCE_SAMPLE).flat_map(|x| (-SOURCE_SAMPLE..SOURCE_SAMPLE).map(move |z| (x, z)))
}

#[test]
fn large_feature_multipliers_are_odd_and_seed_the_origin_with_the_level_seed() {
    let (x_multiplier, z_multiplier) = large_feature_multipliers(LEVEL_SEED);
    assert_eq!(x_multiplier & 1, 1);
    assert_eq!(z_multiplier & 1, 1);
    assert_eq!(source_seed(0, 0), LEVEL_SEED);
    assert_eq!(source_seed(1, 0), LEVEL_SEED ^ x_multiplier);
    assert_eq!(source_seed(0, 1), LEVEL_SEED ^ z_multiplier);
}

#[test]
fn sine_and_cosine_hit_the_quarter_turns() {
    assert_eq!(trig::sin(0.0), 0.0);
    assert!((trig::sin(FRAC_PI_2) - 1.0).abs() < 1e-6);
    assert!((trig::cos(0.0) - 1.0).abs() < 1e-6);
    assert!((trig::cos(PI) + 1.0).abs() < 1e-6);
    assert!((trig::sin(-1.0) + 0.841_470_96).abs() < 1e-6);
}

#[test]
fn cave_segments_stay_near_their_source_and_inside_the_build_height() {
    let mut segments: Vec<Segment> = Vec::new();
    let mut produced = 0;
    for settings in [&UNDERGROUND_CAVE_CARVER, &EXTRA_CAVE_CARVER] {
        for (x, z) in sources() {
            settings.segments(&mut MersenneTwister::new(source_seed(x, z)), x, z, &mut segments);
            produced += segments.len();
            for segment in &segments {
                assert!(segment.step >= 0 && segment.step < segment.length && segment.length <= MAX_CAVE_LENGTH);
                assert!(segment.horizontal_radius > 0.0 && segment.vertical_radius > 0.0);
                assert!(segment.floor_level >= -1.0 && segment.floor_level <= -0.4);
                let reach = (MAX_CAVE_LENGTH as f32 + 2.0) * 2.0;
                assert!((segment.center[0] - (x as f32 + 0.5) * CHUNK_SIZE).abs() < reach);
                assert!((segment.center[2] - (z as f32 + 0.5) * CHUNK_SIZE).abs() < reach);
                assert!(segment.center[1] > -(MAX_CAVE_LENGTH as f32) && segment.center[1] < (settings.height_limit + MAX_CAVE_LENGTH) as f32);
            }
        }
    }
    assert!(produced > 0);
}

#[test]
fn cave_generation_is_repeatable_and_the_skip_gate_is_rarely_open() {
    let mut first: Vec<Segment> = Vec::new();
    let mut second: Vec<Segment> = Vec::new();
    let (mut open, mut total) = (0, 0);
    for (x, z) in sources() {
        UNDERGROUND_CAVE_CARVER.segments(&mut MersenneTwister::new(source_seed(x, z)), x, z, &mut first);
        UNDERGROUND_CAVE_CARVER.segments(&mut MersenneTwister::new(source_seed(x, z)), x, z, &mut second);
        assert_eq!(first.len(), second.len());
        assert!(first.iter().zip(&second).all(|(a, b)| a.center == b.center && a.horizontal_radius == b.horizontal_radius));
        total += 1;
        open += usize::from(!first.is_empty());
    }
    assert!(open * 10 < total);
}

#[test]
fn canyon_consumes_twelve_draws_before_its_walk_and_never_more_than_one_step_per_length() {
    let mut steps = Vec::new();
    let mut found = 0;
    for (x, z) in sources() {
        let seed = source_seed(x, z);
        let mut random = MersenneTwister::new(seed);
        let Some(canyon) = canyon::canyon(&mut random, x, z, -64, &mut steps) else { continue };
        found += 1;
        let mut reference = MersenneTwister::new(seed);
        reference.skip(CANYON_DRAWS_BEFORE_WALK);
        assert_eq!(random.next_u32(), reference.next_u32());
        assert!(!steps.is_empty() && steps.len() as i32 <= MAX_CANYON_LENGTH);
        assert!(steps.windows(2).all(|pair| pair[0].remaining > pair[1].remaining));
        assert!(steps.iter().all(|step| step.horizontal_radius > 0.0 && step.vertical_radius > 0.0));
        assert!(canyon.widths.iter().all(|&width| (1.0..=4.0).contains(&width)));
        assert!(steps.iter().all(|step| step.center[1] < max_y(384) as f32));
    }
    assert!(found > 0);
}

#[test]
fn canyon_chance_is_about_one_in_a_hundred() {
    let mut steps = Vec::new();
    let found = sources()
        .filter(|&(x, z)| canyon::canyon(&mut MersenneTwister::new(source_seed(x, z)), x, z, -64, &mut steps).is_some())
        .count();
    let total = (SOURCE_SAMPLE * SOURCE_SAMPLE * 4) as usize;
    assert!(found > total / 300 && found < total / 40, "{found} of {total}");
}

#[test]
fn the_leading_word_checks_agree_with_the_full_generators() {
    let mut segments = Vec::new();
    let mut steps = Vec::new();
    for seed in (0..20000u32).map(|value| value.wrapping_mul(2_654_435_761)) {
        for settings in [&UNDERGROUND_CAVE_CARVER, &EXTRA_CAVE_CARVER] {
            settings.segments(&mut MersenneTwister::new(seed), 3, -4, &mut segments);
            if !settings.starts(MersenneTwister::leading_words::<4>(seed)) {
                assert!(segments.is_empty(), "{seed}");
            }
        }
        if !canyon::starts(MersenneTwister::leading_words::<1>(seed)[0]) {
            assert!(canyon::canyon(&mut MersenneTwister::new(seed), 3, -4, -64, &mut steps).is_none(), "{seed}");
        }
    }
}
