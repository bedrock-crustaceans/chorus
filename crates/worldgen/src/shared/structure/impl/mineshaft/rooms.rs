use super::canvas::{Canvas, Cell};
use super::frame::{EAST, Frame, NORTH, SOUTH, WEST};
use super::{Builder, Kind, Materials};
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::{BoundingBox, Probed};

const ENTRANCE_DEPTH: i32 = 1;
const ROOM_HEADROOM: i32 = 3;
const ROOM_DOME_START: i32 = 4;
const MIN_ROOM_HEIGHT: i32 = 5;
const WALL_HEIGHT_MARGIN: i32 = 4;
const CHILD_WIDTH: i32 = 3;
const CHILD_GAP: i32 = 4;
const UPPER_EXIT_HEIGHT: i32 = 4;

pub fn room_children(builder: &mut Builder, index: usize, random: &mut MersenneTwister) {
    if random.next_float() > builder.materials.room_chance {
        return;
    }
    let (room, depth) = (builder.pieces[index].frame.bounds, builder.pieces[index].frame.depth);
    let heights = (room.y1 - room.y0 + 1).max(MIN_ROOM_HEIGHT) - WALL_HEIGHT_MARGIN;
    let (x_span, z_span) = (room.x1 - room.x0 + 1, room.z1 - room.z0 + 1);
    for (span, direction) in [(x_span, NORTH), (x_span, SOUTH), (z_span, WEST), (z_span, EAST)] {
        let mut offset = 0;
        while offset < span {
            offset += random.next_int_bounded(span);
            if offset + CHILD_WIDTH > span {
                break;
            }
            let y = room.y0 + random.next_int_bounded(heights) + 1;
            let at = match direction {
                NORTH => (room.x0 + offset, y, room.z0 - 1),
                SOUTH => (room.x0 + offset, y, room.z1 + 1),
                WEST => (room.x0 - 1, y, room.z0 + offset),
                _ => (room.x1 + 1, y, room.z0 + offset),
            };
            let before = builder.pieces.len();
            builder.add(random, at, direction, depth);
            if builder.pieces.len() > before {
                let child = builder.pieces[before].frame.bounds;
                let entrance = match direction {
                    NORTH => BoundingBox::new(child.x0, child.y0, room.z0, child.x1, child.y1, room.z0 + ENTRANCE_DEPTH),
                    SOUTH => BoundingBox::new(child.x0, child.y0, room.z1 - ENTRANCE_DEPTH, child.x1, child.y1, room.z1),
                    WEST => BoundingBox::new(room.x0, child.y0, child.z0, room.x0 + ENTRANCE_DEPTH, child.y1, child.z1),
                    _ => BoundingBox::new(room.x1 - ENTRANCE_DEPTH, child.y0, child.z0, room.x1, child.y1, child.z1),
                };
                if let Kind::Room(entrances) = &mut builder.pieces[index].kind {
                    entrances.push(entrance);
                }
            }
            offset += CHILD_GAP;
        }
    }
}

pub fn room(frame: &Frame, entrances: &[BoundingBox], canvas: &mut Canvas) -> Probed<()> {
    let (b, air) = (frame.bounds, canvas.ids.air);
    frame.generate_box(canvas, (b.x0, b.y0 + 1, b.z0), (b.x1, (b.y0 + ROOM_HEADROOM).min(b.y1), b.z1), air, air)?;
    for e in entrances {
        frame.generate_box(canvas, (e.x0, e.y1 - 2, e.z0), (e.x1, e.y1, e.z1), air, air)?;
    }
    frame.upper_half_sphere(canvas, (b.x0, b.y0 + ROOM_DOME_START, b.z0), (b.x1, b.y1, b.z1), air)
}

pub fn crossing_children(builder: &mut Builder, index: usize, random: &mut MersenneTwister) {
    let piece = &builder.pieces[index];
    let (b, depth) = (piece.frame.bounds, piece.frame.depth);
    let Kind::Crossing { direction, two_floors } = piece.kind else { return };
    let north = (b.x0 + 1, b.y0, b.z0 - 1);
    let south = (b.x0 + 1, b.y0, b.z1 + 1);
    let west = (b.x0 - 1, b.y0, b.z0 + 1);
    let east = (b.x1 + 1, b.y0, b.z0 + 1);
    let exits: [(Cell, u8); 3] = match direction {
        SOUTH => [(south, SOUTH), (west, WEST), (east, EAST)],
        WEST => [(north, NORTH), (south, SOUTH), (west, WEST)],
        NORTH => [(north, NORTH), (west, WEST), (east, EAST)],
        _ => [(north, NORTH), (south, SOUTH), (east, EAST)],
    };
    for (at, direction) in exits {
        builder.add(random, at, direction, depth);
    }
    if two_floors {
        let up = b.y0 + UPPER_EXIT_HEIGHT;
        for (at, direction) in [
            ((b.x0 + 1, up, b.z0 - 1), NORTH),
            ((b.x0 - 1, up, b.z0 + 1), WEST),
            ((b.x1 + 1, up, b.z0 + 1), EAST),
            ((b.x0 + 1, up, b.z1 + 1), SOUTH),
        ] {
            if random.next_boolean() {
                builder.add(random, at, direction, depth);
            }
        }
    }
}

pub fn crossing(frame: &Frame, two_floors: bool, materials: Materials, canvas: &mut Canvas) -> Probed<()> {
    let (b, air, planks) = (frame.bounds, canvas.ids.air, materials.planks(canvas.shaft));
    if two_floors {
        frame.generate_box(canvas, (b.x0 + 1, b.y0, b.z0), (b.x1 - 1, b.y0 + 2, b.z1), air, air)?;
        frame.generate_box(canvas, (b.x0, b.y0, b.z0 + 1), (b.x1, b.y0 + 2, b.z1 - 1), air, air)?;
        frame.generate_box(canvas, (b.x0 + 1, b.y1 - 2, b.z0), (b.x1 - 1, b.y1, b.z1), air, air)?;
        frame.generate_box(canvas, (b.x0, b.y1 - 2, b.z0 + 1), (b.x1, b.y1, b.z1 - 1), air, air)?;
        frame.generate_box(canvas, (b.x0 + 1, b.y0 + 3, b.z0 + 1), (b.x1 - 1, b.y0 + 3, b.z1 - 1), air, air)?;
    } else {
        frame.generate_box(canvas, (b.x0 + 1, b.y0, b.z0), (b.x1 - 1, b.y1, b.z1), air, air)?;
        frame.generate_box(canvas, (b.x0, b.y0, b.z0 + 1), (b.x1, b.y1, b.z1 - 1), air, air)?;
    }
    for (x, z) in [(b.x0 + 1, b.z0 + 1), (b.x0 + 1, b.z1 - 1), (b.x1 - 1, b.z0 + 1), (b.x1 - 1, b.z1 - 1)] {
        if frame.get(canvas, (x, b.y1 + 1, z))? != air {
            frame.generate_box(canvas, (x, b.y0, z), (x, b.y1, z), planks, air)?;
        }
    }
    for x in b.x0..=b.x1 {
        for z in b.z0..=b.z1 {
            let below = canvas.get(x, b.y0 - 1, z)?;
            if canvas.interior(x, b.y0, z)? && (below == air || below == canvas.shaft.chain) {
                canvas.set(x, b.y0 - 1, z, planks);
            }
        }
    }
    Ok(())
}

pub fn stairs_children(builder: &mut Builder, index: usize, random: &mut MersenneTwister) {
    let frame = builder.pieces[index].frame;
    let (b, depth) = (frame.bounds, frame.depth);
    let (at, direction) = match frame.orientation.unwrap_or(SOUTH) {
        SOUTH => ((b.x0, b.y0, b.z1 + 1), SOUTH),
        WEST => ((b.x0 - 1, b.y0, b.z0), WEST),
        NORTH => ((b.x0, b.y0, b.z0 - 1), NORTH),
        _ => ((b.x1 + 1, b.y0, b.z0), EAST),
    };
    builder.add(random, at, direction, depth);
}

pub fn stairs(frame: &Frame, canvas: &mut Canvas) -> Probed<()> {
    let air = canvas.ids.air;
    frame.generate_box(canvas, (0, 5, 0), (2, 7, 1), air, air)?;
    frame.generate_box(canvas, (0, 0, 7), (2, 2, 8), air, air)?;
    for i in 0..5 {
        frame.generate_box(canvas, (0, 5 - i - i32::from(i < 4), 2 + i), (2, 7 - i, 2 + i), air, air)?;
    }
    Ok(())
}
