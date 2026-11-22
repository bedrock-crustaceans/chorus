use super::super::frame::{Canvas, Fill, Frame, fill};
use super::super::material::Material::{Bricks, Dark, Gold, Lantern, Prismarine};
use super::double::layered;

const CORE_ENDS: [i32; 2] = [0, 15];

const CORE_BRICKS: [Fill; 28] = [
    fill(Bricks, (5, 1, 6), (5, 2, 6)),
    fill(Bricks, (5, 1, 9), (5, 2, 9)),
    fill(Bricks, (10, 1, 6), (10, 2, 6)),
    fill(Bricks, (10, 1, 9), (10, 2, 9)),
    fill(Bricks, (6, 1, 5), (6, 2, 5)),
    fill(Bricks, (9, 1, 5), (9, 2, 5)),
    fill(Bricks, (6, 1, 10), (6, 2, 10)),
    fill(Bricks, (9, 1, 10), (9, 2, 10)),
    fill(Bricks, (5, 2, 5), (5, 6, 5)),
    fill(Bricks, (5, 2, 10), (5, 6, 10)),
    fill(Bricks, (10, 2, 5), (10, 6, 5)),
    fill(Bricks, (10, 2, 10), (10, 6, 10)),
    fill(Bricks, (5, 7, 1), (5, 7, 6)),
    fill(Bricks, (10, 7, 1), (10, 7, 6)),
    fill(Bricks, (5, 7, 9), (5, 7, 14)),
    fill(Bricks, (10, 7, 9), (10, 7, 14)),
    fill(Bricks, (1, 7, 5), (6, 7, 5)),
    fill(Bricks, (1, 7, 10), (6, 7, 10)),
    fill(Bricks, (9, 7, 5), (14, 7, 5)),
    fill(Bricks, (9, 7, 10), (14, 7, 10)),
    fill(Bricks, (2, 1, 2), (2, 1, 3)),
    fill(Bricks, (3, 1, 2), (3, 1, 2)),
    fill(Bricks, (13, 1, 2), (13, 1, 3)),
    fill(Bricks, (12, 1, 2), (12, 1, 2)),
    fill(Bricks, (2, 1, 12), (2, 1, 13)),
    fill(Bricks, (3, 1, 13), (3, 1, 13)),
    fill(Bricks, (13, 1, 12), (13, 1, 13)),
    fill(Bricks, (12, 1, 13), (12, 1, 13)),
];

pub(super) fn core(frame: &Frame, canvas: &mut Canvas) {
    frame.fill_over_water(canvas, Prismarine, (1, 8, 0), (14, 8, 14));
    frame.fill_all(
        canvas,
        &[
            fill(Bricks, (0, 7, 0), (0, 7, 15)),
            fill(Bricks, (15, 7, 0), (15, 7, 15)),
            fill(Bricks, (1, 7, 0), (15, 7, 0)),
            fill(Bricks, (1, 7, 15), (14, 7, 15)),
        ],
    );
    for y in 1..=6 {
        let material = layered(y);
        for x in CORE_ENDS {
            frame.fill_all(
                canvas,
                &[fill(material, (x, y, 0), (x, y, 1)), fill(material, (x, y, 6), (x, y, 9)), fill(material, (x, y, 14), (x, y, 15))],
            );
        }
        frame.fill_all(
            canvas,
            &[
                fill(material, (1, y, 0), (1, y, 0)),
                fill(material, (6, y, 0), (9, y, 0)),
                fill(material, (14, y, 0), (14, y, 0)),
                fill(material, (1, y, 15), (14, y, 15)),
            ],
        );
    }
    frame.fill(canvas, Dark, (6, 3, 6), (9, 6, 9));
    frame.fill(canvas, Gold, (7, 4, 7), (8, 5, 8));
    for y in [3, 6] {
        for x in [6, 9] {
            frame.place(canvas, Lantern, (x, y, 6));
            frame.place(canvas, Lantern, (x, y, 9));
        }
    }
    frame.fill_all(canvas, &CORE_BRICKS);
}

pub(super) fn penthouse(frame: &Frame, canvas: &mut Canvas) {
    frame.fill(canvas, Bricks, (2, -1, 2), (11, -1, 11));
    frame.fill_all(
        canvas,
        &[
            fill(Prismarine, (0, -1, 0), (1, -1, 11)),
            fill(Prismarine, (12, -1, 0), (13, -1, 11)),
            fill(Prismarine, (2, -1, 0), (11, -1, 1)),
            fill(Prismarine, (2, -1, 12), (11, -1, 13)),
            fill(Bricks, (0, 0, 0), (0, 0, 13)),
            fill(Bricks, (13, 0, 0), (13, 0, 13)),
            fill(Bricks, (1, 0, 0), (12, 0, 0)),
            fill(Bricks, (1, 0, 13), (12, 0, 13)),
        ],
    );
    for i in (2..=11).step_by(3) {
        frame.place(canvas, Lantern, (0, 0, i));
        frame.place(canvas, Lantern, (13, 0, i));
        frame.place(canvas, Lantern, (i, 0, 0));
    }
    frame.fill_all(canvas, &[fill(Bricks, (2, 0, 3), (4, 0, 9)), fill(Bricks, (9, 0, 3), (11, 0, 9)), fill(Bricks, (4, 0, 9), (9, 0, 11))]);
    for (x, z) in [(5, 8), (8, 8), (10, 10), (3, 10)] {
        frame.place(canvas, Bricks, (x, 0, z));
    }
    frame.fill_all(canvas, &[fill(Dark, (3, 0, 3), (3, 0, 7)), fill(Dark, (10, 0, 3), (10, 0, 7)), fill(Dark, (6, 0, 10), (7, 0, 10))]);
    for x in [3, 10] {
        for z in (2..=8).step_by(3) {
            frame.fill(canvas, Bricks, (x, 0, z), (x, 2, z));
        }
    }
    frame.fill_all(
        canvas,
        &[fill(Bricks, (5, 0, 10), (5, 2, 10)), fill(Bricks, (8, 0, 10), (8, 2, 10)), fill(Dark, (6, -1, 7), (7, -1, 8))],
    );
    frame.flood(canvas, (6, -1, 3), (7, -1, 4));
}
