use super::desert::create_chest;
use super::material::Material::{self, *};
use super::piece::{Cell, Piece, turned_stairs};
use crate::shared::rng::mersenne::MersenneTwister;

const STONE_STAIRS: [Material; 4] = [StoneStairsEast, StoneStairsWest, StoneStairsSouth, StoneStairsNorth];
pub const WIDTH: i32 = 12;
pub const DEPTH: i32 = 15;
const COBBLESTONE_CHANCE: f32 = 0.4;
const SHELL: [(Cell, Cell); 13] = [
    ((0, -4, 0), (11, 0, 14)),
    ((2, 1, 2), (9, 2, 2)),
    ((2, 1, 12), (9, 2, 12)),
    ((2, 1, 3), (2, 2, 11)),
    ((9, 1, 3), (9, 2, 11)),
    ((1, 3, 1), (10, 6, 1)),
    ((1, 3, 13), (10, 6, 13)),
    ((1, 3, 2), (1, 6, 12)),
    ((10, 3, 2), (10, 6, 12)),
    ((2, 3, 2), (9, 3, 12)),
    ((2, 6, 2), (9, 6, 12)),
    ((3, 7, 3), (8, 7, 11)),
    ((4, 8, 4), (7, 8, 10)),
];
const HOLLOWS: [(Cell, Cell); 5] = [((3, 1, 3), (8, 2, 11)), ((4, 3, 6), (7, 3, 9)), ((2, 4, 2), (9, 5, 12)), ((4, 6, 5), (7, 6, 9)), ((5, 7, 6), (6, 7, 8))];
const DOORWAYS: [(Cell, Cell); 4] = [((5, 1, 2), (6, 2, 2)), ((5, 2, 12), (6, 2, 12)), ((5, 5, 1), (6, 5, 1)), ((5, 5, 13), (6, 5, 13))];

fn stone(piece: &mut Piece, random: &mut MersenneTwister, (x0, y0, z0): Cell, (x1, y1, z1): Cell) {
    for y in y0..=y1 {
        for x in x0..=x1 {
            for z in z0..=z1 {
                let block = if random.next_float() < COBBLESTONE_CHANCE { Cobblestone } else { MossyCobblestone };
                piece.place(block, (x, y, z));
            }
        }
    }
}

fn air(piece: &mut Piece, from: Cell, to: Cell) {
    piece.generate_box(from, to, Air, Air, false);
}

fn place_all(piece: &mut Piece, block: Material, cells: &[Cell]) {
    for &cell in cells {
        piece.place(block, cell);
    }
}

pub fn post_process(piece: &mut Piece, random: &mut MersenneTwister) {
    for (from, to) in SHELL {
        stone(piece, random, from, to);
    }
    for (from, to) in HOLLOWS.into_iter().chain(DOORWAYS) {
        air(piece, from, to);
    }
    for (x, z) in [(1, 5), (10, 5), (1, 9), (10, 9)] {
        piece.place(Air, (x, 5, z));
    }
    for z in [0, 14] {
        for x in [2, 4, 7, 9] {
            stone(piece, random, (x, 4, z), (x, 5, z));
        }
    }
    stone(piece, random, (5, 6, 0), (6, 6, 0));
    for x in [0, 11] {
        for z in (2..=12).step_by(2) {
            stone(piece, random, (x, 4, z), (x, 5, z));
        }
        stone(piece, random, (x, 6, 5), (x, 6, 5));
        stone(piece, random, (x, 6, 9), (x, 6, 9));
    }
    for (x, z) in [(2, 2), (9, 2), (2, 12), (9, 12)] {
        stone(piece, random, (x, 7, z), (x, 9, z));
    }
    for (x, z) in [(4, 4), (7, 4), (4, 10), (7, 10)] {
        stone(piece, random, (x, 9, z), (x, 9, z));
    }
    stone(piece, random, (5, 9, 7), (6, 9, 7));
    let [east, west, south, north] = [0, 1, 2, 3].map(|data| turned_stairs(STONE_STAIRS, data));
    for (block, x, y, z) in [(north, 5, 9, 6), (north, 6, 9, 6), (south, 5, 9, 8), (south, 6, 9, 8)] {
        piece.place(block, (x, y, z));
    }
    for x in 4..=7 {
        piece.place(north, (x, 0, 0));
    }
    for x in [4, 7] {
        for i in 0..3 {
            piece.place(north, (x, 1 + i, 8 + i));
        }
    }
    stone(piece, random, (4, 1, 9), (4, 1, 9));
    stone(piece, random, (7, 1, 9), (7, 1, 9));
    stone(piece, random, (4, 1, 10), (7, 2, 10));
    stone(piece, random, (5, 4, 5), (6, 4, 5));
    piece.place(east, (4, 4, 5));
    piece.place(west, (7, 4, 5));
    for i in 0..4 {
        piece.place(south, (5, -i, 6 + i));
        piece.place(south, (6, -i, 6 + i));
        air(piece, (5, -i, 7 + i), (6, -i, 9 + i));
    }
    air(piece, (1, -3, 12), (10, -1, 13));
    air(piece, (1, -3, 1), (3, -1, 13));
    air(piece, (1, -3, 1), (9, -1, 5));
    for z in (1..=13).step_by(2) {
        stone(piece, random, (1, -3, z), (1, -2, z));
    }
    for z in (2..=12).step_by(2) {
        stone(piece, random, (1, -1, z), (3, -1, z));
    }
    stone(piece, random, (2, -2, 1), (5, -2, 1));
    stone(piece, random, (7, -2, 1), (9, -2, 1));
    stone(piece, random, (6, -3, 1), (6, -3, 1));
    stone(piece, random, (6, -1, 1), (6, -1, 1));
    traps(piece, random);
}

fn traps(piece: &mut Piece, random: &mut MersenneTwister) {
    piece.place(TripwireHookEast, (1, -3, 8));
    piece.place(TripwireHookWest, (4, -3, 8));
    place_all(piece, TripWireEastWest, &[(2, -3, 8), (3, -3, 8)]);
    place_all(piece, RedstoneWire, &[(5, -3, 7), (5, -3, 6), (5, -3, 5), (5, -3, 4), (5, -3, 3), (5, -3, 2), (5, -3, 1), (4, -3, 1)]);
    piece.place(MossyCobblestone, (3, -3, 1));
    create_container(piece, random, (3, -2, 1), DispenserSouth);
    piece.place(VineAllSides, (3, -2, 2));
    piece.place(TripwireHookSouth, (7, -3, 1));
    piece.place(TripwireHookNorth, (7, -3, 5));
    place_all(piece, TripWireNorthSouth, &[(7, -3, 2), (7, -3, 3), (7, -3, 4)]);
    place_all(piece, RedstoneWire, &[(8, -3, 6), (9, -3, 6), (9, -3, 5)]);
    piece.place(MossyCobblestone, (9, -3, 4));
    piece.place(RedstoneWire, (9, -2, 4));
    create_container(piece, random, (9, -2, 3), DispenserWest);
    place_all(piece, VineAllSides, &[(8, -1, 3), (8, -2, 3)]);
    create_chest(piece, random, (8, -3, 3), ChestWest);
    place_all(
        piece,
        MossyCobblestone,
        &[(9, -3, 2), (8, -3, 1), (4, -3, 5), (5, -2, 5), (5, -1, 5), (6, -3, 5), (7, -2, 5), (7, -1, 5), (8, -3, 5)],
    );
    stone(piece, random, (9, -1, 1), (9, -1, 5));
    air(piece, (8, -3, 8), (10, -1, 10));
    place_all(piece, ChiseledStoneBricks, &[(8, -2, 11), (9, -2, 11), (10, -2, 11)]);
    place_all(piece, LeverSouth, &[(8, -2, 12), (9, -2, 12), (10, -2, 12)]);
    stone(piece, random, (8, -3, 8), (8, -3, 10));
    stone(piece, random, (10, -3, 8), (10, -3, 10));
    piece.place(MossyCobblestone, (10, -2, 9));
    place_all(piece, RedstoneWire, &[(8, -2, 9), (8, -2, 10), (10, -1, 9)]);
    piece.place(StickyPistonUp, (9, -2, 8));
    place_all(piece, StickyPistonWest, &[(10, -2, 8), (10, -1, 8)]);
    piece.place(RepeaterSouth, (10, -2, 10));
    create_chest(piece, random, (9, -3, 10), ChestSouth);
}

fn create_container(piece: &mut Piece, random: &mut MersenneTwister, local: Cell, block: Material) {
    let world = piece.world(local);
    if piece.contains(world) && !piece.ids.is_any(piece.block_at(world), &Material::DISPENSERS) {
        piece.set_world(world, piece.ids.id(block));
        random.next_u32();
    }
}
