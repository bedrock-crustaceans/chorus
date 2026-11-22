mod common;

use common::seeds::GENERAL_1;
use common::{fingerprints, positions};

#[test]
#[ignore = "run with: cargo test --release -p chorus_worldgen --test determinism -- --ignored"]
fn chunks_do_not_depend_on_request_order() {
    let forward = positions();
    let mut reversed = forward.clone();
    reversed.reverse();
    let first = fingerprints(GENERAL_1, &forward);
    let second = fingerprints(GENERAL_1, &reversed);
    let different: Vec<_> = forward.iter().filter(|cell| first[cell] != second[cell]).collect();
    assert!(
        different.is_empty(),
        "{} of {} chunks depend on request order, for example {:?}",
        different.len(),
        forward.len(),
        &different[..different.len().min(5)]
    );
}
