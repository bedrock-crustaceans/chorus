use super::frame::{Canvas, Cell, Frame};
use super::material::Material::{Bricks, Dark, Lantern, Prismarine};

const SIZE_STEP: i32 = 9;
const PILLAR_ROWS: i32 = 7;
const PILLAR_WIDTH: i32 = 4;
const SLOPE_STEPS: i32 = 5;
const SLOPE_TOP: i32 = 23;
const SLOPE_FAR: i32 = 58;

fn deck(frame: &Frame, canvas: &mut Canvas, floor: (Cell, Cell), water: (Cell, Cell)) {
    frame.fill(canvas, Prismarine, floor.0, floor.1);
    frame.flood(canvas, water.0, water.1);
}

pub fn build(frame: &Frame, canvas: &mut Canvas) {
    wing(frame, canvas, 0, false);
    wing(frame, canvas, 33, true);
    entrance_arches(frame, canvas);
    entrance_wall(frame, canvas);
    roof(frame, canvas);
    lower_wall(frame, canvas);
    middle_wall(frame, canvas);
    upper_wall(frame, canvas);
    pillars(frame, canvas);
    slope(frame, canvas);
}

fn pillars(frame: &Frame, canvas: &mut Canvas) {
    for j in 0..PILLAR_ROWS {
        let mut k = 0;
        while k < PILLAR_ROWS {
            if k == 0 && j == 3 {
                k = 6;
            }
            let (x, z) = (j * SIZE_STEP, k * SIZE_STEP);
            for n in 0..PILLAR_WIDTH {
                for o in 0..PILLAR_WIDTH {
                    frame.place(canvas, Bricks, (x + n, 0, z + o));
                    frame.pillar_down(canvas, Bricks, (x + n, -1, z + o));
                }
            }
            k += if j == 0 || j == 6 { 1 } else { 6 };
        }
    }
}

fn slope(frame: &Frame, canvas: &mut Canvas) {
    for j in 0..SLOPE_STEPS {
        let low = j * 2;
        frame.flood(canvas, (-1 - j, low, -1 - j), (-1 - j, SLOPE_TOP, SLOPE_FAR + j));
        frame.flood(canvas, (SLOPE_FAR + j, low, -1 - j), (SLOPE_FAR + j, SLOPE_TOP, SLOPE_FAR + j));
        frame.flood(canvas, (-j, low, -1 - j), (SLOPE_FAR - 1 + j, SLOPE_TOP, -1 - j));
        frame.flood(canvas, (-j, low, SLOPE_FAR + j), (SLOPE_FAR - 1 + j, SLOPE_TOP, SLOPE_FAR + j));
    }
}

fn wing(frame: &Frame, canvas: &mut Canvas, x: i32, mirrored: bool) {
    if !frame.reaches(canvas, (x, 0), (x + 23, 20)) {
        return;
    }
    deck(frame, canvas, ((x, 0, 0), (x + 24, 0, 20)), ((x, 1, 0), (x + 24, 10, 20)));
    for i in 0..4 {
        frame.fill(canvas, Bricks, (x + i, i + 1, i), (x + i, i + 1, 20));
        frame.fill(canvas, Bricks, (x + i + 7, i + 5, i + 7), (x + i + 7, i + 5, 20));
        frame.fill(canvas, Bricks, (x + 17 - i, i + 5, i + 7), (x + 17 - i, i + 5, 20));
        frame.fill(canvas, Bricks, (x + 24 - i, i + 1, i), (x + 24 - i, i + 1, 20));
        frame.fill(canvas, Bricks, (x + i + 1, i + 1, i), (x + 23 - i, i + 1, i));
        frame.fill(canvas, Bricks, (x + i + 8, i + 5, i + 7), (x + 16 - i, i + 5, i + 7));
    }
    frame.fill(canvas, Prismarine, (x + 4, 4, 4), (x + 6, 4, 20));
    frame.fill(canvas, Prismarine, (x + 7, 4, 4), (x + 17, 4, 6));
    frame.fill(canvas, Prismarine, (x + 18, 4, 4), (x + 20, 4, 20));
    frame.fill(canvas, Prismarine, (x + 11, 8, 11), (x + 13, 8, 20));
    for z in [12, 15, 18] {
        frame.place(canvas, Bricks, (x + 12, 9, z));
    }
    let (near, far) = if mirrored { (x + 19, x + 5) } else { (x + 5, x + 19) };
    for z in (5..=20).rev().step_by(3) {
        frame.place(canvas, Bricks, (near, 5, z));
    }
    for z in (7..=19).rev().step_by(3) {
        frame.place(canvas, Bricks, (far, 5, z));
    }
    for l in 0..4 {
        let step = if mirrored { x + 24 - (17 - l * 3) } else { x + 17 - l * 3 };
        frame.place(canvas, Bricks, (step, 5, 5));
    }
    frame.place(canvas, Bricks, (far, 5, 5));
    frame.fill(canvas, Prismarine, (x + 11, 1, 12), (x + 13, 7, 12));
    frame.fill(canvas, Prismarine, (x + 12, 1, 11), (x + 12, 7, 13));
}

fn entrance_arches(frame: &Frame, canvas: &mut Canvas) {
    if !frame.reaches(canvas, (22, 5), (35, 17)) {
        return;
    }
    frame.flood(canvas, (25, 0, 0), (32, 8, 20));
    for i in 0..4 {
        let z = 5 + i * 4;
        frame.fill(canvas, Bricks, (24, 2, z), (24, 4, z));
        frame.fill(canvas, Bricks, (22, 4, z), (23, 4, z));
        frame.place(canvas, Bricks, (25, 5, z));
        frame.place(canvas, Bricks, (26, 6, z));
        frame.place(canvas, Lantern, (26, 5, z));
        frame.fill(canvas, Bricks, (33, 2, z), (33, 4, z));
        frame.fill(canvas, Bricks, (34, 4, z), (35, 4, z));
        frame.place(canvas, Bricks, (32, 5, z));
        frame.place(canvas, Bricks, (31, 6, z));
        frame.place(canvas, Lantern, (31, 5, z));
        frame.fill(canvas, Prismarine, (27, 6, z), (30, 6, z));
    }
}

const WALL_LAYERS: [(i32, i32, i32); 3] = [(21, 12, 36), (17, 11, 40), (16, 10, 41)];
const WALL_STEPS: [(i32, i32, i32); 7] = [(16, 6, 41), (17, 5, 40), (21, 4, 36), (22, 3, 26), (31, 3, 35), (23, 2, 25), (32, 2, 34)];
const DOORWAY_BRICKS: [(i32, i32); 6] = [(27, 3), (30, 3), (26, 2), (31, 2), (25, 1), (32, 1)];
const OPENING_WATER: [(i32, i32, i32); 12] = [
    (15, 6, 15),
    (16, 5, 16),
    (17, 4, 20),
    (21, 3, 21),
    (22, 2, 22),
    (23, 1, 24),
    (42, 6, 42),
    (41, 5, 41),
    (37, 4, 40),
    (36, 3, 36),
    (33, 1, 34),
    (35, 2, 35),
];

fn entrance_wall(frame: &Frame, canvas: &mut Canvas) {
    if !frame.reaches(canvas, (15, 20), (42, 21)) {
        return;
    }
    frame.fill(canvas, Prismarine, (15, 0, 21), (42, 0, 21));
    frame.flood(canvas, (26, 1, 21), (31, 3, 21));
    for (x0, y, x1) in WALL_LAYERS {
        frame.fill(canvas, Prismarine, (x0, y, 21), (x1, y, 21));
    }
    frame.fill(canvas, Prismarine, (15, 7, 21), (42, 9, 21));
    for (x0, y, x1) in WALL_STEPS {
        frame.fill(canvas, Prismarine, (x0, y, 21), (x1, y, 21));
    }
    frame.fill(canvas, Bricks, (28, 4, 20), (29, 4, 21));
    for (x, y) in DOORWAY_BRICKS {
        frame.place(canvas, Bricks, (x, y, 21));
    }
    for i in 0..7 {
        frame.place(canvas, Dark, (28 - i, 6 + i, 21));
        frame.place(canvas, Dark, (29 + i, 6 + i, 21));
    }
    for i in 0..4 {
        frame.place(canvas, Dark, (28 - i, 9 + i, 21));
        frame.place(canvas, Dark, (29 + i, 9 + i, 21));
    }
    frame.place(canvas, Dark, (28, 12, 21));
    frame.place(canvas, Dark, (29, 12, 21));
    for i in 0..3 {
        for (x, y) in [(22 - i * 2, 8), (22 - i * 2, 9), (35 + i * 2, 8), (35 + i * 2, 9)] {
            frame.place(canvas, Dark, (x, y, 21));
        }
    }
    frame.flood(canvas, (15, 13, 21), (42, 15, 21));
    for (x0, y1, x1) in OPENING_WATER {
        frame.flood(canvas, (x0, 1, 21), (x1, y1, 21));
    }
}

fn roof(frame: &Frame, canvas: &mut Canvas) {
    if !frame.reaches(canvas, (21, 21), (36, 36)) {
        return;
    }
    deck(frame, canvas, ((21, 0, 22), (36, 0, 36)), ((21, 1, 22), (36, 23, 36)));
    for i in 0..4 {
        let y = 13 + i;
        frame.fill(canvas, Bricks, (21 + i, y, 21 + i), (36 - i, y, 21 + i));
        frame.fill(canvas, Bricks, (21 + i, y, 36 - i), (36 - i, y, 36 - i));
        frame.fill(canvas, Bricks, (21 + i, y, 22 + i), (21 + i, y, 35 - i));
        frame.fill(canvas, Bricks, (36 - i, y, 22 + i), (36 - i, y, 35 - i));
    }
    frame.fill(canvas, Prismarine, (25, 16, 25), (32, 16, 32));
    for (x, z) in [(25, 25), (32, 25), (25, 32), (32, 32)] {
        frame.fill(canvas, Bricks, (x, 17, z), (x, 19, z));
    }
    for (corner, inner) in [((26, 26), (27, 27)), ((26, 31), (27, 30)), ((31, 31), (30, 30)), ((31, 26), (30, 27))] {
        frame.place(canvas, Bricks, (corner.0, 20, corner.1));
        frame.place(canvas, Bricks, (inner.0, 21, inner.1));
        frame.place(canvas, Lantern, (inner.0, 20, inner.1));
    }
    frame.fill(canvas, Prismarine, (28, 21, 27), (29, 21, 27));
    frame.fill(canvas, Prismarine, (27, 21, 28), (27, 21, 29));
    frame.fill(canvas, Prismarine, (28, 21, 30), (29, 21, 30));
    frame.fill(canvas, Prismarine, (30, 21, 28), (30, 21, 29));
}

fn lower_wall(frame: &Frame, canvas: &mut Canvas) {
    if frame.reaches(canvas, (0, 21), (6, 58)) {
        deck(frame, canvas, ((0, 0, 21), (6, 0, 57)), ((0, 1, 21), (6, 7, 57)));
        frame.fill(canvas, Prismarine, (4, 4, 21), (6, 4, 53));
        for i in 0..4 {
            frame.fill(canvas, Bricks, (i, i + 1, 21), (i, i + 1, 57 - i));
        }
        for z in (23..53).step_by(3).chain([52]) {
            frame.place(canvas, Bricks, (5, 5, z));
        }
        frame.fill(canvas, Prismarine, (4, 1, 52), (6, 3, 52));
        frame.fill(canvas, Prismarine, (5, 1, 51), (5, 3, 53));
    }
    if frame.reaches(canvas, (51, 21), (58, 58)) {
        deck(frame, canvas, ((51, 0, 21), (57, 0, 57)), ((51, 1, 21), (57, 7, 57)));
        frame.fill(canvas, Prismarine, (51, 4, 21), (53, 4, 53));
        for i in 0..4 {
            frame.fill(canvas, Bricks, (57 - i, i + 1, 21), (57 - i, i + 1, 57 - i));
        }
        for z in (23..53).step_by(3).chain([52]) {
            frame.place(canvas, Bricks, (52, 5, z));
        }
        frame.fill(canvas, Prismarine, (51, 1, 52), (53, 3, 52));
        frame.fill(canvas, Prismarine, (52, 1, 51), (52, 3, 53));
    }
    if frame.reaches(canvas, (0, 51), (57, 57)) {
        deck(frame, canvas, ((7, 0, 51), (50, 0, 57)), ((7, 1, 51), (50, 10, 57)));
        for i in 0..4 {
            frame.fill(canvas, Bricks, (i + 1, i + 1, 57 - i), (56 - i, i + 1, 57 - i));
        }
    }
}

const MIDDLE_BACK_COLUMNS: [i32; 6] = [12, 18, 24, 33, 39, 45];
const MIDDLE_BACK_ARCH: [(i32, i32); 10] = [(9, 47), (9, 50), (10, 45), (10, 46), (10, 51), (10, 52), (11, 47), (11, 50), (12, 48), (12, 49)];

fn middle_wall(frame: &Frame, canvas: &mut Canvas) {
    if frame.reaches(canvas, (7, 21), (13, 54)) {
        deck(frame, canvas, ((7, 0, 21), (13, 0, 50)), ((7, 1, 21), (13, 10, 50)));
        frame.fill(canvas, Prismarine, (11, 8, 21), (13, 8, 53));
        for i in 0..4 {
            frame.fill(canvas, Bricks, (i + 7, i + 5, 21), (i + 7, i + 5, 54));
        }
        for z in (21..=45).step_by(3) {
            frame.place(canvas, Bricks, (12, 9, z));
        }
    }
    if frame.reaches(canvas, (44, 21), (50, 54)) {
        deck(frame, canvas, ((44, 0, 21), (50, 0, 50)), ((44, 1, 21), (50, 10, 50)));
        frame.fill(canvas, Prismarine, (44, 8, 21), (46, 8, 53));
        for i in 0..4 {
            frame.fill(canvas, Bricks, (50 - i, i + 5, 21), (50 - i, i + 5, 54));
        }
        for z in (21..=45).step_by(3) {
            frame.place(canvas, Bricks, (45, 9, z));
        }
    }
    if frame.reaches(canvas, (8, 44), (49, 54)) {
        deck(frame, canvas, ((14, 0, 44), (43, 0, 50)), ((14, 1, 44), (43, 10, 50)));
        for x in (12..=45).step_by(3) {
            frame.place(canvas, Bricks, (x, 9, 45));
            frame.place(canvas, Bricks, (x, 9, 52));
            if MIDDLE_BACK_COLUMNS.contains(&x) {
                for (y, z) in MIDDLE_BACK_ARCH {
                    frame.place(canvas, Bricks, (x, y, z));
                }
            }
        }
        for i in 0..3 {
            frame.fill(canvas, Prismarine, (8 + i, 5 + i, 54), (49 - i, 5 + i, 54));
        }
        frame.fill(canvas, Bricks, (11, 8, 54), (46, 8, 54));
        frame.fill(canvas, Prismarine, (14, 8, 44), (43, 8, 53));
    }
}

fn upper_wall(frame: &Frame, canvas: &mut Canvas) {
    if frame.reaches(canvas, (14, 21), (20, 43)) {
        deck(frame, canvas, ((14, 0, 21), (20, 0, 43)), ((14, 1, 22), (20, 14, 43)));
        frame.fill(canvas, Prismarine, (18, 12, 22), (20, 12, 39));
        frame.fill(canvas, Bricks, (18, 12, 21), (20, 12, 21));
        for i in 0..4 {
            frame.fill(canvas, Bricks, (i + 14, i + 9, 21), (i + 14, i + 9, 43 - i));
        }
        for z in (23..=39).step_by(3) {
            frame.place(canvas, Bricks, (19, 13, z));
        }
    }
    if frame.reaches(canvas, (37, 21), (43, 43)) {
        deck(frame, canvas, ((37, 0, 21), (43, 0, 43)), ((37, 1, 22), (43, 14, 43)));
        frame.fill(canvas, Prismarine, (37, 12, 22), (39, 12, 39));
        frame.fill(canvas, Bricks, (37, 12, 21), (39, 12, 21));
        for i in 0..4 {
            frame.fill(canvas, Bricks, (43 - i, i + 9, 21), (43 - i, i + 9, 43 - i));
        }
        for z in (23..=39).step_by(3) {
            frame.place(canvas, Bricks, (38, 13, z));
        }
    }
    if frame.reaches(canvas, (15, 37), (42, 43)) {
        deck(frame, canvas, ((21, 0, 37), (36, 0, 43)), ((21, 1, 37), (36, 14, 43)));
        frame.fill(canvas, Prismarine, (21, 12, 37), (36, 12, 39));
        for i in 0..4 {
            frame.fill(canvas, Bricks, (15 + i, i + 9, 43 - i), (42 - i, i + 9, 43 - i));
        }
        for x in (21..=36).step_by(3) {
            frame.place(canvas, Bricks, (x, 13, 38));
        }
    }
}
