use crate::shared::blocks::Blocks;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::BoundingBox;

use super::super::{NetherBlocks, NetherChunk};
use super::fortress::{Fortress, PieceKind};
use super::{FortressCanvas, Frame};

pub fn draw(fortress: &Fortress, chunk: (i32, i32), base: &mut NetherChunk, random: &mut MersenneTwister, ids: &NetherBlocks, blocks: &Blocks) {
    let column = BoundingBox::chunk(chunk.0, chunk.1, base.definition());
    for piece in fortress.pieces.iter().filter(|piece| piece.frame.bb.intersects(&column)) {
        let mut canvas = FortressCanvas::new(base, chunk, blocks);
        let frame = &piece.frame;
        match piece.kind {
            PieceKind::StraightBridge => straight(frame, &mut canvas, ids),
            PieceKind::CrossingBridge => crossing(frame, &mut canvas, ids),
            PieceKind::BridgeEnd => end(piece, &mut canvas, ids),
            PieceKind::RoomCrossing => room(frame, &mut canvas, ids),
            PieceKind::StairRoom => stair_room(frame, &mut canvas, ids),
            PieceKind::ThroneRoom => throne(frame, &mut canvas, ids),
            PieceKind::CastleGate => gate(frame, &mut canvas, ids),
            PieceKind::StalkRoom => stalk(frame, &mut canvas, ids),
            PieceKind::SmallCorridor => corridor(frame, &mut canvas, ids),
            PieceKind::CorridorCrossing => corridor_crossing(frame, &mut canvas, ids),
            PieceKind::RightTurn => turn(frame, &mut canvas, random, piece.chest, false, ids),
            PieceKind::LeftTurn => turn(frame, &mut canvas, random, piece.chest, true, ids),
            PieceKind::CorridorStairs => corridor_stairs(frame, &mut canvas, ids),
            PieceKind::Balcony => balcony(frame, &mut canvas, ids),
        }
    }
}

type Lo = (i32, i32, i32);

fn brick(frame: &Frame, canvas: &mut FortressCanvas, lo: Lo, hi: Lo, ids: &NetherBlocks) {
    frame.fill(canvas, lo, hi, ids.nether_brick);
}

fn fences(frame: &Frame, canvas: &mut FortressCanvas, lo: Lo, hi: Lo, ids: &NetherBlocks) {
    frame.fill(canvas, lo, hi, ids.nether_brick_fence);
}

fn clear(frame: &Frame, canvas: &mut FortressCanvas, lo: Lo, hi: Lo, ids: &NetherBlocks) {
    frame.fill(canvas, lo, hi, ids.air);
}

fn down(frame: &Frame, canvas: &mut FortressCanvas, x: i32, z: i32, ids: &NetherBlocks) {
    frame.fill_down(canvas, ids.nether_brick, x, z);
}

fn step(frame: &Frame, data: u8, ids: &NetherBlocks) -> u16 {
    frame.stairs(ids.nether_brick_stairs, data)
}

fn place_chest(frame: &Frame, canvas: &mut FortressCanvas, random: &mut MersenneTwister, (x, y, z): Lo, ids: &NetherBlocks) {
    let (world_x, world_y, world_z) = frame.world(x, y, z);
    if canvas.contains(world_x, world_y, world_z) && !canvas.chest_at(world_x, world_y, world_z) {
        canvas.set(world_x, world_y, world_z, ids.chest);
        random.next_u32();
    }
}

fn straight(frame: &Frame, canvas: &mut FortressCanvas, ids: &NetherBlocks) {
    brick(frame, canvas, (0, 3, 0), (4, 4, 18), ids);
    clear(frame, canvas, (1, 5, 0), (3, 7, 18), ids);
    brick(frame, canvas, (0, 5, 0), (0, 5, 18), ids);
    brick(frame, canvas, (4, 5, 0), (4, 5, 18), ids);
    brick(frame, canvas, (0, 2, 0), (4, 2, 5), ids);
    brick(frame, canvas, (0, 2, 13), (4, 2, 18), ids);
    brick(frame, canvas, (0, 0, 0), (4, 1, 3), ids);
    brick(frame, canvas, (0, 0, 15), (4, 1, 18), ids);
    for x in 0..=4 {
        for z in 0..=2 {
            down(frame, canvas, x, z, ids);
            down(frame, canvas, x, 18 - z, ids);
        }
    }
    for x in [0, 4] {
        fences(frame, canvas, (x, 1, 1), (x, 4, 1), ids);
        fences(frame, canvas, (x, 3, 4), (x, 4, 4), ids);
        fences(frame, canvas, (x, 3, 14), (x, 4, 14), ids);
        fences(frame, canvas, (x, 1, 17), (x, 4, 17), ids);
    }
}

fn crossing(frame: &Frame, canvas: &mut FortressCanvas, ids: &NetherBlocks) {
    brick(frame, canvas, (7, 3, 0), (11, 4, 18), ids);
    brick(frame, canvas, (0, 3, 7), (18, 4, 11), ids);
    clear(frame, canvas, (8, 5, 0), (10, 7, 18), ids);
    clear(frame, canvas, (0, 5, 8), (18, 7, 10), ids);
    brick(frame, canvas, (7, 5, 0), (7, 5, 7), ids);
    brick(frame, canvas, (7, 5, 11), (7, 5, 18), ids);
    brick(frame, canvas, (11, 5, 0), (11, 5, 7), ids);
    brick(frame, canvas, (11, 5, 11), (11, 5, 18), ids);
    brick(frame, canvas, (0, 5, 7), (7, 5, 7), ids);
    brick(frame, canvas, (11, 5, 7), (18, 5, 7), ids);
    brick(frame, canvas, (0, 5, 11), (7, 5, 11), ids);
    brick(frame, canvas, (11, 5, 11), (18, 5, 11), ids);
    brick(frame, canvas, (7, 2, 0), (11, 2, 5), ids);
    brick(frame, canvas, (7, 2, 13), (11, 2, 18), ids);
    brick(frame, canvas, (7, 0, 0), (11, 1, 3), ids);
    brick(frame, canvas, (7, 0, 15), (11, 1, 18), ids);
    for x in 7..=11 {
        for z in 0..=2 {
            down(frame, canvas, x, z, ids);
            down(frame, canvas, x, 18 - z, ids);
        }
    }
    brick(frame, canvas, (0, 2, 7), (5, 2, 11), ids);
    brick(frame, canvas, (13, 2, 7), (18, 2, 11), ids);
    brick(frame, canvas, (0, 0, 7), (3, 1, 11), ids);
    brick(frame, canvas, (15, 0, 7), (18, 1, 11), ids);
    for x in 0..=2 {
        for z in 7..=11 {
            down(frame, canvas, x, z, ids);
            down(frame, canvas, 18 - x, z, ids);
        }
    }
}

fn end(piece: &super::fortress::Piece, canvas: &mut FortressCanvas, ids: &NetherBlocks) {
    let frame = &piece.frame;
    let mut random = MersenneTwister::new(piece.seed);
    for x in 0..=4 {
        for y in 3..=4 {
            let z = random.next_int_bounded(8);
            brick(frame, canvas, (x, y, 0), (x, y, z), ids);
        }
    }
    let z = random.next_int_bounded(8);
    brick(frame, canvas, (0, 5, 0), (0, 5, z), ids);
    let z = random.next_int_bounded(8);
    brick(frame, canvas, (4, 5, 0), (4, 5, z), ids);
    for x in 0..=4 {
        let z = random.next_int_bounded(5);
        brick(frame, canvas, (x, 2, 0), (x, 2, z), ids);
    }
    for x in 0..=4 {
        for y in 0..=1 {
            let z = random.next_int_bounded(3);
            brick(frame, canvas, (x, y, 0), (x, y, z), ids);
        }
    }
}

fn room_floor(frame: &Frame, canvas: &mut FortressCanvas, ids: &NetherBlocks) {
    for x in 0..=6 {
        for z in 0..=6 {
            down(frame, canvas, x, z, ids);
        }
    }
}

fn room(frame: &Frame, canvas: &mut FortressCanvas, ids: &NetherBlocks) {
    brick(frame, canvas, (0, 0, 0), (6, 1, 6), ids);
    clear(frame, canvas, (0, 2, 0), (6, 7, 6), ids);
    brick(frame, canvas, (0, 2, 0), (1, 6, 0), ids);
    brick(frame, canvas, (0, 2, 6), (1, 6, 6), ids);
    brick(frame, canvas, (5, 2, 0), (6, 6, 0), ids);
    brick(frame, canvas, (5, 2, 6), (6, 6, 6), ids);
    brick(frame, canvas, (0, 2, 0), (0, 6, 1), ids);
    brick(frame, canvas, (0, 2, 5), (0, 6, 6), ids);
    brick(frame, canvas, (6, 2, 0), (6, 6, 1), ids);
    brick(frame, canvas, (6, 2, 5), (6, 6, 6), ids);
    brick(frame, canvas, (2, 6, 0), (4, 6, 0), ids);
    fences(frame, canvas, (2, 5, 0), (4, 5, 0), ids);
    brick(frame, canvas, (2, 6, 6), (4, 6, 6), ids);
    fences(frame, canvas, (2, 5, 6), (4, 5, 6), ids);
    brick(frame, canvas, (0, 6, 2), (0, 6, 4), ids);
    fences(frame, canvas, (0, 5, 2), (0, 5, 4), ids);
    brick(frame, canvas, (6, 6, 2), (6, 6, 4), ids);
    fences(frame, canvas, (6, 5, 2), (6, 5, 4), ids);
    room_floor(frame, canvas, ids);
}

fn stair_room(frame: &Frame, canvas: &mut FortressCanvas, ids: &NetherBlocks) {
    brick(frame, canvas, (0, 0, 0), (6, 1, 6), ids);
    clear(frame, canvas, (0, 2, 0), (6, 10, 6), ids);
    brick(frame, canvas, (0, 2, 0), (1, 8, 0), ids);
    brick(frame, canvas, (5, 2, 0), (6, 8, 0), ids);
    brick(frame, canvas, (0, 2, 1), (0, 8, 6), ids);
    brick(frame, canvas, (6, 2, 1), (6, 8, 6), ids);
    brick(frame, canvas, (1, 2, 6), (5, 8, 6), ids);
    fences(frame, canvas, (0, 3, 2), (0, 5, 4), ids);
    fences(frame, canvas, (6, 3, 2), (6, 5, 2), ids);
    fences(frame, canvas, (6, 3, 4), (6, 5, 4), ids);
    frame.place(canvas, ids.nether_brick, 5, 2, 5);
    brick(frame, canvas, (4, 2, 5), (4, 3, 5), ids);
    brick(frame, canvas, (3, 2, 5), (3, 4, 5), ids);
    brick(frame, canvas, (2, 2, 5), (2, 5, 5), ids);
    brick(frame, canvas, (1, 2, 5), (1, 6, 5), ids);
    brick(frame, canvas, (1, 7, 1), (5, 7, 4), ids);
    clear(frame, canvas, (6, 8, 2), (6, 8, 4), ids);
    brick(frame, canvas, (2, 6, 0), (4, 8, 0), ids);
    fences(frame, canvas, (2, 5, 0), (4, 5, 0), ids);
    room_floor(frame, canvas, ids);
}

fn throne(frame: &Frame, canvas: &mut FortressCanvas, ids: &NetherBlocks) {
    clear(frame, canvas, (0, 2, 0), (6, 7, 7), ids);
    brick(frame, canvas, (1, 0, 0), (5, 1, 7), ids);
    brick(frame, canvas, (1, 2, 1), (5, 2, 7), ids);
    brick(frame, canvas, (1, 3, 2), (5, 3, 7), ids);
    brick(frame, canvas, (1, 4, 3), (5, 4, 7), ids);
    brick(frame, canvas, (1, 2, 0), (1, 4, 2), ids);
    brick(frame, canvas, (5, 2, 0), (5, 4, 2), ids);
    brick(frame, canvas, (1, 5, 2), (1, 5, 3), ids);
    brick(frame, canvas, (5, 5, 2), (5, 5, 3), ids);
    brick(frame, canvas, (0, 5, 3), (0, 5, 8), ids);
    brick(frame, canvas, (6, 5, 3), (6, 5, 8), ids);
    brick(frame, canvas, (1, 5, 8), (5, 5, 8), ids);
    frame.place(canvas, ids.nether_brick_fence, 1, 6, 3);
    frame.place(canvas, ids.nether_brick_fence, 5, 6, 3);
    fences(frame, canvas, (0, 6, 3), (0, 6, 8), ids);
    fences(frame, canvas, (6, 6, 3), (6, 6, 8), ids);
    fences(frame, canvas, (1, 6, 8), (5, 7, 8), ids);
    fences(frame, canvas, (2, 8, 8), (4, 8, 8), ids);
    let (world_x, world_y, world_z) = frame.world(3, 5, 5);
    if canvas.contains(world_x, world_y, world_z) {
        canvas.set(world_x, world_y, world_z, ids.mob_spawner);
    }
    room_floor(frame, canvas, ids);
}

fn shell(frame: &Frame, canvas: &mut FortressCanvas, gate: bool, ids: &NetherBlocks) {
    brick(frame, canvas, (0, 3, 0), (12, 4, 12), ids);
    clear(frame, canvas, (0, 5, 0), (12, 13, 12), ids);
    brick(frame, canvas, (0, 5, 0), (1, 12, 12), ids);
    brick(frame, canvas, (11, 5, 0), (12, 12, 12), ids);
    brick(frame, canvas, (2, 5, 11), (4, 12, 12), ids);
    brick(frame, canvas, (8, 5, 11), (10, 12, 12), ids);
    brick(frame, canvas, (5, 9, 11), (7, 12, 12), ids);
    brick(frame, canvas, (2, 5, 0), (4, 12, 1), ids);
    brick(frame, canvas, (8, 5, 0), (10, 12, 1), ids);
    brick(frame, canvas, (5, 9, 0), (7, 12, 1), ids);
    brick(frame, canvas, (2, 11, 2), (10, 12, 10), ids);
    if gate {
        fences(frame, canvas, (5, 8, 0), (7, 8, 0), ids);
    }
    for i in (1..=11).step_by(2) {
        fences(frame, canvas, (i, 10, 0), (i, 11, 0), ids);
        fences(frame, canvas, (i, 10, 12), (i, 11, 12), ids);
        fences(frame, canvas, (0, 10, i), (0, 11, i), ids);
        fences(frame, canvas, (12, 10, i), (12, 11, i), ids);
        for (x, z) in [(i, 0), (i, 12), (0, i), (12, i)] {
            frame.place(canvas, ids.nether_brick, x, 13, z);
        }
        for (x, z) in [(i + 1, 0), (i + 1, 12), (0, i + 1), (12, i + 1)] {
            frame.place(canvas, ids.nether_brick_fence, x, 13, z);
        }
    }
    for (x, z) in [(0, 0), (0, 12), (12, 12), (12, 0)] {
        frame.place(canvas, ids.nether_brick_fence, x, 13, z);
    }
    for z in (3..=9).step_by(2) {
        fences(frame, canvas, (1, 7, z), (1, 8, z), ids);
        fences(frame, canvas, (11, 7, z), (11, 8, z), ids);
    }
}

fn base(frame: &Frame, canvas: &mut FortressCanvas, ids: &NetherBlocks) {
    brick(frame, canvas, (4, 2, 0), (8, 2, 12), ids);
    brick(frame, canvas, (0, 2, 4), (12, 2, 8), ids);
    brick(frame, canvas, (4, 0, 0), (8, 1, 3), ids);
    brick(frame, canvas, (4, 0, 9), (8, 1, 12), ids);
    brick(frame, canvas, (0, 0, 4), (3, 1, 8), ids);
    brick(frame, canvas, (9, 0, 4), (12, 1, 8), ids);
    for x in 4..=8 {
        for z in 0..=2 {
            down(frame, canvas, x, z, ids);
            down(frame, canvas, x, 12 - z, ids);
        }
    }
    for x in 0..=2 {
        for z in 4..=8 {
            down(frame, canvas, x, z, ids);
            down(frame, canvas, 12 - x, z, ids);
        }
    }
}

fn gate(frame: &Frame, canvas: &mut FortressCanvas, ids: &NetherBlocks) {
    shell(frame, canvas, true, ids);
    base(frame, canvas, ids);
    brick(frame, canvas, (5, 5, 5), (7, 5, 7), ids);
    clear(frame, canvas, (6, 1, 6), (6, 4, 6), ids);
    frame.place(canvas, ids.nether_brick, 6, 0, 6);
    frame.place(canvas, ids.lava, 6, 5, 6);
}

fn stalk(frame: &Frame, canvas: &mut FortressCanvas, ids: &NetherBlocks) {
    shell(frame, canvas, false, ids);
    let flight = step(frame, 3, ids);
    for k in 0..=6 {
        let z = k + 4;
        for x in 5..=7 {
            frame.place(canvas, flight, x, 5 + k, z);
        }
        if (1..=4).contains(&k) {
            brick(frame, canvas, (5, 5, z), (7, k + 4, z), ids);
        } else if k >= 5 {
            brick(frame, canvas, (5, 8, z), (7, k + 4, z), ids);
        }
        if k >= 1 {
            clear(frame, canvas, (5, 6 + k, z), (7, 9 + k, z), ids);
        }
    }
    for x in 5..=7 {
        frame.place(canvas, flight, x, 12, 11);
    }
    fences(frame, canvas, (5, 6, 7), (5, 7, 7), ids);
    fences(frame, canvas, (7, 6, 7), (7, 7, 7), ids);
    clear(frame, canvas, (5, 13, 12), (7, 13, 12), ids);
    brick(frame, canvas, (2, 5, 2), (3, 5, 3), ids);
    brick(frame, canvas, (2, 5, 9), (3, 5, 10), ids);
    brick(frame, canvas, (2, 5, 4), (2, 5, 8), ids);
    brick(frame, canvas, (9, 5, 2), (10, 5, 3), ids);
    brick(frame, canvas, (9, 5, 9), (10, 5, 10), ids);
    brick(frame, canvas, (10, 5, 4), (10, 5, 8), ids);
    let (west, east) = (step(frame, 1, ids), step(frame, 0, ids));
    for z in [2, 3, 9, 10] {
        frame.place(canvas, west, 4, 5, z);
    }
    for z in [2, 3, 9, 10] {
        frame.place(canvas, east, 8, 5, z);
    }
    frame.fill(canvas, (3, 4, 4), (4, 4, 8), ids.soul_sand);
    frame.fill(canvas, (8, 4, 4), (9, 4, 8), ids.soul_sand);
    frame.fill(canvas, (3, 5, 4), (4, 5, 8), ids.nether_wart_plant);
    frame.fill(canvas, (8, 5, 4), (9, 5, 8), ids.nether_wart_plant);
    base(frame, canvas, ids);
}

fn corridor_floor(frame: &Frame, canvas: &mut FortressCanvas, ids: &NetherBlocks) {
    for x in 0..=4 {
        for z in 0..=4 {
            down(frame, canvas, x, z, ids);
        }
    }
}

fn corridor(frame: &Frame, canvas: &mut FortressCanvas, ids: &NetherBlocks) {
    brick(frame, canvas, (0, 0, 0), (4, 1, 4), ids);
    clear(frame, canvas, (0, 2, 0), (4, 5, 4), ids);
    brick(frame, canvas, (0, 2, 0), (0, 5, 4), ids);
    brick(frame, canvas, (4, 2, 0), (4, 5, 4), ids);
    fences(frame, canvas, (0, 3, 1), (0, 4, 1), ids);
    fences(frame, canvas, (0, 3, 3), (0, 4, 3), ids);
    fences(frame, canvas, (4, 3, 1), (4, 4, 1), ids);
    fences(frame, canvas, (4, 3, 3), (4, 4, 3), ids);
    brick(frame, canvas, (0, 6, 0), (4, 6, 4), ids);
    corridor_floor(frame, canvas, ids);
}

fn corridor_crossing(frame: &Frame, canvas: &mut FortressCanvas, ids: &NetherBlocks) {
    brick(frame, canvas, (0, 0, 0), (4, 1, 4), ids);
    clear(frame, canvas, (0, 2, 0), (4, 5, 4), ids);
    brick(frame, canvas, (0, 2, 0), (0, 5, 0), ids);
    brick(frame, canvas, (4, 2, 0), (4, 5, 0), ids);
    brick(frame, canvas, (0, 2, 4), (0, 5, 4), ids);
    brick(frame, canvas, (4, 2, 4), (4, 5, 4), ids);
    brick(frame, canvas, (0, 6, 0), (4, 6, 4), ids);
    corridor_floor(frame, canvas, ids);
}

fn turn(frame: &Frame, canvas: &mut FortressCanvas, random: &mut MersenneTwister, chest: bool, left: bool, ids: &NetherBlocks) {
    let (wall, open) = if left { (4, 0) } else { (0, 4) };
    brick(frame, canvas, (0, 0, 0), (4, 1, 4), ids);
    clear(frame, canvas, (0, 2, 0), (4, 5, 4), ids);
    brick(frame, canvas, (wall, 2, 0), (wall, 5, 4), ids);
    fences(frame, canvas, (wall, 3, 1), (wall, 4, 1), ids);
    fences(frame, canvas, (wall, 3, 3), (wall, 4, 3), ids);
    brick(frame, canvas, (open, 2, 0), (open, 5, 0), ids);
    if left {
        brick(frame, canvas, (0, 2, 4), (3, 5, 4), ids);
    } else {
        brick(frame, canvas, (1, 2, 4), (4, 5, 4), ids);
    }
    fences(frame, canvas, (1, 3, 4), (1, 4, 4), ids);
    fences(frame, canvas, (3, 3, 4), (3, 4, 4), ids);
    if chest {
        place_chest(frame, canvas, random, (if left { 3 } else { 1 }, 2, 3), ids);
    }
    brick(frame, canvas, (0, 6, 0), (4, 6, 4), ids);
    corridor_floor(frame, canvas, ids);
}

fn corridor_stairs(frame: &Frame, canvas: &mut FortressCanvas, ids: &NetherBlocks) {
    let stair = step(frame, 2, ids);
    for z in 0..=9 {
        let floor = (7 - z).max(1);
        let ceiling = (floor + 5).max(14 - z).min(13);
        brick(frame, canvas, (0, 0, z), (4, floor, z), ids);
        clear(frame, canvas, (1, floor + 1, z), (3, ceiling - 1, z), ids);
        if z <= 6 {
            for x in 1..=3 {
                frame.place(canvas, stair, x, floor + 1, z);
            }
        }
        brick(frame, canvas, (0, ceiling, z), (4, ceiling, z), ids);
        brick(frame, canvas, (0, floor + 1, z), (0, ceiling - 1, z), ids);
        brick(frame, canvas, (4, floor + 1, z), (4, ceiling - 1, z), ids);
        if z & 1 == 0 {
            fences(frame, canvas, (0, floor + 2, z), (0, floor + 3, z), ids);
            fences(frame, canvas, (4, floor + 2, z), (4, floor + 3, z), ids);
        }
        for x in 0..=4 {
            down(frame, canvas, x, z, ids);
        }
    }
}

fn balcony(frame: &Frame, canvas: &mut FortressCanvas, ids: &NetherBlocks) {
    brick(frame, canvas, (0, 0, 0), (8, 1, 8), ids);
    clear(frame, canvas, (0, 2, 0), (8, 5, 8), ids);
    brick(frame, canvas, (0, 6, 0), (8, 6, 5), ids);
    brick(frame, canvas, (0, 2, 0), (2, 5, 0), ids);
    brick(frame, canvas, (6, 2, 0), (8, 5, 0), ids);
    fences(frame, canvas, (1, 3, 0), (1, 4, 0), ids);
    fences(frame, canvas, (7, 3, 0), (7, 4, 0), ids);
    brick(frame, canvas, (0, 2, 4), (8, 2, 8), ids);
    clear(frame, canvas, (1, 1, 4), (2, 2, 4), ids);
    clear(frame, canvas, (6, 1, 4), (7, 2, 4), ids);
    fences(frame, canvas, (0, 3, 8), (8, 3, 8), ids);
    fences(frame, canvas, (0, 3, 6), (0, 3, 7), ids);
    fences(frame, canvas, (8, 3, 6), (8, 3, 7), ids);
    brick(frame, canvas, (0, 3, 4), (0, 5, 5), ids);
    brick(frame, canvas, (8, 3, 4), (8, 5, 5), ids);
    brick(frame, canvas, (1, 3, 5), (2, 5, 5), ids);
    brick(frame, canvas, (6, 3, 5), (7, 5, 5), ids);
    fences(frame, canvas, (1, 4, 5), (1, 5, 5), ids);
    fences(frame, canvas, (7, 4, 5), (7, 5, 5), ids);
    for z in 0..=5 {
        for x in 0..=8 {
            down(frame, canvas, x, z, ids);
        }
    }
}
