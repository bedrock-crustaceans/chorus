use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::BoundingBox;

use super::{Cell, Frame, orient_box};

const SEA_LEVEL: i32 = 32;
const HEIGHT_BONUS: i32 = 16;
const MAX_SPAN: i32 = 22;
const FAR_LIMIT: i32 = 112;
const MAX_DEPTH: i32 = 30;
const TRIES: usize = 5;
const START_HEIGHT: i32 = 64;
const START_SPAN: i32 = 18;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PieceKind {
    StraightBridge,
    CrossingBridge,
    BridgeEnd,
    RoomCrossing,
    StairRoom,
    ThroneRoom,
    CastleGate,
    SmallCorridor,
    CorridorCrossing,
    RightTurn,
    LeftTurn,
    CorridorStairs,
    Balcony,
    StalkRoom,
}

struct Weight {
    kind: PieceKind,
    weight: i32,
    placed: i32,
    max: i32,
    in_row: bool,
}

const BRIDGE_WEIGHTS: [(PieceKind, i32, i32, bool); 6] = [
    (PieceKind::StraightBridge, 30, 0, true),
    (PieceKind::CrossingBridge, 10, 4, false),
    (PieceKind::RoomCrossing, 10, 4, false),
    (PieceKind::StairRoom, 10, 3, false),
    (PieceKind::ThroneRoom, 5, 2, false),
    (PieceKind::CastleGate, 5, 1, false),
];

const CASTLE_WEIGHTS: [(PieceKind, i32, i32, bool); 7] = [
    (PieceKind::SmallCorridor, 25, 0, true),
    (PieceKind::CorridorCrossing, 15, 5, false),
    (PieceKind::RightTurn, 5, 10, false),
    (PieceKind::LeftTurn, 5, 10, false),
    (PieceKind::CorridorStairs, 10, 3, true),
    (PieceKind::Balcony, 7, 2, false),
    (PieceKind::StalkRoom, 5, 2, false),
];

pub struct Piece {
    pub kind: PieceKind,
    pub frame: Frame,
    pub chest: bool,
    pub seed: u32,
}

pub struct Fortress {
    pub pieces: Vec<Piece>,
    pub bb: BoundingBox,
}

fn weights(list: &[(PieceKind, i32, i32, bool)]) -> Vec<Weight> {
    list.iter().map(|&(kind, weight, max, in_row)| Weight { kind, weight, placed: 0, max, in_row }).collect()
}

pub fn generate(random: &mut MersenneTwister, (chunk_x, chunk_z): (i32, i32)) -> Fortress {
    let orientation = random.next_int_bounded(4) as u8;
    let (x, z) = (chunk_x * 16 + 2, chunk_z * 16 + 2);
    let bb = BoundingBox::new(x, START_HEIGHT, z, x + START_SPAN, START_HEIGHT + 9, z + START_SPAN);
    let start = Piece {
        kind: PieceKind::CrossingBridge,
        frame: Frame { bb, orientation, depth: 0 },
        chest: false,
        seed: 0,
    };
    let mut builder = Builder {
        pieces: vec![start],
        pending: Vec::new(),
        bridge: weights(&BRIDGE_WEIGHTS),
        castle: weights(&CASTLE_WEIGHTS),
        previous: None,
        start: bb,
    };
    builder.add_children(0, random);
    while !builder.pending.is_empty() {
        let index = builder.pending.remove(random.next_int_bounded(builder.pending.len() as i32) as usize);
        builder.add_children(index, random);
    }
    let union = builder.pieces.iter().skip(1).fold(builder.pieces[0].frame.bb, |union, piece| union.union(&piece.frame.bb));
    let low = SEA_LEVEL + HEIGHT_BONUS;
    let span = MAX_SPAN - (union.y1 - union.y0);
    let y0 = if span > 1 { low + random.next_int_bounded(span) } else { low };
    let shift = y0 - union.y0;
    let mut pieces = builder.pieces;
    pieces.iter_mut().for_each(|piece| piece.frame.bb = piece.frame.bb.shifted_y(shift));
    Fortress { pieces, bb: union.shifted_y(shift) }
}

struct Builder {
    pieces: Vec<Piece>,
    pending: Vec<usize>,
    bridge: Vec<Weight>,
    castle: Vec<Weight>,
    previous: Option<PieceKind>,
    start: BoundingBox,
}

impl Builder {
    fn list(&self, castle: bool) -> &Vec<Weight> {
        if castle { &self.castle } else { &self.bridge }
    }

    fn list_mut(&mut self, castle: bool) -> &mut Vec<Weight> {
        if castle { &mut self.castle } else { &mut self.bridge }
    }

    fn add_children(&mut self, index: usize, random: &mut MersenneTwister) {
        let frame = Frame {
            bb: self.pieces[index].frame.bb,
            orientation: self.pieces[index].frame.orientation,
            depth: self.pieces[index].frame.depth,
        };
        match self.pieces[index].kind {
            PieceKind::CrossingBridge => {
                self.forward(random, &frame, 8, 3, false);
                self.left(random, &frame, 3, 8, false);
                self.right(random, &frame, 3, 8, false);
            }
            PieceKind::StraightBridge => self.forward(random, &frame, 1, 3, false),
            PieceKind::RoomCrossing => {
                self.forward(random, &frame, 2, 0, false);
                self.left(random, &frame, 0, 2, false);
                self.right(random, &frame, 0, 2, false);
            }
            PieceKind::StairRoom => self.right(random, &frame, 6, 2, false),
            PieceKind::CastleGate => self.forward(random, &frame, 5, 3, true),
            PieceKind::StalkRoom => {
                self.forward(random, &frame, 5, 3, true);
                self.forward(random, &frame, 5, 11, true);
            }
            PieceKind::SmallCorridor | PieceKind::CorridorStairs => self.forward(random, &frame, 1, 0, true),
            PieceKind::CorridorCrossing => {
                self.forward(random, &frame, 1, 0, true);
                self.left(random, &frame, 0, 1, true);
                self.right(random, &frame, 0, 1, true);
            }
            PieceKind::RightTurn => self.right(random, &frame, 0, 1, true),
            PieceKind::LeftTurn => self.left(random, &frame, 0, 1, true),
            PieceKind::Balcony => {
                let offset = if matches!(frame.orientation, super::WEST | super::NORTH) { 5 } else { 1 };
                let castle = random.next_int_bounded(8) > 0;
                self.left(random, &frame, 0, offset, castle);
                let castle = random.next_int_bounded(8) > 0;
                self.right(random, &frame, 0, offset, castle);
            }
            PieceKind::BridgeEnd | PieceKind::ThroneRoom => {}
        }
    }

    fn forward(&mut self, random: &mut MersenneTwister, frame: &Frame, dx: i32, dy: i32, castle: bool) {
        let bb = frame.bb;
        let (x, z) = match frame.orientation {
            super::SOUTH => (bb.x0 + dx, bb.z1 + 1),
            super::WEST => (bb.x0 - 1, bb.z0 + dx),
            super::NORTH => (bb.x0 + dx, bb.z0 - 1),
            _ => (bb.x1 + 1, bb.z0 + dx),
        };
        self.add(random, (x, bb.y0 + dy, z), frame.orientation, frame.depth, castle);
    }

    fn left(&mut self, random: &mut MersenneTwister, frame: &Frame, dy: i32, dz: i32, castle: bool) {
        let bb = frame.bb;
        match frame.orientation {
            super::SOUTH | super::NORTH => self.add(random, (bb.x0 - 1, bb.y0 + dy, bb.z0 + dz), super::WEST, frame.depth, castle),
            _ => self.add(random, (bb.x0 + dz, bb.y0 + dy, bb.z0 - 1), super::NORTH, frame.depth, castle),
        }
    }

    fn right(&mut self, random: &mut MersenneTwister, frame: &Frame, dy: i32, dz: i32, castle: bool) {
        let bb = frame.bb;
        match frame.orientation {
            super::SOUTH | super::NORTH => self.add(random, (bb.x1 + 1, bb.y0 + dy, bb.z0 + dz), super::EAST, frame.depth, castle),
            _ => self.add(random, (bb.x0 + dz, bb.y0 + dy, bb.z1 + 1), super::SOUTH, frame.depth, castle),
        }
    }

    fn add(&mut self, random: &mut MersenneTwister, at: Cell, direction: u8, depth: i32, castle: bool) {
        let far = (at.0 - self.start.x0).abs() > FAR_LIMIT || (at.2 - self.start.z0).abs() > FAR_LIMIT;
        let piece = if far {
            self.create(PieceKind::BridgeEnd, random, at, direction, depth)
        } else {
            self.choose(random, at, direction, depth + 1, castle)
        };
        if let Some(piece) = piece {
            self.pending.push(self.pieces.len());
            self.pieces.push(piece);
        }
    }

    fn choose(&mut self, random: &mut MersenneTwister, at: Cell, direction: u8, depth: i32, castle: bool) -> Option<Piece> {
        let total: i32 = self.list(castle).iter().map(|weight| weight.weight).sum();
        let open = self.list(castle).iter().any(|weight| weight.max > 0 && weight.placed < weight.max);
        if open && total > 0 && depth <= MAX_DEPTH {
            for _ in 0..TRIES {
                let mut roll = random.next_int_bounded(total);
                let length = self.list(castle).len();
                for index in 0..length {
                    let weight = &self.list(castle)[index];
                    roll -= weight.weight;
                    if roll >= 0 {
                        continue;
                    }
                    if weight.max != 0 && weight.placed >= weight.max || self.previous == Some(weight.kind) && !weight.in_row {
                        break;
                    }
                    let kind = weight.kind;
                    if let Some(piece) = self.create(kind, random, at, direction, depth) {
                        let list = self.list_mut(castle);
                        list[index].placed += 1;
                        if list[index].max != 0 && list[index].placed >= list[index].max {
                            list.remove(index);
                        }
                        self.previous = Some(kind);
                        return Some(piece);
                    }
                }
            }
        }
        self.create(PieceKind::BridgeEnd, random, at, direction, depth)
    }

    fn create(&self, kind: PieceKind, random: &mut MersenneTwister, at: Cell, direction: u8, depth: i32) -> Option<Piece> {
        let (offset, size) = match kind {
            PieceKind::StraightBridge => ((-1, -3, 0), (5, 10, 19)),
            PieceKind::CrossingBridge => ((-8, -3, 0), (19, 10, 19)),
            PieceKind::BridgeEnd => ((-1, -3, 0), (5, 10, 8)),
            PieceKind::RoomCrossing => ((-2, 0, 0), (7, 9, 7)),
            PieceKind::StairRoom => ((-2, 0, 0), (7, 11, 7)),
            PieceKind::ThroneRoom => ((-2, 0, 0), (7, 8, 9)),
            PieceKind::CastleGate | PieceKind::StalkRoom => ((-5, -3, 0), (13, 14, 13)),
            PieceKind::SmallCorridor | PieceKind::CorridorCrossing | PieceKind::RightTurn | PieceKind::LeftTurn => ((-1, 0, 0), (5, 7, 5)),
            PieceKind::CorridorStairs => ((-1, -7, 0), (5, 14, 10)),
            PieceKind::Balcony => ((-3, 0, 0), (9, 7, 9)),
        };
        let bb = orient_box(at, offset, size, direction);
        if bb.y0 <= 10 || self.pieces.iter().any(|piece| piece.frame.bb.intersects(&bb)) {
            return None;
        }
        let mut piece = Piece {
            kind,
            frame: Frame { bb, orientation: direction, depth },
            chest: false,
            seed: 0,
        };
        match kind {
            PieceKind::RightTurn | PieceKind::LeftTurn => piece.chest = random.next_int_bounded(3) == 0,
            PieceKind::BridgeEnd => piece.seed = random.next_int() as u32,
            _ => {}
        }
        Some(piece)
    }
}
