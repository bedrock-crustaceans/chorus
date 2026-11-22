use super::build::{Dir, air_box, fill, selector_box, small_door, torch};
use super::frame::{Canvas, EAST, NORTH};
use super::material::Material;
use super::{Door, Kind, Piece};
use crate::shared::rng::mersenne::MersenneTwister;

const TORCH_CHANCE: f32 = 0.1;
const STAIR_STEPS: [(Material, i32, i32, i32); 17] = [
    (Material::StoneBricks, 2, 6, 1),
    (Material::StoneBricks, 1, 5, 1),
    (Material::StoneBrickSlab, 1, 6, 1),
    (Material::StoneBricks, 1, 5, 2),
    (Material::StoneBricks, 1, 4, 3),
    (Material::StoneBrickSlab, 1, 5, 3),
    (Material::StoneBricks, 2, 4, 3),
    (Material::StoneBricks, 3, 3, 3),
    (Material::StoneBrickSlab, 3, 4, 3),
    (Material::StoneBricks, 3, 3, 2),
    (Material::StoneBricks, 3, 2, 1),
    (Material::StoneBrickSlab, 3, 3, 1),
    (Material::StoneBricks, 2, 2, 1),
    (Material::StoneBricks, 1, 1, 1),
    (Material::StoneBrickSlab, 1, 2, 1),
    (Material::StoneBricks, 1, 1, 2),
    (Material::StoneBrickSlab, 1, 1, 3),
];
const CHEST_SLABS: [(i32, i32, i32); 7] = [(3, 1, 1), (3, 1, 5), (3, 2, 2), (3, 2, 4), (2, 1, 2), (2, 1, 3), (2, 1, 4)];

pub fn post_process(piece: &Piece, canvas: &mut Canvas, random: &mut MersenneTwister) {
    let frame = &piece.frame;
    let turns_left_first = frame.orientation == NORTH || frame.orientation == EAST;
    match piece.kind {
        Kind::StairsDown => {
            selector_box(frame, canvas, random, (0, 0, 0), (4, 10, 4));
            small_door(frame, canvas, piece.door, (1, 7, 0));
            small_door(frame, canvas, Door::Opening, (1, 1, 4));
            STAIR_STEPS.iter().for_each(|&(material, x, y, z)| frame.place(canvas, material, (x, y, z)));
        }
        Kind::Straight => {
            selector_box(frame, canvas, random, (0, 0, 0), (4, 4, 6));
            small_door(frame, canvas, piece.door, (1, 1, 0));
            small_door(frame, canvas, Door::Opening, (1, 1, 6));
            let (east, west) = (torch(frame, Dir::East), torch(frame, Dir::West));
            for (material, x, z) in [(east, 1, 1), (west, 3, 1), (east, 1, 5), (west, 3, 5)] {
                if random.next_float() < TORCH_CHANCE {
                    frame.place(canvas, material, (x, 2, z));
                }
            }
            if piece.flags[0] {
                air_box(frame, canvas, (0, 1, 2), (0, 3, 4));
            }
            if piece.flags[1] {
                air_box(frame, canvas, (4, 1, 2), (4, 3, 4));
            }
        }
        Kind::PrisonHall => {
            selector_box(frame, canvas, random, (0, 0, 0), (8, 4, 10));
            small_door(frame, canvas, piece.door, (1, 1, 0));
            air_box(frame, canvas, (1, 1, 10), (3, 3, 10));
            for z in [1, 3, 7, 9] {
                selector_box(frame, canvas, random, (4, 1, z), (4, 3, z));
            }
            fill(frame, canvas, (4, 1, 4), (4, 3, 6), Material::IronBars);
            fill(frame, canvas, (5, 1, 5), (7, 3, 5), Material::IronBars);
            frame.place(canvas, Material::IronBars, (4, 3, 2));
            frame.place(canvas, Material::IronBars, (4, 3, 8));
        }
        Kind::LeftTurn | Kind::RightTurn => {
            selector_box(frame, canvas, random, (0, 0, 0), (4, 4, 4));
            small_door(frame, canvas, piece.door, (1, 1, 0));
            let x = if turns_left_first == (piece.kind == Kind::LeftTurn) { 0 } else { 4 };
            air_box(frame, canvas, (x, 1, 1), (x, 3, 3));
        }
        Kind::StraightStairsDown => {
            selector_box(frame, canvas, random, (0, 0, 0), (4, 10, 7));
            small_door(frame, canvas, piece.door, (1, 7, 0));
            small_door(frame, canvas, Door::Opening, (1, 1, 7));
            let step = frame.stairs(Material::STONE_STAIRS, 2);
            for i in 0..6 {
                (1..=3).for_each(|x| frame.place(canvas, step, (x, 6 - i, 1 + i)));
                if i < 5 {
                    (1..=3).for_each(|x| frame.place(canvas, Material::StoneBricks, (x, 5 - i, 1 + i)));
                }
            }
        }
        Kind::ChestCorridor => {
            selector_box(frame, canvas, random, (0, 0, 0), (4, 4, 6));
            small_door(frame, canvas, piece.door, (1, 1, 0));
            small_door(frame, canvas, Door::Opening, (1, 1, 6));
            fill(frame, canvas, (3, 1, 2), (3, 1, 4), Material::StoneBricks);
            CHEST_SLABS.iter().for_each(|&at| frame.place(canvas, Material::StoneBrickSlab, at));
            frame.chest(canvas, random, (3, 2, 3), Material::ChestWest);
        }
        Kind::Filler => {
            for z in 0..piece.value {
                for y in 0..=4 {
                    for x in 0..=4 {
                        let wall = y == 0 || y == 4 || x == 0 || x == 4;
                        frame.place(canvas, if wall { Material::StoneBricks } else { Material::Air }, (x, y, z));
                    }
                }
            }
        }
        Kind::RoomCrossing | Kind::FiveCrossing | Kind::Library | Kind::PortalRoom => {}
    }
}
