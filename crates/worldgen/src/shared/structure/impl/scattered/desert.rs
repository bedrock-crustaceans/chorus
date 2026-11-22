use super::cellar;
use super::material::Material::{self, *};
use super::piece::{Cell, Piece, turned_stairs};
use crate::shared::rng::mersenne::MersenneTwister;

const SANDSTONE_STAIRS: [Material; 4] = [SandstoneStairsEast, SandstoneStairsWest, SandstoneStairsSouth, SandstoneStairsNorth];
pub const WIDTH: i32 = 21;
pub const DEPTH: i32 = 21;
const TOWER_ROWS: [[Material; 3]; 7] = [
    [CutSandstone, OrangeTerracotta, CutSandstone],
    [CutSandstone, OrangeTerracotta, CutSandstone],
    [OrangeTerracotta, ChiseledSandstone, OrangeTerracotta],
    [CutSandstone, OrangeTerracotta, CutSandstone],
    [OrangeTerracotta, ChiseledSandstone, OrangeTerracotta],
    [OrangeTerracotta, OrangeTerracotta, OrangeTerracotta],
    [CutSandstone, CutSandstone, CutSandstone],
];
const CUT_DOORWAY: [(i32, i32); 7] = [(9, 1), (9, 2), (9, 3), (10, 3), (11, 3), (11, 2), (11, 1)];
const PLATFORM_ORANGE: [(i32, i32); 12] = [(10, 7), (10, 8), (9, 9), (11, 9), (8, 10), (12, 10), (7, 10), (13, 10), (9, 11), (11, 11), (10, 12), (10, 13)];
const LEDGE_AIR: [(i32, i32); 6] = [(5, 5), (5, 6), (6, 6), (WIDTH - 6, 5), (WIDTH - 6, 6), (WIDTH - 7, 6)];

fn fill(piece: &mut Piece, from: Cell, to: Cell, edge: Material, inside: Material) {
    piece.generate_box(from, to, edge, inside, false);
}

pub fn post_process(piece: &mut Piece, random: &mut MersenneTwister) {
    let (w, d) = (WIDTH, DEPTH);
    fill(piece, (0, -4, 0), (w - 1, 0, d - 1), Sandstone, Sandstone);
    for i in 1..=9 {
        fill(piece, (i, i, i), (w - 1 - i, i, d - 1 - i), Sandstone, Sandstone);
        fill(piece, (i + 1, i, i + 1), (w - 2 - i, i, d - 2 - i), Air, Air);
    }
    for x in 0..w {
        for z in 0..d {
            piece.fill_column_down(Sandstone, (x, -5, z));
        }
    }
    let [east, west, south, north] = [0, 1, 2, 3].map(|data| turned_stairs(SANDSTONE_STAIRS, data));
    for x in [0, w - 5] {
        fill(piece, (x, 0, 0), (x + 4, 9, 4), Sandstone, Air);
        fill(piece, (x + 1, 10, 1), (x + 3, 10, 3), Sandstone, Sandstone);
        piece.place(north, (x + 2, 10, 0));
        piece.place(south, (x + 2, 10, 4));
        piece.place(east, (x, 10, 2));
        piece.place(west, (x + 4, 10, 2));
    }
    fill(piece, (8, 0, 0), (12, 4, 4), Sandstone, Air);
    fill(piece, (9, 1, 0), (11, 3, 4), Air, Air);
    for (x, y) in CUT_DOORWAY {
        piece.place(CutSandstone, (x, y, 1));
    }
    fill(piece, (4, 1, 1), (8, 3, 3), Sandstone, Air);
    fill(piece, (4, 1, 2), (8, 2, 2), Air, Air);
    fill(piece, (12, 1, 1), (16, 3, 3), Sandstone, Air);
    fill(piece, (12, 1, 2), (16, 2, 2), Air, Air);
    fill(piece, (5, 4, 5), (w - 6, 4, d - 6), Sandstone, Sandstone);
    fill(piece, (9, 4, 9), (11, 4, 11), Air, Air);
    for (x, z) in [(8, 8), (12, 8), (8, 12), (12, 12)] {
        fill(piece, (x, 1, z), (x, 3, z), CutSandstone, Sandstone);
    }
    fill(piece, (1, 1, 5), (4, 4, 11), Sandstone, Sandstone);
    fill(piece, (w - 5, 1, 5), (w - 2, 4, 11), Sandstone, Sandstone);
    fill(piece, (6, 7, 9), (6, 7, 11), Sandstone, Sandstone);
    fill(piece, (w - 7, 7, 9), (w - 7, 7, 11), Sandstone, Sandstone);
    fill(piece, (5, 5, 9), (5, 7, 11), CutSandstone, CutSandstone);
    fill(piece, (w - 6, 5, 9), (w - 6, 7, 11), CutSandstone, CutSandstone);
    for (x, y) in LEDGE_AIR {
        piece.place(Air, (x, y, 10));
    }
    fill(piece, (2, 4, 4), (2, 6, 4), Air, Air);
    fill(piece, (w - 3, 4, 4), (w - 3, 6, 4), Air, Air);
    for x in [2, w - 3] {
        piece.place(north, (x, 4, 5));
        piece.place(north, (x, 3, 4));
    }
    fill(piece, (1, 1, 3), (2, 2, 3), Sandstone, Sandstone);
    fill(piece, (w - 3, 1, 3), (w - 2, 2, 3), Sandstone, Sandstone);
    piece.place(SandstoneStairsEast, (1, 1, 2));
    piece.place(SandstoneStairsEast, (w - 2, 1, 2));
    piece.place(SandstoneSlab, (1, 2, 2));
    piece.place(SandstoneSlab, (w - 2, 2, 2));
    piece.place(west, (2, 1, 2));
    piece.place(east, (w - 3, 1, 2));
    fill(piece, (4, 3, 5), (4, 3, 18), Sandstone, Sandstone);
    fill(piece, (w - 5, 3, 5), (w - 5, 3, 17), Sandstone, Sandstone);
    fill(piece, (3, 1, 5), (4, 2, 16), Air, Air);
    fill(piece, (w - 6, 1, 5), (w - 5, 2, 16), Air, Air);
    for z in (5..=17).step_by(2) {
        piece.place(CutSandstone, (4, 1, z));
        piece.place(ChiseledSandstone, (4, 2, z));
        piece.place(CutSandstone, (w - 5, 1, z));
        piece.place(ChiseledSandstone, (w - 5, 2, z));
    }
    for (x, z) in PLATFORM_ORANGE {
        piece.place(OrangeTerracotta, (x, 0, z));
    }
    piece.place(BlueTerracotta, (10, 0, 10));
    for x in [0, w - 1] {
        for (row, y) in TOWER_ROWS.iter().zip(2..) {
            for (&block, z) in row.iter().zip(1..) {
                piece.place(block, (x, y, z));
            }
        }
    }
    for x in [2, w - 3] {
        for (row, y) in TOWER_ROWS.iter().zip(2..) {
            for (&block, dx) in row.iter().zip(-1..) {
                piece.place(block, (x + dx, y, 0));
            }
        }
    }
    fill(piece, (8, 4, 0), (12, 6, 0), CutSandstone, CutSandstone);
    piece.place(Air, (8, 6, 0));
    piece.place(Air, (12, 6, 0));
    piece.place(OrangeTerracotta, (9, 5, 0));
    piece.place(ChiseledSandstone, (10, 5, 0));
    piece.place(OrangeTerracotta, (11, 5, 0));
    treasure_room(piece, random);
    cellar::place(piece, random);
}

fn treasure_room(piece: &mut Piece, random: &mut MersenneTwister) {
    fill(piece, (8, -14, 8), (12, -11, 12), CutSandstone, CutSandstone);
    fill(piece, (8, -10, 8), (12, -10, 12), ChiseledSandstone, ChiseledSandstone);
    fill(piece, (8, -9, 8), (12, -9, 12), CutSandstone, CutSandstone);
    fill(piece, (8, -8, 8), (12, -1, 12), Sandstone, Sandstone);
    fill(piece, (9, -11, 9), (11, -1, 11), Air, Air);
    piece.place(StonePressurePlate, (10, -11, 10));
    fill(piece, (9, -13, 9), (11, -13, 11), Tnt, Air);
    for (x, z, dx, dz) in [(8, 10, -1, 0), (12, 10, 1, 0), (10, 8, 0, -1), (10, 12, 0, 1)] {
        piece.place(Air, (x, -11, z));
        piece.place(Air, (x, -10, z));
        piece.place(ChiseledSandstone, (x + dx, -10, z + dz));
        piece.place(CutSandstone, (x + dx, -11, z + dz));
    }
    for (x, z, chest) in [(10, 12, ChestNorth), (8, 10, ChestEast), (10, 8, ChestSouth), (12, 10, ChestWest)] {
        create_chest(piece, random, (x, -11, z), chest);
    }
}

pub fn create_chest(piece: &mut Piece, random: &mut MersenneTwister, local: Cell, chest: Material) {
    let world = piece.world(local);
    if piece.contains(world) && !piece.ids.is_any(piece.block_at(world), &Material::CHESTS) {
        piece.set_world(world, piece.ids.id(chest));
        random.next_u32();
    }
}
