use super::frame::{Canvas, Fill, Frame, fill};
use super::material::Material::{Bricks, Dark, Lantern};

const FIRST_BRICKS: [Fill; 10] = [
    fill(Bricks, (7, 0, 6), (15, 0, 16)),
    fill(Bricks, (6, 0, 6), (6, 3, 20)),
    fill(Bricks, (16, 0, 6), (16, 3, 20)),
    fill(Bricks, (7, 1, 7), (7, 1, 20)),
    fill(Bricks, (15, 1, 7), (15, 1, 20)),
    fill(Bricks, (7, 1, 6), (9, 3, 6)),
    fill(Bricks, (13, 1, 6), (15, 3, 6)),
    fill(Bricks, (8, 1, 7), (9, 1, 7)),
    fill(Bricks, (13, 1, 7), (14, 1, 7)),
    fill(Bricks, (9, 0, 5), (13, 0, 5)),
];
const FIRST_DARK: [Fill; 3] = [fill(Dark, (10, 0, 7), (12, 0, 7)), fill(Dark, (8, 0, 10), (8, 0, 12)), fill(Dark, (14, 0, 10), (14, 0, 12))];
const FIRST_LAMPS: [(i32, i32, i32); 6] = [(10, 0, 10), (12, 0, 10), (10, 0, 12), (12, 0, 12), (8, 3, 6), (14, 3, 6)];
const FIRST_CORNERS: [(i32, i32); 4] = [(4, 4), (18, 4), (4, 18), (18, 18)];
const FIRST_LINTELS: [(i32, i32, i32); 2] = [(9, 7, 20), (13, 7, 20)];
const FIRST_PILLARS: [Fill; 2] = [fill(Bricks, (6, 0, 21), (7, 4, 21)), fill(Bricks, (15, 0, 21), (16, 4, 21))];

const SECOND_ARCH_COLUMNS: [i32; 2] = [9, 13];
const SECOND_POSTS: [i32; 2] = [10, 12];
const SECOND_FEET: [i32; 2] = [8, 14];

fn first_design(frame: &Frame, canvas: &mut Canvas) {
    for i in 0..4 {
        frame.fill(canvas, Bricks, (10 - i, 3 - i, 20 - i), (12 + i, 3 - i, 20));
    }
    frame.fill_all(canvas, &FIRST_BRICKS);
    frame.fill_all(canvas, &FIRST_DARK);
    for z in (7..=18).rev().step_by(3) {
        frame.place(canvas, Lantern, (6, 3, z));
        frame.place(canvas, Lantern, (16, 3, z));
    }
    for lamp in FIRST_LAMPS {
        frame.place(canvas, Lantern, lamp);
    }
    for (x, z) in FIRST_CORNERS {
        frame.place(canvas, Bricks, (x, 2, z));
        frame.place(canvas, Lantern, (x, 1, z));
        frame.place(canvas, Bricks, (x, 0, z));
    }
    for lintel in FIRST_LINTELS {
        frame.place(canvas, Bricks, lintel);
    }
    frame.fill_all(canvas, &FIRST_PILLARS);
}

fn second_design(frame: &Frame, canvas: &mut Canvas) {
    frame.fill(canvas, Bricks, (9, 3, 18), (13, 3, 20));
    frame.fill(canvas, Bricks, (9, 0, 18), (9, 2, 18));
    frame.fill(canvas, Bricks, (13, 0, 18), (13, 2, 18));
    for x in SECOND_ARCH_COLUMNS {
        frame.place(canvas, Bricks, (x, 6, 20));
        frame.place(canvas, Lantern, (x, 5, 20));
        frame.place(canvas, Bricks, (x, 4, 20));
    }
    frame.fill(canvas, Bricks, (7, 3, 7), (15, 3, 14));
    for x in SECOND_POSTS {
        frame.fill(canvas, Bricks, (x, 0, 10), (x, 6, 10));
        frame.fill(canvas, Bricks, (x, 0, 12), (x, 6, 12));
        for (y, z) in [(0, 10), (0, 12), (4, 10), (4, 12)] {
            frame.place(canvas, Lantern, (x, y, z));
        }
    }
    for x in SECOND_FEET {
        frame.fill(canvas, Bricks, (x, 0, 7), (x, 2, 7));
        frame.fill(canvas, Bricks, (x, 0, 14), (x, 2, 14));
    }
    frame.fill(canvas, Dark, (8, 3, 8), (8, 3, 13));
    frame.fill(canvas, Dark, (14, 3, 8), (14, 3, 13));
}

pub fn build(frame: &Frame, design: u8, canvas: &mut Canvas) {
    if design == 0 {
        first_design(frame, canvas);
    } else {
        second_design(frame, canvas);
    }
}
