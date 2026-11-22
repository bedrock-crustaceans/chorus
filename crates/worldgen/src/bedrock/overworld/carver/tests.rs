use super::cave::{EXTRA_CAVE_CARVER, Segment, UNDERGROUND_CAVE_CARVER};
use crate::bedrock::overworld::decoration::seed::large_feature_multipliers;
use crate::shared::rng::mersenne::MersenneTwister;

const GENERAL_1: i64 = 4802433367836389877;
const LEVEL_SEED: u32 = GENERAL_1 as u32;
const SOURCE_SAMPLE: i32 = 40;
const MAX_CAVE_LENGTH: i32 = 112;
const CHUNK_SIZE: f32 = 16.0;

fn source_seed(x: i32, z: i32) -> u32 {
    let (x_multiplier, z_multiplier) = large_feature_multipliers(LEVEL_SEED);
    LEVEL_SEED ^ (z as u32).wrapping_mul(z_multiplier).wrapping_add((x as u32).wrapping_mul(x_multiplier))
}

fn sources() -> impl Iterator<Item = (i32, i32)> {
    (-SOURCE_SAMPLE..SOURCE_SAMPLE).flat_map(|x| (-SOURCE_SAMPLE..SOURCE_SAMPLE).map(move |z| (x, z)))
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
