use super::super::frame::{Canvas, Cell, Frame, fill};
use super::super::graph::{DOWN, EAST, NORTH, SOUTH, UP, WEST};
use super::super::material::Material::{Bricks, Dark, Lantern, Prismarine, WetSponge};
use super::{Column, RoomState, floor};
use crate::shared::rng::mersenne::MersenneTwister;

const PILLAR_DESIGN_FIRST: u8 = 1;

const SPONGE_ODDS: i32 = 3;

const SPONGE_LOW_ODDS: i32 = 4;

fn ceiling(frame: &Frame, state: RoomState, canvas: &mut Canvas, upper: bool) {
    if upper {
        floor(frame, canvas, (0, 0), state.open[DOWN]);
    }
    if !state.has_room_above {
        frame.fill_over_water(canvas, Prismarine, (1, 4, 1), (6, 4, 6));
    }
}

const CORNER_WALLS: [(Cell, Cell, i32, i32, i32, i32); 4] = [
    ((0, 1, 0), (2, 1, 2), 0, 0, 1, 1),
    ((5, 1, 0), (7, 1, 2), 7, 0, 6, 1),
    ((0, 1, 5), (2, 1, 7), 0, 7, 1, 6),
    ((5, 1, 5), (7, 1, 7), 7, 7, 6, 6),
];

struct Side {
    face: usize,
    lintel: (Cell, Cell),
    wall: (Cell, Cell),
    band: (Cell, Cell),
}

const SIDES: [Side; 4] = [
    Side {
        face: SOUTH,
        lintel: ((3, 3, 0), (4, 3, 0)),
        wall: ((3, 3, 0), (4, 3, 1)),
        band: ((3, 2, 0), (4, 2, 0)),
    },
    Side {
        face: NORTH,
        lintel: ((3, 3, 7), (4, 3, 7)),
        wall: ((3, 3, 6), (4, 3, 7)),
        band: ((3, 2, 7), (4, 2, 7)),
    },
    Side {
        face: WEST,
        lintel: ((0, 3, 3), (0, 3, 4)),
        wall: ((0, 3, 3), (1, 3, 4)),
        band: ((0, 2, 3), (0, 2, 4)),
    },
    Side {
        face: EAST,
        lintel: ((7, 3, 3), (7, 3, 4)),
        wall: ((6, 3, 3), (7, 3, 4)),
        band: ((7, 2, 3), (7, 2, 4)),
    },
];

const POSTS: [(i32, i32); 4] = [(2, 2), (2, 5), (5, 5), (5, 2)];

const POST_FEET: [((i32, i32), (i32, i32)); 8] = [
    ((0, 0), (1, 0)),
    ((0, 1), (0, 1)),
    ((0, 7), (1, 7)),
    ((0, 6), (0, 6)),
    ((6, 7), (7, 7)),
    ((7, 6), (7, 6)),
    ((6, 0), (7, 0)),
    ((7, 1), (7, 1)),
];

const POST_BANDS: [(i32, i32); 8] = [(1, 0), (0, 1), (1, 7), (0, 6), (6, 7), (7, 6), (6, 0), (7, 1)];

const LONG_WALLS: [(usize, Column, Column); 4] = [(SOUTH, (1, 0), (6, 0)), (NORTH, (1, 7), (6, 7)), (WEST, (0, 1), (0, 6)), (EAST, (7, 1), (7, 6))];

const DOORS: [(usize, Cell, Cell); 4] = [(SOUTH, (3, 1, 0), (4, 2, 0)), (NORTH, (3, 1, 7), (4, 2, 7)), (WEST, (0, 1, 3), (0, 2, 4)), (EAST, (7, 1, 3), (7, 2, 4))];

fn banded_ring(frame: &Frame, canvas: &mut Canvas) {
    for (y, material) in [(1, Bricks), (2, Dark), (3, Bricks)] {
        frame.fill_all(
            canvas,
            &[
                fill(material, (0, y, 0), (0, y, 7)),
                fill(material, (7, y, 0), (7, y, 7)),
                fill(material, (1, y, 0), (6, y, 0)),
                fill(material, (1, y, 7), (6, y, 7)),
            ],
        );
    }
    for (_, from, to) in DOORS {
        frame.fill(canvas, Dark, from, to);
    }
}

pub(super) fn simple(frame: &Frame, state: RoomState, design: u8, upper: bool, canvas: &mut Canvas, random: &mut MersenneTwister) {
    ceiling(frame, state, canvas, upper);
    let open = state.open;
    let doorways = [NORTH, SOUTH, WEST, EAST].iter().filter(|&&face| open[face]).count();
    let pillar = design >= PILLAR_DESIGN_FIRST && random.next_boolean() && !open[DOWN] && !open[UP] && doorways > 1;
    match design {
        0 => {
            for (low, high, edge_x, edge_z, lamp_x, lamp_z) in CORNER_WALLS {
                frame.fill(canvas, Bricks, low, high);
                frame.fill(canvas, Bricks, (low.0, 3, low.2), (high.0, 3, high.2));
                frame.fill(canvas, Prismarine, (edge_x, 2, low.2), (edge_x, 2, high.2));
                let (from_x, to_x) = if edge_x == 0 { (1, 2) } else { (5, 6) };
                frame.fill(canvas, Prismarine, (from_x, 2, edge_z), (to_x, 2, edge_z));
                frame.place(canvas, Lantern, (lamp_x, 2, lamp_z));
            }
            for side in SIDES {
                if open[side.face] {
                    frame.fill(canvas, Bricks, side.lintel.0, side.lintel.1);
                } else {
                    frame.fill(canvas, Bricks, side.wall.0, side.wall.1);
                    frame.fill(canvas, Prismarine, side.band.0, side.band.1);
                    frame.fill(canvas, Bricks, (side.wall.0.0, 1, side.wall.0.2), (side.wall.1.0, 1, side.wall.1.2));
                }
            }
        }
        1 => {
            for (x, z) in POSTS {
                frame.fill(canvas, Bricks, (x, 1, z), (x, 3, z));
            }
            for (x, z) in POSTS {
                frame.place(canvas, Lantern, (x, 2, z));
            }
            for (from, to) in POST_FEET {
                frame.fill(canvas, Bricks, (from.0, 1, from.1), (to.0, 3, to.1));
            }
            for (x, z) in POST_BANDS {
                frame.place(canvas, Prismarine, (x, 2, z));
            }
            for (face, from, to) in LONG_WALLS {
                if !open[face] {
                    frame.fill(canvas, Bricks, (from.0, 3, from.1), (to.0, 3, to.1));
                    frame.fill(canvas, Prismarine, (from.0, 2, from.1), (to.0, 2, to.1));
                    frame.fill(canvas, Bricks, (from.0, 1, from.1), (to.0, 1, to.1));
                }
            }
        }
        _ => {
            banded_ring(frame, canvas);
            for (face, from, to) in DOORS {
                if open[face] {
                    frame.flood(canvas, from, to);
                }
            }
        }
    }
    if pillar {
        frame.fill(canvas, Bricks, (3, 1, 3), (4, 1, 4));
        frame.fill(canvas, Prismarine, (3, 2, 3), (4, 2, 4));
        frame.fill(canvas, Bricks, (3, 3, 3), (4, 3, 4));
    }
}

pub(super) fn simple_top(frame: &Frame, state: RoomState, upper: bool, canvas: &mut Canvas, random: &mut MersenneTwister) {
    ceiling(frame, state, canvas, upper);
    for x in 1..=6 {
        for z in 1..=6 {
            if random.next_int_bounded(SPONGE_ODDS) != 0 {
                let y = 2 + i32::from(random.next_int_bounded(SPONGE_LOW_ODDS) != 0);
                frame.fill(canvas, WetSponge, (x, y, z), (x, 3, z));
            }
        }
    }
    banded_ring(frame, canvas);
    frame.flood_where(canvas, &[(state.open[SOUTH], (3, 1, 0), (4, 2, 0))]);
}
