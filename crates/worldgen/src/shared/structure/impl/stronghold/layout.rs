use super::frame::{EAST, Frame, NORTH, SOUTH, WEST, orient_box};
use super::{Door, Kind, Piece, Stronghold};
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::BoundingBox;
use chorus_level::dimension_definition::DimensionDefinition;

const SURFACE_MARGIN: i32 = 10;
const MIN_PIECE_Y: i32 = 10;
const MIN_FILLER_Y: i32 = 1;
const MAX_DEPTH: i32 = 50;
const MAX_DISTANCE: i32 = 112;
const PICK_ATTEMPTS: usize = 5;
const START_Y: i32 = 64;
const START_INSET: i32 = 2;
const START_SPAN: i32 = 4;
const START_HEIGHT: i32 = 10;
const DOOR_ROLLS: i32 = 5;
const PORTAL_FALLBACK_HEIGHT: i32 = 7;
const PORTAL_FALLBACK_LENGTH: i32 = 15;
const FILLER_WIDTH: i32 = 5;
const FILLER_MAX_LENGTH: i32 = 3;
const LIBRARY_TALL_HEIGHT: i32 = 11;
const LIBRARY_LOW_HEIGHT: i32 = 6;

struct Entry {
    kind: Kind,
    weight: i32,
    most: i32,
    least_depth: i32,
    placed: i32,
}

impl Entry {
    const fn new(kind: Kind, weight: i32, most: i32, least_depth: i32) -> Self {
        Self {
            kind,
            weight,
            most,
            least_depth,
            placed: 0,
        }
    }

    fn exhausted(&self) -> bool {
        self.most != 0 && self.placed >= self.most
    }
}

fn entries() -> Vec<Entry> {
    vec![
        Entry::new(Kind::Straight, 40, 0, 0),
        Entry::new(Kind::PrisonHall, 5, 5, 0),
        Entry::new(Kind::LeftTurn, 20, 0, 0),
        Entry::new(Kind::RightTurn, 20, 0, 0),
        Entry::new(Kind::RoomCrossing, 10, 6, 0),
        Entry::new(Kind::StraightStairsDown, 5, 5, 0),
        Entry::new(Kind::StairsDown, 5, 5, 0),
        Entry::new(Kind::FiveCrossing, 5, 4, 0),
        Entry::new(Kind::ChestCorridor, 5, 4, 0),
        Entry::new(Kind::Library, 10, 2, 5),
        Entry::new(Kind::PortalRoom, 10, 1, 6),
    ]
}

fn piece_frame(bounds: BoundingBox, orientation: u8, depth: i32) -> Frame {
    Frame { bounds, orientation, depth }
}

fn random_door(random: &mut MersenneTwister) -> Door {
    match random.next_int_bounded(DOOR_ROLLS) {
        2 => Door::Wood,
        3 => Door::Grates,
        4 => Door::Iron,
        _ => Door::Opening,
    }
}

pub fn generate(random: &mut MersenneTwister, (chunk_x, chunk_z): (i32, i32), definition: DimensionDefinition, sea_level: i32) -> Stronghold {
    let orientation = random.next_int_bounded(4) as u8;
    let (x, z) = (chunk_x * 16, chunk_z * 16);
    let bounds = BoundingBox::new(
        x + START_INSET,
        START_Y,
        z + START_INSET,
        x + START_INSET + START_SPAN,
        START_Y + START_HEIGHT,
        z + START_INSET + START_SPAN,
    );
    let first = Piece {
        kind: Kind::StairsDown,
        frame: piece_frame(bounds, orientation, 0),
        door: Door::Opening,
        flags: [true, false, false, false],
        value: 0,
    };
    let mut builder = Builder {
        pieces: vec![first],
        pending: Vec::new(),
        entries: entries(),
        previous: None,
        imposed: None,
        start: bounds,
    };
    builder.add_children(0, random);
    while !builder.pending.is_empty() {
        let slot = random.next_int_bounded(builder.pending.len() as i32) as usize;
        let index = builder.pending.remove(slot);
        builder.add_children(index, random);
    }
    if !builder.pieces.iter().any(|piece| piece.kind == Kind::PortalRoom) {
        builder.portal_fallback();
    }
    let all = builder.pieces.iter().fold(builder.pieces[0].frame.bounds, |all, piece| all.union(&piece.frame.bounds));
    let mut top = all.y1 - all.y0 + definition.min_y + 2;
    if top < sea_level - SURFACE_MARGIN {
        top += random.next_int_bounded(sea_level - SURFACE_MARGIN - top);
    }
    let shift = top - all.y1;
    builder.pieces.iter_mut().for_each(|piece| piece.frame.bounds = piece.frame.bounds.shifted_y(shift));
    Stronghold {
        pieces: builder.pieces,
        bounds: all.shifted_y(shift),
    }
}

struct Builder {
    pieces: Vec<Piece>,
    pending: Vec<usize>,
    entries: Vec<Entry>,
    previous: Option<Kind>,
    imposed: Option<Kind>,
    start: BoundingBox,
}

impl Builder {
    fn add_children(&mut self, index: usize, random: &mut MersenneTwister) {
        let piece = self.pieces[index].clone();
        let orientation = piece.frame.orientation;
        let flipped = orientation == NORTH || orientation == EAST;
        match piece.kind {
            Kind::StairsDown => {
                if piece.flags[0] {
                    self.imposed = Some(Kind::FiveCrossing);
                }
                self.forward(random, &piece, 1);
            }
            Kind::Straight => {
                self.forward(random, &piece, 1);
                if piece.flags[0] {
                    self.left(random, &piece, 1, 2);
                }
                if piece.flags[1] {
                    self.right(random, &piece, 1, 2);
                }
            }
            Kind::ChestCorridor | Kind::PrisonHall | Kind::StraightStairsDown => self.forward(random, &piece, 1),
            Kind::LeftTurn if flipped => self.left(random, &piece, 1, 1),
            Kind::LeftTurn => self.right(random, &piece, 1, 1),
            Kind::RightTurn if flipped => self.right(random, &piece, 1, 1),
            Kind::RightTurn => self.left(random, &piece, 1, 1),
            Kind::RoomCrossing => {
                self.forward(random, &piece, 4);
                self.left(random, &piece, 1, 4);
                self.right(random, &piece, 1, 4);
            }
            Kind::FiveCrossing => {
                let (low, high) = if orientation == WEST || orientation == NORTH { (5, 3) } else { (3, 5) };
                self.forward(random, &piece, 5);
                let [left_low, left_high, right_low, right_high] = piece.flags;
                if left_low {
                    self.left(random, &piece, low, 1);
                }
                if left_high {
                    self.left(random, &piece, high, 7);
                }
                if right_low {
                    self.right(random, &piece, low, 1);
                }
                if right_high {
                    self.right(random, &piece, high, 7);
                }
            }
            Kind::Library | Kind::PortalRoom | Kind::Filler => {}
        }
    }

    fn forward(&mut self, random: &MersenneTwister, piece: &Piece, offset: i32) {
        let (bounds, orientation) = (piece.frame.bounds, piece.frame.orientation);
        let (x, z) = match orientation {
            SOUTH => (bounds.x0 + offset, bounds.z1 + 1),
            WEST => (bounds.x0 - 1, bounds.z0 + offset),
            NORTH => (bounds.x0 + offset, bounds.z0 - 1),
            _ => (bounds.x1 + 1, bounds.z0 + offset),
        };
        self.piece_at_door(random, (x, bounds.y0 + 1, z), orientation, piece.frame.depth);
    }

    fn left(&mut self, random: &MersenneTwister, piece: &Piece, dy: i32, offset: i32) {
        let bounds = piece.frame.bounds;
        match piece.frame.orientation {
            SOUTH | NORTH => self.piece_at_door(random, (bounds.x0 - 1, bounds.y0 + dy, bounds.z0 + offset), WEST, piece.frame.depth),
            _ => self.piece_at_door(random, (bounds.x0 + offset, bounds.y0 + dy, bounds.z0 - 1), NORTH, piece.frame.depth),
        }
    }

    fn right(&mut self, random: &MersenneTwister, piece: &Piece, dy: i32, offset: i32) {
        let bounds = piece.frame.bounds;
        match piece.frame.orientation {
            SOUTH | NORTH => self.piece_at_door(random, (bounds.x1 + 1, bounds.y0 + dy, bounds.z0 + offset), EAST, piece.frame.depth),
            _ => self.piece_at_door(random, (bounds.x0 + offset, bounds.y0 + dy, bounds.z1 + 1), SOUTH, piece.frame.depth),
        }
    }

    fn piece_at_door(&mut self, random: &MersenneTwister, at: (i32, i32, i32), orientation: u8, depth: i32) {
        if depth > MAX_DEPTH || (at.0 - self.start.x0).abs() > MAX_DISTANCE || (at.2 - self.start.z0).abs() > MAX_DISTANCE {
            return;
        }
        let mut sibling = random.clone();
        if let Some(piece) = self.choose(&mut sibling, at, orientation, depth + 1) {
            self.pending.push(self.pieces.len());
            self.pieces.push(piece);
        }
    }

    fn choose(&mut self, random: &mut MersenneTwister, at: (i32, i32, i32), orientation: u8, depth: i32) -> Option<Piece> {
        if let Some(kind) = self.imposed.take()
            && let Some(piece) = self.create(kind, random, at, orientation, depth)
        {
            return Some(piece);
        }
        let total: i32 = self.entries.iter().map(|entry| entry.weight).sum();
        if !self.entries.iter().any(|entry| entry.most > 0 && entry.placed < entry.most) || total <= 0 {
            return None;
        }
        for _ in 0..PICK_ATTEMPTS {
            let mut roll = (random.next_u32() % total as u32) as i32;
            for index in 0..self.entries.len() {
                roll -= self.entries[index].weight;
                if roll >= 0 {
                    continue;
                }
                let entry = &self.entries[index];
                let repeats = self.previous == Some(entry.kind) && self.entries.len() > 1;
                if entry.exhausted() || depth < entry.least_depth || repeats {
                    break;
                }
                let kind = entry.kind;
                if let Some(piece) = self.create(kind, random, at, orientation, depth) {
                    self.entries[index].placed += 1;
                    self.previous = Some(kind);
                    if self.entries[index].exhausted() {
                        self.entries.remove(index);
                    }
                    return Some(piece);
                }
            }
        }
        self.filler(at, orientation, depth)
    }

    fn collides(&self, bounds: &BoundingBox) -> bool {
        self.pieces.iter().any(|piece| piece.frame.bounds.intersects(bounds))
    }

    fn create(&self, kind: Kind, random: &mut MersenneTwister, at: (i32, i32, i32), orientation: u8, depth: i32) -> Option<Piece> {
        let sized = |offset, size| Some(orient_box(at, offset, size, orientation)).filter(|bounds| bounds.y0 > MIN_PIECE_Y && !self.collides(bounds));
        let bounds = match kind {
            Kind::Straight | Kind::ChestCorridor => sized((-1, -1, 0), (5, 5, 7)),
            Kind::PrisonHall => sized((-1, -1, 0), (9, 5, 11)),
            Kind::LeftTurn | Kind::RightTurn => sized((-1, -1, 0), (5, 5, 5)),
            Kind::RoomCrossing => sized((-4, -1, 0), (11, 7, 11)),
            Kind::StraightStairsDown => sized((-1, -7, 0), (5, 11, 8)),
            Kind::StairsDown => sized((-1, -7, 0), (5, 11, 5)),
            Kind::FiveCrossing => sized((-4, -3, 0), (10, 9, 11)),
            Kind::Library => sized((-4, -1, 0), (14, LIBRARY_TALL_HEIGHT, 15)).or_else(|| sized((-4, -1, 0), (14, LIBRARY_LOW_HEIGHT, 15))),
            Kind::PortalRoom => sized((-4, -1, 0), (11, 8, 16)),
            Kind::Filler => None,
        }?;
        let door = if kind == Kind::PortalRoom { Door::Opening } else { random_door(random) };
        let mut piece = Piece {
            kind,
            frame: piece_frame(bounds, orientation, depth),
            door,
            flags: [false; 4],
            value: 0,
        };
        match kind {
            Kind::Straight => piece.flags = [random.next_int_bounded(2) == 0, random.next_int_bounded(2) == 0, false, false],
            Kind::RoomCrossing => piece.value = random.next_int_bounded(5),
            Kind::FiveCrossing => {
                let left_low = random.next_boolean();
                let left_high = random.next_boolean();
                let right_low = random.next_boolean();
                piece.flags = [left_low, left_high, right_low, random.next_int_bounded(3) > 0];
            }
            Kind::Library => piece.flags[0] = bounds.y1 - bounds.y0 > LIBRARY_LOW_HEIGHT - 1,
            _ => {}
        }
        Some(piece)
    }

    fn filler(&self, at: (i32, i32, i32), orientation: u8, depth: i32) -> Option<Piece> {
        let probe = orient_box(at, (-1, -1, 0), (FILLER_WIDTH, FILLER_WIDTH, FILLER_MAX_LENGTH + 1), orientation);
        let other = self.pieces.iter().find(|piece| piece.frame.bounds.intersects(&probe))?.frame.bounds;
        if other.y0 != probe.y0 {
            return None;
        }
        let length = (1..=FILLER_MAX_LENGTH)
            .rev()
            .find(|&length| !other.intersects(&orient_box(at, (-1, -1, 0), (FILLER_WIDTH, FILLER_WIDTH, length - 1), orientation)))?;
        let bounds = orient_box(at, (-1, -1, 0), (FILLER_WIDTH, FILLER_WIDTH, length), orientation);
        (bounds.y0 > MIN_FILLER_Y).then(|| Piece {
            kind: Kind::Filler,
            frame: piece_frame(bounds, orientation, depth),
            door: Door::Opening,
            flags: [false; 4],
            value: length,
        })
    }

    fn portal_fallback(&mut self) {
        if self.pieces.len() < 2 {
            return;
        }
        let removed = self.pieces.pop().expect("two pieces");
        let orientation = removed.frame.orientation;
        let last = self.pieces[self.pieces.len() - 1].frame;
        let bounds = last.bounds;
        let (mid_x, mid_z) = ((bounds.x0 + bounds.x1) / 2, (bounds.z0 + bounds.z1) / 2);
        let (x0, z0, x1, z1) = match orientation {
            SOUTH => (mid_x - 3, bounds.z1, mid_x + 7, bounds.z1 + PORTAL_FALLBACK_LENGTH),
            WEST => (bounds.x0 - PORTAL_FALLBACK_LENGTH, mid_z - 5, bounds.x0, mid_z + 5),
            NORTH => (mid_x - 5, bounds.z0 - PORTAL_FALLBACK_LENGTH, mid_x + 5, bounds.z0),
            _ => (bounds.x1, mid_z - 3, bounds.x1 + PORTAL_FALLBACK_LENGTH, mid_z + 7),
        };
        if bounds.y0 <= MIN_PIECE_Y {
            return;
        }
        let portal = BoundingBox::new(x0, bounds.y0, z0, x1, bounds.y0 + PORTAL_FALLBACK_HEIGHT, z1);
        self.pieces.push(Piece {
            kind: Kind::PortalRoom,
            frame: piece_frame(portal, orientation, last.depth),
            door: Door::Opening,
            flags: [false; 4],
            value: 0,
        });
    }
}
