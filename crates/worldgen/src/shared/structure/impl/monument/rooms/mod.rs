use super::frame::{Canvas, Fill, Frame, fill};
use super::graph::{EAST, NORTH, WEST};
use super::material::Material::{self, Bricks, Prismarine};
use crate::shared::rng::mersenne::MersenneTwister;

mod core_room;
mod double;
mod single;

use core_room::{core, penthouse};
use double::{double_x, double_xy, double_y, double_yz, double_z};
use single::{simple, simple_top};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Room {
    Entry,
    Core,
    Simple(u8),
    SimpleTop,
    DoubleX,
    DoubleY,
    DoubleZ,
    DoubleXY,
    DoubleYZ,
    Wing(u8),
    Penthouse,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RoomState {
    pub open: [bool; 6],
    pub has_room_above: bool,
}

type Column = (i32, i32);

const LEVEL_SPLIT: usize = 25;

pub fn build(room: Room, frame: &Frame, index: usize, states: &[RoomState], canvas: &mut Canvas, random: &mut MersenneTwister) {
    let upper = index / LEVEL_SPLIT > 0;
    match room {
        Room::Entry => entry(frame, states[0], canvas),
        Room::Simple(design) => simple(frame, states[0], design, upper, canvas, random),
        Room::SimpleTop => simple_top(frame, states[0], upper, canvas, random),
        Room::DoubleY => double_y(frame, states, upper, canvas),
        Room::DoubleX => double_x(frame, states, upper, canvas),
        Room::DoubleZ => double_z(frame, states, upper, canvas),
        Room::DoubleXY => double_xy(frame, states, upper, canvas),
        Room::DoubleYZ => double_yz(frame, states, upper, canvas),
        Room::Core => core(frame, canvas),
        Room::Penthouse => penthouse(frame, canvas),
        Room::Wing(_) => {}
    }
}

fn floor(frame: &Frame, canvas: &mut Canvas, (x, z): (i32, i32), open_below: bool) {
    let mut solid = |from: (i32, i32), to: (i32, i32), material: Material| frame.fill(canvas, material, (x + from.0, 0, z + from.1), (x + to.0, 0, z + to.1));
    if !open_below {
        solid((0, 0), (7, 7), Prismarine);
        return;
    }
    solid((0, 0), (2, 7), Prismarine);
    solid((5, 0), (7, 7), Prismarine);
    solid((3, 0), (4, 2), Prismarine);
    solid((3, 5), (4, 7), Prismarine);
    solid((3, 2), (4, 2), Bricks);
    solid((3, 5), (4, 5), Bricks);
    solid((2, 3), (2, 4), Bricks);
    solid((5, 3), (5, 4), Bricks);
}

const ENTRY_BRICKS: [Fill; 9] = [
    fill(Bricks, (0, 3, 0), (2, 3, 7)),
    fill(Bricks, (5, 3, 0), (7, 3, 7)),
    fill(Bricks, (0, 2, 0), (1, 2, 7)),
    fill(Bricks, (6, 2, 0), (7, 2, 7)),
    fill(Bricks, (0, 1, 0), (0, 1, 7)),
    fill(Bricks, (7, 1, 0), (7, 1, 7)),
    fill(Bricks, (0, 1, 7), (7, 3, 7)),
    fill(Bricks, (1, 1, 0), (2, 3, 0)),
    fill(Bricks, (5, 1, 0), (6, 3, 0)),
];

fn entry(frame: &Frame, state: RoomState, canvas: &mut Canvas) {
    frame.fill_all(canvas, &ENTRY_BRICKS);
    frame.flood_where(
        canvas,
        &[
            (state.open[NORTH], (3, 1, 7), (4, 2, 7)),
            (state.open[WEST], (0, 1, 3), (1, 2, 4)),
            (state.open[EAST], (6, 1, 3), (7, 2, 4)),
        ],
    );
}
