use crate::shared::math::sine;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::Probed;

const MAX_STEPS: usize = 1000;
const WANTED_STARTS: usize = 3;
const FULL_TURN: f32 = 6.283_185_5;
const FIRST_RADIUS: i32 = 40;
const RADIUS_SPREAD: i32 = 16;
const SCAN_BEFORE: i32 = 8;
const SCAN_AFTER: i32 = 7;
const HIT_RADIUS_STEP: i32 = 8;
const HIT_TURN: f32 = 1.884_955_8;
const MISS_RADIUS_STEP: i32 = 4;
const MISS_TURN: f32 = std::f32::consts::FRAC_PI_4;

pub fn spiral_starts(level_seed: u32, is_village: &dyn Fn((i32, i32)) -> Probed<bool>) -> Probed<Vec<(i32, i32)>> {
    let mut random = MersenneTwister::new(level_seed);
    let mut angle = random.next_float() * FULL_TURN;
    let mut radius = random.next_int_bounded(RADIUS_SPREAD) + FIRST_RADIUS;
    let mut found = Vec::with_capacity(WANTED_STARTS);
    for _ in 0..MAX_STEPS {
        let x = (sine::cos(angle) * radius as f32).floor() as i32;
        let z = (radius as f32 * sine::sin(angle)).floor() as i32;
        let mut hit = None;
        'scan: for chunk_x in x - SCAN_BEFORE..=x + SCAN_AFTER {
            for chunk_z in z - SCAN_BEFORE..=z + SCAN_AFTER {
                if is_village((chunk_x, chunk_z))? {
                    hit = Some((chunk_x, chunk_z));
                    break 'scan;
                }
            }
        }
        let (step, turn) = match hit {
            Some(chunk) => {
                found.push(chunk);
                (HIT_RADIUS_STEP, HIT_TURN)
            }
            None => (MISS_RADIUS_STEP, MISS_TURN),
        };
        if found.len() >= WANTED_STARTS {
            break;
        }
        radius += step;
        angle += turn;
    }
    Ok(found)
}
