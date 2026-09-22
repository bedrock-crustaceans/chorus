use crate::error::invalid_index::InvalidIndexError;
use crate::math::enums::block_face::BlockFace;
use strum_macros::{Display, EnumString, VariantNames};

#[derive(Clone, Debug, PartialEq, Eq, Hash, EnumString, VariantNames, Display)]
#[strum(serialize_all = "snake_case")]
pub enum CompassRoseDirection {
    North,
    East,
    South,
    West,
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
    WestNorthWest,
    NorthNorthWest,
    NorthNorthEast,
    EastNorthEast,
    EastSouthEast,
    SouthSouthEast,
    SouthSouthWest,
    WestSouthWest,
}

impl CompassRoseDirection {
    pub fn get_mod_x(&self) -> i8 {
        match self {
            CompassRoseDirection::North => 0,
            CompassRoseDirection::East => 1,
            CompassRoseDirection::South => 0,
            CompassRoseDirection::West => -1,
            CompassRoseDirection::NorthEast => 1,
            CompassRoseDirection::NorthWest => -1,
            CompassRoseDirection::SouthEast => 1,
            CompassRoseDirection::SouthWest => -1,
            CompassRoseDirection::WestNorthWest => -2,
            CompassRoseDirection::NorthNorthWest => -1,
            CompassRoseDirection::NorthNorthEast => 1,
            CompassRoseDirection::EastNorthEast => 2,
            CompassRoseDirection::EastSouthEast => 2,
            CompassRoseDirection::SouthSouthEast => 1,
            CompassRoseDirection::SouthSouthWest => -1,
            CompassRoseDirection::WestSouthWest => -2,
        }
    }

    pub fn get_mod_z(&self) -> i8 {
        match self {
            CompassRoseDirection::North => -1,
            CompassRoseDirection::East => 0,
            CompassRoseDirection::South => 1,
            CompassRoseDirection::West => 0,
            CompassRoseDirection::NorthEast => -1,
            CompassRoseDirection::NorthWest => -1,
            CompassRoseDirection::SouthEast => 1,
            CompassRoseDirection::SouthWest => 1,
            CompassRoseDirection::WestNorthWest => -1,
            CompassRoseDirection::NorthNorthWest => -2,
            CompassRoseDirection::NorthNorthEast => -2,
            CompassRoseDirection::EastNorthEast => -1,
            CompassRoseDirection::EastSouthEast => 1,
            CompassRoseDirection::SouthSouthEast => 2,
            CompassRoseDirection::SouthSouthWest => 2,
            CompassRoseDirection::WestSouthWest => 1,
        }
    }

    pub fn get_closest_face(&self) -> BlockFace {
        match self {
            CompassRoseDirection::North => BlockFace::North,
            CompassRoseDirection::East => BlockFace::East,
            CompassRoseDirection::South => BlockFace::South,
            CompassRoseDirection::West => BlockFace::West,
            CompassRoseDirection::NorthEast => BlockFace::North,
            CompassRoseDirection::NorthWest => BlockFace::West,
            CompassRoseDirection::SouthEast => BlockFace::East,
            CompassRoseDirection::SouthWest => BlockFace::South,
            CompassRoseDirection::WestNorthWest => BlockFace::West,
            CompassRoseDirection::NorthNorthWest => BlockFace::North,
            CompassRoseDirection::NorthNorthEast => BlockFace::North,
            CompassRoseDirection::EastNorthEast => BlockFace::East,
            CompassRoseDirection::EastSouthEast => BlockFace::East,
            CompassRoseDirection::SouthSouthEast => BlockFace::South,
            CompassRoseDirection::SouthSouthWest => BlockFace::South,
            CompassRoseDirection::WestSouthWest => BlockFace::West,
        }
    }

    pub fn get_yaw(&self) -> f32 {
        match self {
            CompassRoseDirection::North => 0.0,
            CompassRoseDirection::East => 90.0,
            CompassRoseDirection::South => 180.0,
            CompassRoseDirection::West => 270.0,
            CompassRoseDirection::NorthEast => 45.0,
            CompassRoseDirection::NorthWest => 315.0,
            CompassRoseDirection::SouthEast => 135.0,
            CompassRoseDirection::SouthWest => 225.0,
            CompassRoseDirection::WestNorthWest => 292.5,
            CompassRoseDirection::NorthNorthWest => 337.5,
            CompassRoseDirection::NorthNorthEast => 22.5,
            CompassRoseDirection::EastNorthEast => 67.5,
            CompassRoseDirection::EastSouthEast => 112.5,
            CompassRoseDirection::SouthSouthEast => 157.5,
            CompassRoseDirection::SouthSouthWest => 202.5,
            CompassRoseDirection::WestSouthWest => 247.5,
        }
    }

    pub fn get_index(&self) -> usize {
        match self {
            CompassRoseDirection::North => 8,
            CompassRoseDirection::East => 12,
            CompassRoseDirection::South => 0,
            CompassRoseDirection::West => 4,
            CompassRoseDirection::NorthEast => 10,
            CompassRoseDirection::NorthWest => 6,
            CompassRoseDirection::SouthEast => 14,
            CompassRoseDirection::SouthWest => 2,
            CompassRoseDirection::WestNorthWest => 5,
            CompassRoseDirection::NorthNorthWest => 7,
            CompassRoseDirection::NorthNorthEast => 9,
            CompassRoseDirection::EastNorthEast => 11,
            CompassRoseDirection::EastSouthEast => 13,
            CompassRoseDirection::SouthSouthEast => 15,
            CompassRoseDirection::SouthSouthWest => 1,
            CompassRoseDirection::WestSouthWest => 3,
        }
    }

    pub fn get_opposite_face(&self) -> CompassRoseDirection {
        match self {
            CompassRoseDirection::North => CompassRoseDirection::South,
            CompassRoseDirection::South => CompassRoseDirection::North,
            CompassRoseDirection::East => CompassRoseDirection::West,
            CompassRoseDirection::West => CompassRoseDirection::East,
            CompassRoseDirection::NorthEast => CompassRoseDirection::SouthWest,
            CompassRoseDirection::NorthWest => CompassRoseDirection::SouthEast,
            CompassRoseDirection::SouthEast => CompassRoseDirection::NorthWest,
            CompassRoseDirection::SouthWest => CompassRoseDirection::NorthEast,
            CompassRoseDirection::WestNorthWest => CompassRoseDirection::EastSouthEast,
            CompassRoseDirection::NorthNorthWest => CompassRoseDirection::SouthSouthEast,
            CompassRoseDirection::NorthNorthEast => CompassRoseDirection::SouthSouthWest,
            CompassRoseDirection::EastNorthEast => CompassRoseDirection::WestSouthWest,
            CompassRoseDirection::EastSouthEast => CompassRoseDirection::WestNorthWest,
            CompassRoseDirection::SouthSouthEast => CompassRoseDirection::NorthNorthWest,
            CompassRoseDirection::SouthSouthWest => CompassRoseDirection::NorthNorthEast,
            CompassRoseDirection::WestSouthWest => CompassRoseDirection::EastNorthEast,
        }
    }

    pub fn from_index(index: usize) -> Result<CompassRoseDirection, InvalidIndexError> {
        match index {
            8 => Ok(CompassRoseDirection::North),
            12 => Ok(CompassRoseDirection::East),
            0 => Ok(CompassRoseDirection::South),
            4 => Ok(CompassRoseDirection::West),
            10 => Ok(CompassRoseDirection::NorthEast),
            6 => Ok(CompassRoseDirection::NorthWest),
            14 => Ok(CompassRoseDirection::SouthEast),
            2 => Ok(CompassRoseDirection::SouthWest),
            5 => Ok(CompassRoseDirection::WestNorthWest),
            7 => Ok(CompassRoseDirection::NorthNorthWest),
            9 => Ok(CompassRoseDirection::NorthNorthEast),
            11 => Ok(CompassRoseDirection::EastNorthEast),
            13 => Ok(CompassRoseDirection::EastSouthEast),
            15 => Ok(CompassRoseDirection::SouthSouthEast),
            1 => Ok(CompassRoseDirection::SouthSouthWest),
            3 => Ok(CompassRoseDirection::WestSouthWest),
            _ => Err(InvalidIndexError(index)),
        }
    }

    pub fn from_closest_yaw(yaw: f32, precision: Option<Precision>) -> Result<CompassRoseDirection, InvalidIndexError> {
        let precision = precision.unwrap_or(Precision::SecondaryInterCardinal);
        let index = (f32::round(f32::round(yaw + 180.0) * precision.get_directions() as f32 / 360.0) * (16.0 / precision.get_directions() as f32)) as usize & 0x0F;
        Self::from_index(index)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, EnumString, VariantNames, Display)]
pub enum Precision {
    Cardinal,
    PrimaryInterCardinal,
    SecondaryInterCardinal,
}

impl Precision {
    pub fn get_directions(&self) -> usize {
        match self {
            Precision::Cardinal => 4,
            Precision::PrimaryInterCardinal => 8,
            Precision::SecondaryInterCardinal => 16,
        }
    }
}
