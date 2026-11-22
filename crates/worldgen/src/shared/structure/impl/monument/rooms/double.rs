use super::super::frame::{Canvas, Cell, Fill, Frame, fill};
use super::super::graph::{DOWN, EAST, NORTH, SOUTH, WEST};
use super::super::material::Material::{self, Bricks, Dark, Lantern, Prismarine};
use super::{RoomState, floor};

const DOUBLE_Y_POSTS: [((i32, i32), (i32, i32)); 12] = [
    ((0, 0), (0, 7)),
    ((7, 0), (7, 7)),
    ((1, 0), (6, 0)),
    ((1, 7), (6, 7)),
    ((2, 1), (2, 2)),
    ((1, 2), (1, 2)),
    ((5, 1), (5, 2)),
    ((6, 2), (6, 2)),
    ((2, 5), (2, 6)),
    ((1, 5), (1, 5)),
    ((5, 5), (5, 6)),
    ((6, 5), (6, 5)),
];

pub(super) fn double_y(frame: &Frame, states: &[RoomState], upper: bool, canvas: &mut Canvas) {
    let (low, high) = (states[0], states[1]);
    if upper {
        floor(frame, canvas, (0, 0), low.open[SOUTH]);
    }
    if !high.has_room_above {
        frame.fill_over_water(canvas, Prismarine, (1, 8, 1), (6, 8, 6));
    }
    for (from, to) in DOUBLE_Y_POSTS {
        frame.fill(canvas, Bricks, (from.0, 4, from.1), (to.0, 4, to.1));
    }
    for (y, state) in [(1, low), (5, high)] {
        for (face, z) in [(SOUTH, 0), (NORTH, 7)] {
            if state.open[face] {
                frame.fill_all(
                    canvas,
                    &[
                        fill(Bricks, (2, y, z), (2, y + 2, z)),
                        fill(Bricks, (5, y, z), (5, y + 2, z)),
                        fill(Bricks, (3, y + 2, z), (4, y + 2, z)),
                    ],
                );
            } else {
                frame.fill_all(canvas, &[fill(Bricks, (0, y, z), (7, y + 2, z)), fill(Prismarine, (0, y + 1, z), (7, y + 1, z))]);
            }
        }
        for (face, x) in [(WEST, 0), (EAST, 7)] {
            if state.open[face] {
                frame.fill_all(
                    canvas,
                    &[
                        fill(Bricks, (x, y, 2), (x, y + 2, 2)),
                        fill(Bricks, (x, y, 5), (x, y + 2, 5)),
                        fill(Bricks, (x, y + 2, 3), (x, y + 2, 4)),
                    ],
                );
            } else {
                frame.fill_all(canvas, &[fill(Bricks, (x, y, 0), (x, y + 2, 7)), fill(Prismarine, (x, y + 1, 0), (x, y + 1, 7))]);
            }
        }
    }
}

pub(super) fn double_x(frame: &Frame, states: &[RoomState], upper: bool, canvas: &mut Canvas) {
    let (west, east) = (states[0], states[1]);
    if upper {
        floor(frame, canvas, (8, 0), east.open[DOWN]);
        floor(frame, canvas, (0, 0), west.open[DOWN]);
    }
    if !west.has_room_above {
        frame.fill_over_water(canvas, Prismarine, (1, 4, 1), (7, 4, 6));
    }
    if !east.has_room_above {
        frame.fill_over_water(canvas, Prismarine, (8, 4, 1), (14, 4, 6));
    }
    for (y, material) in [(3, Bricks), (2, Prismarine), (1, Bricks)] {
        frame.fill_all(
            canvas,
            &[
                fill(material, (0, y, 0), (0, y, 7)),
                fill(material, (15, y, 0), (15, y, 7)),
                fill(material, (1, y, 0), (15, y, 0)),
                fill(material, (1, y, 7), (14, y, 7)),
            ],
        );
    }
    frame.fill_all(
        canvas,
        &[fill(Bricks, (5, 1, 0), (10, 1, 4)), fill(Prismarine, (6, 2, 0), (9, 2, 3)), fill(Bricks, (5, 3, 0), (10, 3, 4))],
    );
    frame.place(canvas, Lantern, (6, 2, 3));
    frame.place(canvas, Lantern, (9, 2, 3));
    frame.flood_where(
        canvas,
        &[
            (west.open[SOUTH], (3, 1, 0), (4, 2, 0)),
            (west.open[NORTH], (3, 1, 7), (4, 2, 7)),
            (west.open[WEST], (0, 1, 3), (0, 2, 4)),
            (east.open[SOUTH], (11, 1, 0), (12, 2, 0)),
            (east.open[NORTH], (11, 1, 7), (12, 2, 7)),
            (east.open[EAST], (15, 1, 3), (15, 2, 4)),
        ],
    );
}

pub(super) fn layered(y: i32) -> Material {
    if y == 2 || y == 6 { Prismarine } else { Bricks }
}

const DOUBLE_Z_BRICKS: [Fill; 16] = [
    fill(Bricks, (1, 1, 1), (1, 1, 2)),
    fill(Bricks, (6, 1, 1), (6, 1, 2)),
    fill(Bricks, (1, 3, 1), (1, 3, 2)),
    fill(Bricks, (6, 3, 1), (6, 3, 2)),
    fill(Bricks, (1, 1, 13), (1, 1, 14)),
    fill(Bricks, (6, 1, 13), (6, 1, 14)),
    fill(Bricks, (1, 3, 13), (1, 3, 14)),
    fill(Bricks, (6, 3, 13), (6, 3, 14)),
    fill(Bricks, (2, 1, 6), (2, 3, 6)),
    fill(Bricks, (5, 1, 6), (5, 3, 6)),
    fill(Bricks, (2, 1, 9), (2, 3, 9)),
    fill(Bricks, (5, 1, 9), (5, 3, 9)),
    fill(Bricks, (3, 2, 6), (4, 2, 6)),
    fill(Bricks, (3, 2, 9), (4, 2, 9)),
    fill(Bricks, (2, 2, 7), (2, 2, 8)),
    fill(Bricks, (5, 2, 7), (5, 2, 8)),
];

const DOUBLE_Z_LAMPS: [(i32, i32); 4] = [(2, 5), (5, 5), (2, 10), (5, 10)];

pub(super) fn double_z(frame: &Frame, states: &[RoomState], upper: bool, canvas: &mut Canvas) {
    let (south, north) = (states[0], states[1]);
    if upper {
        floor(frame, canvas, (0, 8), north.open[DOWN]);
        floor(frame, canvas, (0, 0), south.open[DOWN]);
    }
    if !south.has_room_above {
        frame.fill_over_water(canvas, Prismarine, (1, 4, 1), (6, 4, 7));
    }
    if !north.has_room_above {
        frame.fill_over_water(canvas, Prismarine, (1, 4, 8), (6, 4, 14));
    }
    for (y, material) in [(3, Bricks), (2, Prismarine), (1, Bricks)] {
        frame.fill_all(
            canvas,
            &[
                fill(material, (0, y, 0), (0, y, 15)),
                fill(material, (7, y, 0), (7, y, 15)),
                fill(material, (1, y, 0), (7, y, 0)),
                fill(material, (1, y, 15), (6, y, 15)),
            ],
        );
    }
    frame.fill_all(canvas, &DOUBLE_Z_BRICKS);
    for (x, z) in DOUBLE_Z_LAMPS {
        frame.place(canvas, Lantern, (x, 2, z));
    }
    for (x, z) in DOUBLE_Z_LAMPS {
        frame.place(canvas, Bricks, (x, 3, z));
    }
    frame.flood_where(
        canvas,
        &[
            (south.open[SOUTH], (3, 1, 0), (4, 2, 0)),
            (south.open[EAST], (7, 1, 3), (7, 2, 4)),
            (south.open[WEST], (0, 1, 3), (0, 2, 4)),
            (north.open[NORTH], (3, 1, 15), (4, 2, 15)),
            (north.open[WEST], (0, 1, 11), (0, 2, 12)),
            (north.open[EAST], (7, 1, 11), (7, 2, 12)),
        ],
    );
}

const DOUBLE_XY_BRICKS: [Fill; 13] = [
    fill(Bricks, (2, 1, 3), (2, 7, 4)),
    fill(Bricks, (3, 1, 2), (4, 7, 2)),
    fill(Bricks, (3, 1, 5), (4, 7, 5)),
    fill(Bricks, (13, 1, 3), (13, 7, 4)),
    fill(Bricks, (11, 1, 2), (12, 7, 2)),
    fill(Bricks, (11, 1, 5), (12, 7, 5)),
    fill(Bricks, (5, 1, 3), (5, 3, 4)),
    fill(Bricks, (10, 1, 3), (10, 3, 4)),
    fill(Bricks, (5, 7, 2), (10, 7, 5)),
    fill(Bricks, (5, 5, 2), (5, 7, 2)),
    fill(Bricks, (10, 5, 2), (10, 7, 2)),
    fill(Bricks, (5, 5, 5), (5, 7, 5)),
    fill(Bricks, (10, 5, 5), (10, 7, 5)),
];

pub(super) fn double_xy(frame: &Frame, states: &[RoomState], upper: bool, canvas: &mut Canvas) {
    let [low, east, high, east_high] = [states[0], states[1], states[2], states[3]];
    if upper {
        floor(frame, canvas, (8, 0), east.open[DOWN]);
        floor(frame, canvas, (0, 0), low.open[DOWN]);
    }
    if !high.has_room_above {
        frame.fill_over_water(canvas, Prismarine, (1, 8, 1), (7, 8, 6));
    }
    if !east_high.has_room_above {
        frame.fill_over_water(canvas, Prismarine, (8, 8, 1), (14, 8, 6));
    }
    for y in 1..=7 {
        let material = layered(y);
        frame.fill_all(
            canvas,
            &[
                fill(material, (0, y, 0), (0, y, 7)),
                fill(material, (15, y, 0), (15, y, 7)),
                fill(material, (1, y, 0), (15, y, 0)),
                fill(material, (1, y, 7), (14, y, 7)),
            ],
        );
    }
    frame.fill_all(canvas, &DOUBLE_XY_BRICKS);
    for (x, z) in [(6, 2), (9, 2), (6, 5), (9, 5)] {
        frame.place(canvas, Bricks, (x, 6, z));
    }
    frame.fill(canvas, Bricks, (5, 4, 3), (6, 4, 4));
    frame.fill(canvas, Bricks, (9, 4, 3), (10, 4, 4));
    for (x, z) in [(5, 2), (5, 5), (10, 2), (10, 5)] {
        frame.place(canvas, Lantern, (x, 4, z));
    }
    for (y, west_state, east_state) in [(1, low, east), (5, high, east_high)] {
        frame.flood_where(
            canvas,
            &[
                (west_state.open[SOUTH], (3, y, 0), (4, y + 1, 0)),
                (west_state.open[NORTH], (3, y, 7), (4, y + 1, 7)),
                (west_state.open[WEST], (0, y, 3), (0, y + 1, 4)),
                (east_state.open[SOUTH], (11, y, 0), (12, y + 1, 0)),
                (east_state.open[NORTH], (11, y, 7), (12, y + 1, 7)),
                (east_state.open[EAST], (15, y, 3), (15, y + 1, 4)),
            ],
        );
    }
}

struct UpperOpening {
    face: usize,
    north_half: bool,
    water: (Cell, Cell),
    bricks: &'static [(Cell, Cell)],
}

const UPPER_OPENINGS: [UpperOpening; 5] = [
    UpperOpening {
        face: EAST,
        north_half: false,
        water: ((7, 5, 3), (7, 6, 4)),
        bricks: &[((5, 4, 2), (6, 4, 5)), ((6, 1, 2), (6, 3, 2)), ((6, 1, 5), (6, 3, 5))],
    },
    UpperOpening {
        face: WEST,
        north_half: false,
        water: ((0, 5, 3), (0, 6, 4)),
        bricks: &[((1, 4, 2), (2, 4, 5)), ((1, 1, 2), (1, 3, 2)), ((1, 1, 5), (1, 3, 5))],
    },
    UpperOpening {
        face: NORTH,
        north_half: true,
        water: ((3, 5, 15), (4, 6, 15)),
        bricks: &[],
    },
    UpperOpening {
        face: WEST,
        north_half: true,
        water: ((0, 5, 11), (0, 6, 12)),
        bricks: &[((1, 4, 10), (2, 4, 13)), ((1, 1, 10), (1, 3, 10)), ((1, 1, 13), (1, 3, 13))],
    },
    UpperOpening {
        face: EAST,
        north_half: true,
        water: ((7, 5, 11), (7, 6, 12)),
        bricks: &[((5, 4, 10), (6, 4, 13)), ((6, 1, 10), (6, 3, 10)), ((6, 1, 13), (6, 3, 13))],
    },
];

pub(super) fn double_yz(frame: &Frame, states: &[RoomState], upper: bool, canvas: &mut Canvas) {
    let [low, north, high, north_high] = [states[0], states[1], states[2], states[3]];
    if upper {
        floor(frame, canvas, (0, 8), north.open[DOWN]);
        floor(frame, canvas, (0, 0), low.open[DOWN]);
    }
    if !high.has_room_above {
        frame.fill_over_water(canvas, Prismarine, (1, 8, 1), (6, 8, 7));
    }
    if !north_high.has_room_above {
        frame.fill_over_water(canvas, Prismarine, (1, 8, 8), (6, 8, 14));
    }
    for y in 1..=7 {
        let material = layered(y);
        frame.fill_all(
            canvas,
            &[
                fill(material, (0, y, 0), (0, y, 15)),
                fill(material, (7, y, 0), (7, y, 15)),
                fill(material, (1, y, 0), (6, y, 0)),
                fill(material, (1, y, 15), (6, y, 15)),
            ],
        );
    }
    for y in 1..=7 {
        frame.fill(canvas, if y == 2 || y == 6 { Lantern } else { Dark }, (3, y, 7), (4, y, 8));
    }
    frame.flood_where(
        canvas,
        &[
            (low.open[SOUTH], (3, 1, 0), (4, 2, 0)),
            (low.open[EAST], (7, 1, 3), (7, 2, 4)),
            (low.open[WEST], (0, 1, 3), (0, 2, 4)),
            (north.open[NORTH], (3, 1, 15), (4, 2, 15)),
            (north.open[WEST], (0, 1, 11), (0, 2, 12)),
            (north.open[EAST], (7, 1, 11), (7, 2, 12)),
            (high.open[SOUTH], (3, 5, 0), (4, 6, 0)),
        ],
    );
    for opening in &UPPER_OPENINGS {
        let state = if opening.north_half { north_high } else { high };
        if state.open[opening.face] {
            frame.flood(canvas, opening.water.0, opening.water.1);
            for &(from, to) in opening.bricks {
                frame.fill(canvas, Bricks, from, to);
            }
        }
    }
}
