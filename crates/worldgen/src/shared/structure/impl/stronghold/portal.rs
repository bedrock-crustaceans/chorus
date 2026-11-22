use super::Door;
use super::build::{fill, portal_frame, selector_box, small_door};
use super::frame::{Canvas, Frame};
use super::material::Material;
use crate::shared::rng::mersenne::MersenneTwister;

const EYE_ABOVE: f32 = 0.9;
const FRAME_CELLS: [(i32, i32); 12] = [(4, 8), (5, 8), (6, 8), (4, 12), (5, 12), (6, 12), (3, 9), (3, 10), (3, 11), (7, 9), (7, 10), (7, 11)];
const FAR_ROW_FACING: [u8; 4] = [2, 3, 0, 1];
const LEFT_COLUMN_FACING: [u8; 4] = [3, 0, 3, 0];
const RIGHT_COLUMN_FACING: [u8; 4] = [1, 2, 1, 2];

pub fn portal_room(frame: &Frame, canvas: &mut Canvas, random: &mut MersenneTwister) {
    selector_box(frame, canvas, random, (0, 0, 0), (10, 7, 15));
    small_door(frame, canvas, Door::Grates, (4, 1, 0));
    selector_box(frame, canvas, random, (1, 6, 1), (1, 6, 14));
    selector_box(frame, canvas, random, (9, 6, 1), (9, 6, 14));
    selector_box(frame, canvas, random, (2, 6, 1), (8, 6, 2));
    selector_box(frame, canvas, random, (2, 6, 14), (8, 6, 14));
    selector_box(frame, canvas, random, (1, 1, 1), (2, 1, 4));
    selector_box(frame, canvas, random, (8, 1, 1), (9, 1, 4));
    fill(frame, canvas, (1, 1, 1), (1, 1, 3), Material::Lava);
    fill(frame, canvas, (9, 1, 1), (9, 1, 3), Material::Lava);
    selector_box(frame, canvas, random, (3, 1, 8), (7, 1, 12));
    fill(frame, canvas, (4, 1, 9), (6, 1, 11), Material::Lava);
    for z in (3..14).step_by(2) {
        fill(frame, canvas, (0, 3, z), (0, 4, z), Material::IronBars);
        fill(frame, canvas, (10, 3, z), (10, 4, z), Material::IronBars);
    }
    for x in (2..9).step_by(2) {
        fill(frame, canvas, (x, 3, 15), (x, 4, 15), Material::IronBars);
    }
    let step = frame.stairs(Material::BRICK_STAIRS, 3);
    selector_box(frame, canvas, random, (4, 1, 5), (6, 1, 7));
    selector_box(frame, canvas, random, (4, 2, 6), (6, 2, 7));
    selector_box(frame, canvas, random, (4, 3, 7), (6, 3, 7));
    for x in 4..=6 {
        frame.place(canvas, step, (x, 1, 4));
        frame.place(canvas, step, (x, 2, 5));
        frame.place(canvas, step, (x, 3, 6));
    }
    let o = frame.orientation as usize;
    let facing = [frame.orientation, FAR_ROW_FACING[o], LEFT_COLUMN_FACING[o], RIGHT_COLUMN_FACING[o]];
    let eyes: [bool; 12] = std::array::from_fn(|_| random.next_float() > EYE_ABOVE);
    for (index, &(x, z)) in FRAME_CELLS.iter().enumerate() {
        frame.place(canvas, portal_frame(facing[index / 3], eyes[index]), (x, 3, z));
    }
    if eyes.iter().all(|&eye| eye) {
        for z in 9..=11 {
            (4..=6).for_each(|x| frame.place(canvas, Material::EndPortal, (x, 3, z)));
        }
    }
    frame.place(canvas, Material::MobSpawner, (5, 3, 6));
}
