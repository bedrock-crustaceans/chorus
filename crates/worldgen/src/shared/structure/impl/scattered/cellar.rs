use super::desert::WIDTH;
use super::material::Material::{self, *};
use super::piece::{Cell, Piece};
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::rng::random::{RandomSource, block_seed};
use crate::shared::rng::xoroshiro::Xoroshiro;

const STAIRS: [(i32, i32); 3] = [(13, -1), (14, -2), (15, -3)];
const ROOM_CORNER: Cell = (16, -4, 13);
const LOWEST_SUSPICIOUS: i32 = 5;
const SUSPICIOUS_SPREAD: i32 = 3;
const ORANGE_FLOOR: [(i32, i32); 9] = [(17, 14), (17, 12), (15, 14), (15, 12), (18, 13), (14, 13), (16, 15), (16, 11), (19, 13)];
type Doorway = (i32, i32, Option<(i32, i32)>);

const DOORWAYS: [Doorway; 4] = [(19, 13, Some((20, 13))), (13, 13, Some((12, 13))), (16, 16, None), (16, 10, Some((16, 9)))];
const WALLS: [(i32, Material); 3] = [(-3, CutSandstone), (-2, ChiseledSandstone), (-1, CutSandstone)];
const WALL_BOXES: [(Cell, Cell); 4] = [((13, 0, 10), (13, 0, 15)), ((19, 0, 10), (19, 0, 15)), ((13, 0, 10), (19, 0, 11)), ((13, 0, 16), (19, 0, 16))];

pub fn place(piece: &mut Piece, random: &mut MersenneTwister) {
    for (x, y) in STAIRS {
        piece.place(SandstoneStairsWest, (x, y, 17));
    }
    let sand_first = random.next_boolean();
    for x in 12..=16 {
        piece.place(Sand, (x, 0, 17));
    }
    piece.place(Sand, (14, -1, 17));
    piece.place(if sand_first { Sand } else { Sandstone }, (15, -1, 17));
    piece.place(if sand_first { Sandstone } else { Sand }, (16, -1, 17));
    piece.place(Sand, (15, -2, 17));
    piece.place(Sandstone, (16, -2, 17));
    piece.place(Sand, (16, -3, 17));
    for (y, wall) in WALLS {
        for ((x0, _, z0), (x1, _, z1)) in WALL_BOXES {
            piece.generate_box((x0, y, z0), (x1, y, z1), wall, wall, true);
        }
    }
    let (x, y, z) = piece.world(ROOM_CORNER);
    let mut xoroshiro = Xoroshiro::new(block_seed(x, y, z) as i32 as i64);
    let mut cells = Vec::new();
    for y in -3..=-1 {
        for x in 14..=18 {
            for z in 11..=15 {
                piece.place(Sand, (x, y, z));
                cells.push((x, y, z));
            }
        }
    }
    for x in 14..=18 {
        for z in 11..=15 {
            let block = if xoroshiro.next_int_bounded(3) != 0 { Sand } else { Sandstone };
            piece.place(block, (x, 0, z));
        }
    }
    piece.place(BlueTerracotta, ROOM_CORNER);
    for (x, z) in ORANGE_FLOOR {
        piece.place(OrangeTerracotta, (x, -4, z));
    }
    for (x, z, outside) in DOORWAYS {
        if (x, z) != (WIDTH - 2, 13) {
            piece.place(OrangeTerracotta, (x, -4, z));
        }
        for y in [-3, -2] {
            piece.place(Sand, (x, y, z));
            cells.push((x, y, z));
        }
        if let Some((outside_x, outside_z)) = outside {
            piece.place(CutSandstone, (outside_x, -3, outside_z));
            piece.place(ChiseledSandstone, (outside_x, -2, outside_z));
        }
    }
    let count = ((xoroshiro.next_int_bounded(SUSPICIOUS_SPREAD) + LOWEST_SUSPICIOUS) as usize).min(cells.len());
    for i in 1..cells.len() {
        let j = xoroshiro.next_int_bounded(i as i32 + 1) as usize;
        cells.swap(i, j);
    }
    for &cell in &cells[..count] {
        suspicious_sand(piece, &mut xoroshiro, cell);
    }
    let x = xoroshiro.next_int_bounded(5) + 14;
    let z = xoroshiro.next_int_bounded(5) + 11;
    suspicious_sand(piece, &mut xoroshiro, (x, 0, z));
}

fn suspicious_sand(piece: &mut Piece, xoroshiro: &mut Xoroshiro, local: Cell) {
    let world = piece.world(local);
    piece.place(SuspiciousSand, local);
    if piece.block_at(world) == piece.ids.id(SuspiciousSand) {
        xoroshiro.next_long();
    }
}
