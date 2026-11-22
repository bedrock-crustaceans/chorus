use super::super::decorate::View;
use crate::shared::blocks::{AIR, BlockId};
use crate::shared::rng::mersenne::MersenneTwister;

const FACES: [[i32; 3]; 6] = [[0, -1, 0], [0, 1, 0], [0, 0, -1], [0, 0, 1], [-1, 0, 0], [1, 0, 0]];

const VINE_SHORT_ODDS: i32 = 5;

const VINE_DOUBLE_ODDS: i32 = 6;

const VINE_LENGTH_ADD: i32 = 1;

const WEEPING_AGES: (i32, i32) = (17, 25);

const WEEPING_SHELL_TRIES: i32 = 200;

const WEEPING_SHELL_REACH: i32 = 12;

const WEEPING_SHELL_DEPTH: i32 = 7;

const WEEPING_SHELL_BELOW: i32 = 5;

const WEEPING_VINE_TRIES: i32 = 100;

const WEEPING_VINE_REACH: i32 = 16;

const WEEPING_VINE_DEPTH: i32 = 9;

const WEEPING_VINE_BELOW: i32 = 8;

const WEEPING_LENGTH_BOUND: i32 = 7;

const TWISTING_TRIES: i32 = 100;

const TWISTING_REACH: i32 = 10;

const TWISTING_DEPTH: i32 = 5;

const TWISTING_BELOW: i32 = 4;

const TWISTING_SHIFT: i32 = 5;

const TWISTING_AGES: i32 = 25;

pub fn vine_length(random: &mut MersenneTwister, bound: i32, add: i32) -> i32 {
    let length = (random.next_int_bounded(bound) + add) << u32::from(random.next_int_bounded(VINE_DOUBLE_ODDS) == 0);
    if random.next_int_bounded(VINE_SHORT_ODDS) == 0 { 1 } else { length }
}

pub(super) fn weeping_column(view: &mut View, random: &mut MersenneTwister, [x, mut y, z]: [i32; 3], length: i32, (low, high): (i32, i32)) {
    for index in 0..=length {
        if view.get(x, y, z) == AIR {
            let age = if low < high { random.next_int_bounded(high - low) + low } else { low };
            let block = view.ids().weeping_vines[age as usize];
            view.set(x, y, z, block);
            if index == length || view.get(x, y - 1, z) != AIR {
                break;
            }
        }
        y -= 1;
    }
}

fn holds_weeping(view: &View, block: BlockId) -> bool {
    block == view.ids().netherrack || block == view.ids().nether_wart
}

pub(super) fn weeping_cluster(view: &mut View, random: &mut MersenneTwister, [x, y, z]: [i32; 3]) -> bool {
    if view.get(x, y, z) != AIR || !holds_weeping(view, view.get(x, y + 1, z)) {
        return false;
    }
    let wart = view.ids().nether_wart;
    view.set(x, y, z, wart);
    for _ in 0..WEEPING_SHELL_TRIES {
        let [px, py, pz] = [
            x + random.next_int_bounded(WEEPING_SHELL_REACH) - WEEPING_SHELL_REACH / 2,
            y + random.next_int_bounded(WEEPING_SHELL_DEPTH) - WEEPING_SHELL_BELOW,
            z + random.next_int_bounded(WEEPING_SHELL_REACH) - WEEPING_SHELL_REACH / 2,
        ];
        let touching = FACES.iter().filter(|[dx, dy, dz]| holds_weeping(view, view.get(px + dx, py + dy, pz + dz))).count();
        if view.get(px, py, pz) == AIR && touching == 1 {
            view.set(px, py, pz, wart);
        }
    }
    for _ in 0..WEEPING_VINE_TRIES {
        let [px, py, pz] = [
            x + random.next_int_bounded(WEEPING_VINE_REACH) - WEEPING_VINE_REACH / 2,
            y + random.next_int_bounded(WEEPING_VINE_DEPTH) - WEEPING_VINE_BELOW,
            z + random.next_int_bounded(WEEPING_VINE_REACH) - WEEPING_VINE_REACH / 2,
        ];
        if view.get(px, py, pz) == AIR && holds_weeping(view, view.get(px, py + 1, pz)) {
            let length = vine_length(random, WEEPING_LENGTH_BOUND, VINE_LENGTH_ADD);
            weeping_column(view, random, [px, py, pz], length, WEEPING_AGES);
        }
    }
    true
}

pub(super) fn twisting_cluster(view: &mut View, random: &mut MersenneTwister, [x, y, z]: [i32; 3]) -> bool {
    let ids = view.ids();
    for _ in 0..TWISTING_TRIES {
        let px = x + random.next_int_bounded(TWISTING_REACH) - random.next_int_bounded(TWISTING_REACH) - TWISTING_SHIFT;
        let pz = z + random.next_int_bounded(TWISTING_REACH) - random.next_int_bounded(TWISTING_REACH) - TWISTING_SHIFT;
        let py = y + random.next_int_bounded(TWISTING_DEPTH) - TWISTING_BELOW;
        if view.get(px, py, pz) != AIR {
            continue;
        }
        let below = view.get(px, py - 1, pz);
        if !(below == ids.netherrack || below == ids.warped_nylium || below == ids.warped_wart) {
            continue;
        }
        let length = vine_length(random, WEEPING_LENGTH_BOUND, VINE_LENGTH_ADD);
        for (cy, index) in (py..).zip(0..=length) {
            if view.get(px, cy, pz) == AIR {
                let age = random.next_int_bounded(TWISTING_AGES);
                let block = ids.twisting_vines[age as usize];
                view.set(px, cy, pz, block);
                if index == length || view.get(px, cy + 1, pz) != AIR {
                    break;
                }
            }
        }
    }
    true
}
