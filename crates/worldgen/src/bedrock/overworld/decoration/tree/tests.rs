use super::legacy::{LegacyTrees, Picker};
use crate::shared::rng::mersenne::MersenneTwister;

fn next_after(random: &mut MersenneTwister) -> u32 {
    random.next_u32()
}

#[test]
fn tree_count_picks_one_of_three_formulas() {
    let trees = LegacyTrees {
        count: 10.0,
        picker: Picker::First,
        features: Vec::new(),
    };
    let mut used = MersenneTwister::new(3);
    let mut fresh = MersenneTwister::new(3);
    let expected = if fresh.next_int_bounded(10) == 0 {
        (10.0 * fresh.next_float()) as i32
    } else {
        (10.0 + fresh.next_int_bounded(10).max(7) as f32) as i32 - 7
    };
    assert_eq!(trees.tree_count(&mut used), expected);
    assert_eq!(next_after(&mut used), next_after(&mut fresh));
}
