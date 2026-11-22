use crate::shared::structure::place::Rotation;

pub type Pos = (i32, i32, i32);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
    North,
    East,
    South,
    West,
}

const CLOCKWISE: [Side; 4] = [Side::North, Side::East, Side::South, Side::West];
const BY_DRAW: [Side; 4] = [Side::North, Side::South, Side::West, Side::East];

impl Side {
    pub const ALL: [Side; 4] = CLOCKWISE;

    pub fn delta(self) -> (i32, i32) {
        match self {
            Side::North => (0, -1),
            Side::East => (1, 0),
            Side::South => (0, 1),
            Side::West => (-1, 0),
        }
    }

    pub fn quarter_turns(self, turns: i32) -> Side {
        CLOCKWISE[(self as i32 + turns).rem_euclid(4) as usize]
    }

    pub fn right(self) -> Side {
        self.quarter_turns(1)
    }

    pub fn left(self) -> Side {
        self.quarter_turns(-1)
    }

    pub fn behind(self) -> Side {
        self.quarter_turns(2)
    }

    pub fn from_draw(draw: i32) -> Side {
        BY_DRAW[draw as usize]
    }

    pub fn beside(self, (x, y): (i32, i32)) -> (i32, i32) {
        let (dx, dy) = self.delta();
        (x + dx, y + dy)
    }
}

pub fn spin(rotation: Rotation, quarter_turns: i32) -> Rotation {
    Rotation::from_index(rotation as i32 + quarter_turns)
}

pub fn go((x, y, z): Pos, rotation: Rotation, side: Side, blocks: i32) -> Pos {
    let (dx, dz) = side.quarter_turns(rotation as i32).delta();
    (x + dx * blocks, y, z + dz * blocks)
}

pub fn lift((x, y, z): Pos, blocks: i32) -> Pos {
    (x, y + blocks, z)
}
