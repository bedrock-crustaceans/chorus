mod builder;
pub use builder::MonumentBuilder;
mod frame;
mod graph;
mod material;
mod rooms;
mod shell;
mod wing;

use crate::shared::blocks::Blocks;
use crate::shared::chunk::proto::ProtoChunk;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::BoundingBox;
use frame::{Canvas, EAST, Frame, NORTH, SOUTH, WEST};
use graph::{DOWN, Graph, UP};
pub use material::MonumentBlocks;
use rooms::{Room, RoomState};

const ORIENTATIONS: i32 = 4;
const FIRST_FACING: i32 = 2;
const SIZE: i32 = 58;
const MARGIN: i32 = 21;
const DEPTH_BELOW_SEA: i32 = 24;
const HEIGHT_BELOW_SEA: i32 = 2;
const CELL: i32 = 8;
const LEVEL_HEIGHT: i32 = 4;
const ROOM_LIMIT: usize = 75;
const ANCHOR: (i32, i32) = (9, 22);
const WING_LEFT: ((i32, i32, i32), (i32, i32, i32)) = ((1, 1, 1), (23, 8, 21));
const WING_RIGHT: ((i32, i32, i32), (i32, i32, i32)) = ((34, 1, 1), (56, 8, 21));
const PENTHOUSE: ((i32, i32, i32), (i32, i32, i32)) = ((22, 13, 22), (35, 17, 35));

struct Piece {
    frame: Frame,
    room: Room,
    index: usize,
    states: Vec<RoomState>,
}

pub struct Monument {
    pub bounds: BoundingBox,
    building: Frame,
    pieces: Vec<Piece>,
}

fn orientation_of(facing: i32) -> u8 {
    match facing {
        2 => NORTH,
        3 => SOUTH,
        4 => WEST,
        _ => EAST,
    }
}

impl Monument {
    pub fn generate(level_seed: u32, (chunk_x, chunk_z): (i32, i32), sea_level: i32) -> Self {
        let mut seeder = MersenneTwister::new(level_seed);
        let (a, b) = (seeder.next_float(), seeder.next_float());
        let seed = (a * chunk_x as f32) as i64 as u32 ^ (b * chunk_z as f32) as i64 as u32 ^ level_seed;
        let mut random = MersenneTwister::new(seed);
        let facing = random.next_int_bounded(ORIENTATIONS) + FIRST_FACING;
        let orientation = orientation_of(facing);
        let (x0, z0) = (chunk_x * 16 - MARGIN, chunk_z * 16 - MARGIN);
        let bounds = BoundingBox::new(x0, sea_level - DEPTH_BELOW_SEA, z0, x0 + SIZE - 1, sea_level - HEIGHT_BELOW_SEA, z0 + SIZE - 1);
        let building = Frame { bounds, orientation };
        let mut graph = Graph::build(&mut random, seed);
        let mut pieces = Vec::new();
        let source = graph::slot(2, 0, 0);
        graph.claim(&[source]);
        pieces.push(Self::room_piece(&graph, &[source], orientation, Room::Entry, (1, 1, 1)));
        pieces.push(Self::room_piece(&graph, &[graph.core], orientation, Room::Core, (2, 2, 2)));
        for slot in graph.order.clone() {
            let room = graph.room(slot);
            if !room.claimed && room.grid_index < ROOM_LIMIT {
                pieces.push(Self::fit(&mut graph, slot, orientation, &mut random));
            }
        }
        let (dx, dy, dz) = (building.world_x(ANCHOR.0, ANCHOR.1), bounds.y0, building.world_z(ANCHOR.0, ANCHOR.1));
        for piece in &mut pieces {
            let b = &mut piece.frame.bounds;
            *b = BoundingBox::new(b.x0 + dx, b.y0 + dy, b.z0 + dz, b.x1 + dx, b.y1 + dy, b.z1 + dz);
        }
        let design = (random.next_int() & 1) as u8;
        let corners = |(from, to): ((i32, i32, i32), (i32, i32, i32))| {
            let (p, q) = (building.world(from), building.world(to));
            BoundingBox::new(p.0.min(q.0), p.1.min(q.1), p.2.min(q.2), p.0.max(q.0), p.1.max(q.1), p.2.max(q.2))
        };
        for (area, room) in [(WING_LEFT, Room::Wing(design)), (WING_RIGHT, Room::Wing(design ^ 1)), (PENTHOUSE, Room::Penthouse)] {
            pieces.push(Piece {
                frame: Frame { bounds: corners(area), orientation },
                room,
                index: 0,
                states: Vec::new(),
            });
        }
        Self { bounds, building, pieces }
    }

    fn room_piece(graph: &Graph, slots: &[usize], orientation: u8, room: Room, (w, h, d): (i32, i32, i32)) -> Piece {
        let index = graph.room(slots[0]).grid_index;
        let (x, z, y) = ((index % 5) as i32, (index / 5 % 5) as i32, (index / 25) as i32);
        let (span_x, span_z) = if orientation == NORTH || orientation == SOUTH { (w, d) } else { (d, w) };
        let (x0, z0) = match orientation {
            WEST => (-(z + d) * CELL + 1, x * CELL),
            SOUTH => (x * CELL, z * CELL),
            NORTH => (x * CELL, -(z + d) * CELL + 1),
            _ => (z * CELL, x * CELL),
        };
        let bounds = BoundingBox::new(x0, y * LEVEL_HEIGHT, z0, x0 + span_x * CELL - 1, y * LEVEL_HEIGHT + h * LEVEL_HEIGHT - 1, z0 + span_z * CELL - 1);
        let states = slots
            .iter()
            .map(|&slot| RoomState {
                open: graph.room(slot).open,
                has_room_above: graph.room(slot).links[UP].is_some(),
            })
            .collect();
        Piece {
            frame: Frame { bounds, orientation },
            room,
            index,
            states,
        }
    }

    fn fit(graph: &mut Graph, slot: usize, orientation: u8, random: &mut MersenneTwister) -> Piece {
        use graph::{EAST as E, NORTH as N};
        let (room, slots, size) = if graph.is_free(slot, E) && graph.is_free(slot, UP) && graph.is_free(graph.through(slot, E), UP) {
            let (east, above) = (graph.through(slot, E), graph.through(slot, UP));
            (Room::DoubleXY, vec![slot, east, above, graph.through(east, UP)], (2, 2, 1))
        } else if graph.is_free(slot, N) && graph.is_free(slot, UP) && graph.is_free(graph.through(slot, N), UP) {
            let (north, above) = (graph.through(slot, N), graph.through(slot, UP));
            (Room::DoubleYZ, vec![slot, north, above, graph.through(north, UP)], (1, 2, 2))
        } else if graph.is_free(slot, N) {
            (Room::DoubleZ, vec![slot, graph.through(slot, N)], (1, 1, 2))
        } else if graph.is_free(slot, E) {
            (Room::DoubleX, vec![slot, graph.through(slot, E)], (2, 1, 1))
        } else if graph.is_free(slot, UP) {
            (Room::DoubleY, vec![slot, graph.through(slot, UP)], (1, 2, 1))
        } else if graph.room(slot).open.iter().enumerate().all(|(face, &open)| face == DOWN || !open) {
            (Room::SimpleTop, vec![slot], (1, 1, 1))
        } else {
            (Room::Simple(random.next_int_bounded(3) as u8), vec![slot], (1, 1, 1))
        };
        graph.claim(&slots);
        Self::room_piece(graph, &slots, orientation, room, size)
    }

    fn place(&self, canvas: &mut Canvas, random: &mut MersenneTwister) {
        shell::build(&self.building, canvas);
        let column = canvas.bounds;
        for piece in self.pieces.iter().filter(|piece| piece.frame.bounds.intersects(&column)) {
            let frame = &piece.frame;
            match piece.room {
                Room::Wing(design) => wing::build(frame, design, canvas),
                _ => rooms::build(piece.room, frame, piece.index, &piece.states, canvas, random),
            }
        }
    }
}

pub fn place(blocks: &Blocks, ids: &MonumentBlocks, chunk: &mut ProtoChunk, monuments: &[std::sync::Arc<Monument>], random: &mut MersenneTwister, sea_level: i32) {
    let mut canvas = Canvas::new(chunk, blocks, ids, sea_level);
    for monument in monuments {
        monument.place(&mut canvas, random);
    }
}
