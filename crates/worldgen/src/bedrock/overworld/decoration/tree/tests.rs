use super::draw::{Odds, Range};
use super::legacy::{LegacyTrees, Picker};
use crate::shared::rng::mersenne::MersenneTwister;

fn next_after(random: &mut MersenneTwister) -> u32 {
    random.next_u32()
}

#[test]
fn constant_ranges_draw_nothing() {
    let mut used = MersenneTwister::new(7);
    let mut fresh = MersenneTwister::new(7);
    assert_eq!(Range::constant(5).draw(&mut used), 5);
    assert_eq!(Range::new(5, 6).draw(&mut used), 5);
    assert_eq!(Range::constant(5).draw_inclusive(&mut used), 5);
    assert_eq!(next_after(&mut used), next_after(&mut fresh));
}

#[test]
fn ranges_draw_one_bounded_value() {
    let mut used = MersenneTwister::new(7);
    let mut fresh = MersenneTwister::new(7);
    assert_eq!(Range::new(4, 7).draw(&mut used), 4 + fresh.next_int_bounded(3));
    assert_eq!(Range::new(4, 7).draw_inclusive(&mut used), 4 + fresh.next_int_bounded(4));
    assert_eq!(Range::constant(2).draw_inclusive_always(&mut used), 2 + fresh.next_int_bounded(1));
}

#[test]
fn odds_use_a_word_for_fractions_and_a_float_for_percents() {
    let mut used = MersenneTwister::new(11);
    let mut fresh = MersenneTwister::new(11);
    assert_eq!(Odds::Fraction(1, 4).roll(&mut used), fresh.next_u32() % 4 < 1);
    assert_eq!(Odds::Percent(30.0).roll(&mut used), fresh.next_float() * 100.0 < 30.0);
    assert!(Odds::Percent(100.0).roll(&mut used));
    assert!(!Odds::Percent(0.0).roll(&mut used));
    assert!(Odds::Fraction(3, 3).roll(&mut used));
    assert_eq!(next_after(&mut used), next_after(&mut fresh));
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
