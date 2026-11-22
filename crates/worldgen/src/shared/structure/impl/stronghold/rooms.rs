use super::build::{Dir, air_box, fill, ladder, selector_box, small_door, torch};
use super::frame::{Canvas, Frame};
use super::material::Material;
use super::{Kind, Piece};
use crate::shared::rng::mersenne::MersenneTwister;

const WEB_CHANCE: f32 = 0.07;
const SPLIT_PILLAR_CELLS: [(i32, i32); 8] = [(4, 4), (4, 5), (4, 6), (6, 4), (6, 5), (6, 6), (5, 4), (5, 6)];
const STOREROOM_CORNERS: [(i32, i32); 4] = [(4, 4), (6, 4), (4, 6), (6, 6)];
const STOREROOM_PILLARS: [(i32, i32, i32); 8] = [(5, 1, 4), (5, 1, 6), (5, 3, 4), (5, 3, 6), (4, 1, 5), (6, 1, 5), (4, 3, 5), (6, 3, 5)];
const GALLERY_CORNER: [(i32, i32); 3] = [(9, 11), (8, 11), (9, 10)];
const CHANDELIER: [(i32, i32, i32); 12] = [
    (6, 9, 7),
    (7, 9, 7),
    (6, 8, 7),
    (7, 8, 7),
    (6, 7, 7),
    (7, 7, 7),
    (5, 7, 7),
    (8, 7, 7),
    (6, 7, 6),
    (6, 7, 8),
    (7, 7, 6),
    (7, 7, 8),
];
const CHANDELIER_TORCHES: [(i32, i32); 6] = [(5, 7), (8, 7), (6, 6), (6, 8), (7, 6), (7, 8)];

pub fn post_process(piece: &Piece, canvas: &mut Canvas, random: &mut MersenneTwister) {
    match piece.kind {
        Kind::RoomCrossing => room_crossing(piece, canvas, random),
        Kind::FiveCrossing => five_crossing(piece, canvas, random),
        _ => library(piece, canvas, random),
    }
}

fn room_crossing(piece: &Piece, canvas: &mut Canvas, random: &mut MersenneTwister) {
    let frame = &piece.frame;
    selector_box(frame, canvas, random, (0, 0, 0), (10, 6, 10));
    small_door(frame, canvas, piece.door, (4, 1, 0));
    air_box(frame, canvas, (4, 1, 10), (6, 3, 10));
    air_box(frame, canvas, (0, 1, 4), (0, 3, 6));
    air_box(frame, canvas, (10, 1, 4), (10, 3, 6));
    match piece.value {
        0 => {
            (1..=3).for_each(|y| frame.place(canvas, Material::StoneBricks, (5, y, 5)));
            frame.place(canvas, torch(frame, Dir::West), (4, 3, 5));
            frame.place(canvas, torch(frame, Dir::East), (6, 3, 5));
            frame.place(canvas, torch(frame, Dir::South), (5, 3, 4));
            frame.place(canvas, torch(frame, Dir::North), (5, 3, 6));
            SPLIT_PILLAR_CELLS.iter().for_each(|&(x, z)| frame.place(canvas, Material::SmoothStoneSlab, (x, 1, z)));
        }
        1 => {
            for i in 0..5 {
                frame.place(canvas, Material::StoneBricks, (3, 1, 3 + i));
                frame.place(canvas, Material::StoneBricks, (7, 1, 3 + i));
                frame.place(canvas, Material::StoneBricks, (3 + i, 1, 3));
                frame.place(canvas, Material::StoneBricks, (3 + i, 1, 7));
            }
            (1..=3).for_each(|y| frame.place(canvas, Material::StoneBricks, (5, y, 5)));
            frame.place(canvas, Material::Water, (5, 4, 5));
        }
        2 => storeroom(frame, canvas, random),
        _ => {}
    }
}

fn storeroom(frame: &Frame, canvas: &mut Canvas, random: &mut MersenneTwister) {
    for i in 1..=9 {
        frame.place(canvas, Material::Cobblestone, (1, 3, i));
        frame.place(canvas, Material::Cobblestone, (9, 3, i));
    }
    for i in 1..=9 {
        frame.place(canvas, Material::Cobblestone, (i, 3, 1));
        frame.place(canvas, Material::Cobblestone, (i, 3, 9));
    }
    STOREROOM_PILLARS.iter().for_each(|&at| frame.place(canvas, Material::Cobblestone, at));
    for y in 1..=3 {
        STOREROOM_CORNERS.iter().for_each(|&(x, z)| frame.place(canvas, Material::Cobblestone, (x, y, z)));
    }
    frame.place(canvas, Material::TorchUnknown, (5, 3, 5));
    for z in 2..=8 {
        let columns: &[i32] = if (4..=6).contains(&z) { &[2, 3, 7, 8] } else { &[2, 3, 4, 5, 6, 7, 8] };
        columns.iter().for_each(|&x| frame.place(canvas, Material::OakPlanks, (x, 3, z)));
    }
    let step = ladder(frame, 4);
    (1..=3).for_each(|y| frame.place(canvas, step, (9, y, 3)));
    frame.chest(canvas, random, (3, 4, 8), Material::ChestWest);
}

fn five_crossing(piece: &Piece, canvas: &mut Canvas, random: &mut MersenneTwister) {
    let frame = &piece.frame;
    let [left_low, left_high, right_low, right_high] = piece.flags;
    selector_box(frame, canvas, random, (0, 0, 0), (9, 8, 10));
    small_door(frame, canvas, piece.door, (4, 3, 0));
    if left_low {
        air_box(frame, canvas, (0, 3, 1), (0, 5, 3));
    }
    if right_low {
        air_box(frame, canvas, (9, 3, 1), (9, 5, 3));
    }
    if left_high {
        air_box(frame, canvas, (0, 5, 7), (0, 7, 9));
    }
    if right_high {
        air_box(frame, canvas, (9, 5, 7), (9, 7, 9));
    }
    air_box(frame, canvas, (5, 1, 10), (7, 3, 10));
    selector_box(frame, canvas, random, (1, 2, 1), (8, 2, 6));
    selector_box(frame, canvas, random, (4, 1, 5), (4, 4, 9));
    selector_box(frame, canvas, random, (8, 1, 5), (8, 4, 9));
    selector_box(frame, canvas, random, (1, 4, 7), (3, 4, 9));
    selector_box(frame, canvas, random, (1, 3, 5), (3, 3, 6));
    fill(frame, canvas, (1, 3, 4), (3, 3, 4), Material::StoneBrickSlab);
    fill(frame, canvas, (1, 4, 6), (3, 4, 6), Material::StoneBrickSlab);
    selector_box(frame, canvas, random, (5, 1, 7), (7, 1, 8));
    fill(frame, canvas, (5, 1, 9), (7, 1, 9), Material::StoneBrickSlab);
    fill(frame, canvas, (5, 2, 7), (7, 2, 7), Material::StoneBrickSlab);
    fill(frame, canvas, (4, 5, 7), (4, 5, 9), Material::StoneBrickSlab);
    fill(frame, canvas, (8, 5, 7), (8, 5, 9), Material::StoneBrickSlab);
    fill(frame, canvas, (5, 5, 7), (7, 5, 9), Material::SmoothStoneDoubleSlab);
    frame.place(canvas, torch(frame, Dir::South), (6, 5, 6));
}

fn library(piece: &Piece, canvas: &mut Canvas, random: &mut MersenneTwister) {
    let frame = &piece.frame;
    let tall = piece.flags[0];
    selector_box(frame, canvas, random, (0, 0, 0), (13, if tall { 10 } else { 5 }, 14));
    small_door(frame, canvas, piece.door, (4, 1, 0));
    frame.scatter_box(canvas, random, WEB_CHANCE, (2, 1, 1), (11, 4, 13), Material::Web);
    for z in 1..=13 {
        let shelf = if (z - 1) % 4 == 0 { Material::OakPlanks } else { Material::Bookshelf };
        fill(frame, canvas, (1, 1, z), (1, 4, z), shelf);
        fill(frame, canvas, (12, 1, z), (12, 4, z), shelf);
        if shelf == Material::OakPlanks {
            frame.place(canvas, torch(frame, Dir::East), (2, 3, z));
            frame.place(canvas, torch(frame, Dir::West), (11, 3, z));
        }
        if tall {
            fill(frame, canvas, (1, 6, z), (1, 9, z), shelf);
            fill(frame, canvas, (12, 6, z), (12, 9, z), shelf);
        }
    }
    for z in (3..12).step_by(2) {
        for x in [3, 6, 9] {
            fill(frame, canvas, (x, 1, z), (x + 1, 3, z), Material::Bookshelf);
        }
    }
    if tall {
        gallery(frame, canvas);
    }
    frame.chest(canvas, random, (3, 3, 5), Material::ChestWest);
    if tall {
        frame.chest(canvas, random, (12, 8, 1), Material::ChestNorth);
    }
}

fn gallery(frame: &Frame, canvas: &mut Canvas) {
    let (planks, fence) = (Material::OakPlanks, Material::OakFence);
    fill(frame, canvas, (1, 5, 1), (3, 5, 13), planks);
    fill(frame, canvas, (10, 5, 1), (12, 5, 13), planks);
    fill(frame, canvas, (4, 5, 1), (9, 5, 2), planks);
    fill(frame, canvas, (4, 5, 12), (9, 5, 13), planks);
    GALLERY_CORNER.iter().for_each(|&(x, z)| frame.place(canvas, planks, (x, 5, z)));
    fill(frame, canvas, (3, 6, 2), (3, 6, 12), fence);
    fill(frame, canvas, (10, 6, 2), (10, 6, 10), fence);
    fill(frame, canvas, (4, 6, 2), (9, 6, 2), fence);
    fill(frame, canvas, (4, 6, 12), (8, 6, 12), fence);
    GALLERY_CORNER.iter().for_each(|&(x, z)| frame.place(canvas, fence, (x, 6, z)));
    let step = ladder(frame, 3);
    (1..=7).for_each(|y| frame.place(canvas, step, (10, y, 13)));
    CHANDELIER.iter().for_each(|&at| frame.place(canvas, fence, at));
    CHANDELIER_TORCHES.iter().for_each(|&(x, z)| frame.place(canvas, Material::TorchTop, (x, 8, z)));
}
