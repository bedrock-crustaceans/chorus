use super::Door;
use super::frame::{Canvas, Cell, Frame};
use super::material::Material;
use crate::shared::rng::mersenne::MersenneTwister;

const CRACKED_BELOW: f32 = 0.2;
const MOSSY_BELOW: f32 = 0.5;
const INFESTED_BELOW: f32 = 0.55;
const DOOR_FACING_DATA: u8 = 3;
const DOOR_ORDER: [usize; 4] = [1, 2, 3, 0];
const LOWER_BUTTON_DATA: u8 = 2;
const UPPER_BUTTON_DATA: u8 = 3;

#[derive(Clone, Copy)]
pub enum Dir {
    East,
    West,
    South,
    North,
}

pub fn torch(frame: &Frame, facing: Dir) -> Material {
    let table = match facing {
        Dir::East => [1, 3, 1, 3],
        Dir::West => [2, 4, 2, 4],
        Dir::South => [4, 1, 3, 2],
        Dir::North => [3, 2, 4, 1],
    };
    Material::TORCHES[table[frame.orientation as usize]]
}

pub fn ladder(frame: &Frame, data: u8) -> Material {
    Material::LADDERS[frame.facing(data) as usize - 2]
}

fn button(frame: &Frame, data: u8) -> Material {
    Material::BUTTONS[frame.facing(data) as usize - 2]
}

pub fn portal_frame(data: u8, eye: bool) -> Material {
    if eye {
        Material::PORTAL_FRAMES_WITH_EYE[data as usize]
    } else {
        Material::PORTAL_FRAMES[data as usize]
    }
}

fn select(random: &mut MersenneTwister) -> Material {
    let roll = random.next_float();
    if roll < CRACKED_BELOW {
        Material::CrackedStoneBricks
    } else if roll < MOSSY_BELOW {
        Material::MossyStoneBricks
    } else if roll < INFESTED_BELOW {
        Material::InfestedStoneBricks
    } else {
        Material::StoneBricks
    }
}

pub fn selector_box(frame: &Frame, canvas: &mut Canvas, random: &mut MersenneTwister, (x0, y0, z0): Cell, (x1, y1, z1): Cell) {
    for y in y0..=y1 {
        for x in x0..=x1 {
            for z in z0..=z1 {
                let on_edge = y == y0 || y == y1 || x == x0 || x == x1 || z == z0 || z == z1;
                let material = if on_edge { select(random) } else { Material::Air };
                frame.place(canvas, material, (x, y, z));
            }
        }
    }
}

pub fn air_box(frame: &Frame, canvas: &mut Canvas, low: Cell, high: Cell) {
    frame.generate_box(canvas, low, high, Material::Air);
}

pub fn fill(frame: &Frame, canvas: &mut Canvas, low: Cell, high: Cell, material: Material) {
    frame.generate_box(canvas, low, high, material);
}

pub fn small_door(frame: &Frame, canvas: &mut Canvas, door: Door, (x, y, z): Cell) {
    let rim = [(x, y), (x, y + 1), (x, y + 2), (x + 1, y + 2), (x + 2, y + 2), (x + 2, y + 1), (x + 2, y)];
    match door {
        Door::Opening => air_box(frame, canvas, (x, y, z), (x + 2, y + 2, z)),
        Door::Grates => {
            frame.place(canvas, Material::Air, (x + 1, y, z));
            frame.place(canvas, Material::Air, (x + 1, y + 1, z));
            rim.iter().for_each(|&(rim_x, rim_y)| frame.place(canvas, Material::IronBars, (rim_x, rim_y, z)));
        }
        Door::Wood | Door::Iron => {
            rim.iter().for_each(|&(rim_x, rim_y)| frame.place(canvas, Material::StoneBricks, (rim_x, rim_y, z)));
            let doors = if door == Door::Iron { Material::IRON_DOORS } else { Material::WOODEN_DOORS };
            let (lower, upper) = doors[DOOR_ORDER[frame.direction(DOOR_FACING_DATA) as usize]];
            frame.place(canvas, lower, (x + 1, y, z));
            frame.place(canvas, upper, (x + 1, y + 1, z));
            if door == Door::Iron {
                frame.place(canvas, button(frame, LOWER_BUTTON_DATA), (x + 2, y + 1, z + 1));
                frame.place(canvas, button(frame, UPPER_BUTTON_DATA), (x + 2, y + 1, z - 1));
            }
        }
    }
}
