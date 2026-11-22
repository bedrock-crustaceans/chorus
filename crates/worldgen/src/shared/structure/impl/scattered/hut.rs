use super::material::Material::{self, *};
use super::piece::{Piece, turned_stairs};
use crate::shared::rng::mersenne::MersenneTwister;

const SPRUCE_STAIRS: [Material; 4] = [SpruceStairsEast, SpruceStairsWest, SpruceStairsSouth, SpruceStairsNorth];
pub const WIDTH: i32 = 7;
pub const DEPTH: i32 = 9;
const CAULDRON_LEVELS: i32 = 3;
const POTION_RANGE: i32 = 100;
const POSTS: [(i32, i32); 4] = [(1, 2), (5, 2), (1, 7), (5, 7)];
type Span = ((i32, i32, i32), (i32, i32, i32));

const PLANK_BOXES: [Span; 7] = [
    ((1, 1, 1), (5, 1, 7)),
    ((1, 5, 2), (5, 5, 7)),
    ((2, 1, 0), (4, 1, 0)),
    ((2, 2, 2), (3, 4, 2)),
    ((1, 2, 3), (1, 4, 6)),
    ((5, 2, 3), (5, 4, 6)),
    ((2, 2, 7), (4, 4, 7)),
];

pub fn post_process(piece: &mut Piece, random: &mut MersenneTwister) {
    piece.generate_box((0, 2, 0), (6, 3, 8), Air, Air, true);
    for (from, to) in PLANK_BOXES {
        piece.generate_box(from, to, SprucePlanks, SprucePlanks, false);
    }
    for (x, z) in POSTS {
        piece.generate_box((x, 0, z), (x, 4, z), OakLog, OakLog, false);
    }
    piece.place(OakFence, (2, 3, 2));
    piece.place(OakFence, (3, 3, 7));
    piece.place(Air, (1, 3, 4));
    piece.place(Air, (5, 3, 4));
    piece.place(Air, (5, 3, 5));
    piece.place(FlowerPot, (1, 3, 5));
    piece.place(SprucePlanks, (4, 4, 2));
    piece.place(CraftingTable, (3, 2, 6));
    let cauldron = piece.world((4, 2, 6));
    if piece.contains(cauldron) && !piece.ids.is_any(piece.block_at(cauldron), &Material::CAULDRONS) {
        let level = Material::CAULDRONS[random.next_int_bounded(CAULDRON_LEVELS) as usize];
        piece.set_world(cauldron, piece.ids.id(level));
        random.next_int_bounded(POTION_RANGE);
    }
    piece.place(OakFence, (1, 2, 1));
    piece.place(OakFence, (5, 2, 1));
    let [east, west, south, north] = [0, 1, 2, 3].map(|data| turned_stairs(SPRUCE_STAIRS, data));
    piece.generate_box((0, 5, 1), (6, 5, 1), north, north, false);
    piece.generate_box((0, 5, 2), (0, 5, 7), east, east, false);
    piece.generate_box((6, 5, 2), (6, 5, 7), west, west, false);
    piece.generate_box((0, 5, 8), (6, 5, 8), south, south, false);
    for (x, z) in POSTS {
        piece.fill_column_down(OakLog, (x, -1, z));
    }
}
