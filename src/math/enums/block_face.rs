use crate::error::invalid_index::InvalidIndexError;
use crate::math::enums::axis::Axis;
use crate::math::enums::axis_direction::AxisDirection;
use crate::math::enums::compass_rose_direction::CompassRoseDirection;
use crate::math::enums::plane::Plane;
use std::collections::HashSet;
use strum_macros::{Display, EnumString, VariantNames};
use glam::Vec3;

#[derive(Clone, Debug, PartialEq, Eq, Hash, EnumString, VariantNames, Display)]
#[strum(serialize_all = "snake_case")]
pub enum BlockFace {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl BlockFace {
    pub fn get_index(&self) -> usize {
        match self {
            BlockFace::Down => 0,
            BlockFace::Up => 1,
            BlockFace::North => 2,
            BlockFace::South => 3,
            BlockFace::West => 4,
            BlockFace::East => 5,
        }
    }

    pub fn get_index_dusnew(&self) -> usize {
        match self {
            BlockFace::Down => 0,
            BlockFace::Up => 1,
            BlockFace::North => 2,
            BlockFace::South => 4,
            BlockFace::West => 5,
            BlockFace::East => 3,
        }
    }

    pub fn get_index_dusnwe(&self) -> usize {
        match self {
            BlockFace::Down => 0,
            BlockFace::Up => 1,
            BlockFace::North => 4,
            BlockFace::South => 2,
            BlockFace::West => 3,
            BlockFace::East => 5,
        }
    }

    pub fn get_opposite(&self) -> BlockFace {
        match self {
            BlockFace::Down => BlockFace::Up,
            BlockFace::Up => BlockFace::Down,
            BlockFace::North => BlockFace::South,
            BlockFace::South => BlockFace::North,
            BlockFace::West => BlockFace::East,
            BlockFace::East => BlockFace::West,
        }
    }

    pub fn get_opposite_index(&self) -> usize {
        self.get_opposite().get_index()
    }

    pub fn get_horizontal_index(&self) -> Option<usize> {
        match self {
            BlockFace::South => Some(0),
            BlockFace::West => Some(1),
            BlockFace::North => Some(2),
            BlockFace::East => Some(3),
            _ => None,
        }
    }

    pub fn get_horizontal_angle(&self) -> Option<f32> {
        match self {
            BlockFace::South => Some(0.0),
            BlockFace::West => Some(90.0),
            BlockFace::North => Some(180.0),
            BlockFace::East => Some(270.0),
            _ => None,
        }
    }

    pub fn get_x_offset(&self) -> i8 {
        if self.get_axis() == Axis::X { self.get_axis_direction().get_offset() } else { 0 }
    }

    pub fn get_y_offset(&self) -> i8 {
        if self.get_axis() == Axis::Y { self.get_axis_direction().get_offset() } else { 0 }
    }

    pub fn get_z_offset(&self) -> i8 {
        if self.get_axis() == Axis::Z { self.get_axis_direction().get_offset() } else { 0 }
    }

    pub fn get_axis(&self) -> Axis {
        match self {
            BlockFace::Down => Axis::Y,
            BlockFace::Up => Axis::Y,
            BlockFace::North => Axis::Z,
            BlockFace::South => Axis::Z,
            BlockFace::West => Axis::X,
            BlockFace::East => Axis::X,
        }
    }

    pub fn get_axis_direction(&self) -> AxisDirection {
        match self {
            BlockFace::Down => AxisDirection::Negative,
            BlockFace::Up => AxisDirection::Positive,
            BlockFace::North => AxisDirection::Negative,
            BlockFace::South => AxisDirection::Positive,
            BlockFace::West => AxisDirection::Negative,
            BlockFace::East => AxisDirection::Positive,
        }
    }

    pub fn get_unit_vec(&self) -> Vec3 {
        match self {
            BlockFace::Down => Vec3::new(0.0, -1.0, 0.0),
            BlockFace::Up => Vec3::new(0.0, 1.0, 0.0),
            BlockFace::North => Vec3::new(0.0, 0.0, -1.0),
            BlockFace::South => Vec3::new(0.0, 0.0, 1.0),
            BlockFace::West => Vec3::new(-1.0, 0.0, 0.0),
            BlockFace::East => Vec3::new(1.0, 0.0, 0.0),
        }
    }

    pub fn rotate_y_cw(&self) -> BlockFace {
        match self {
            BlockFace::North => BlockFace::East,
            BlockFace::South => BlockFace::West,
            BlockFace::West => BlockFace::North,
            BlockFace::East => BlockFace::South,
            _ => self.clone(),
        }
    }

    pub fn rotate_y_ccw(&self) -> BlockFace {
        match self {
            BlockFace::North => BlockFace::West,
            BlockFace::South => BlockFace::East,
            BlockFace::West => BlockFace::South,
            BlockFace::East => BlockFace::North,
            _ => self.clone(),
        }
    }

    pub fn get_compass_rose_direction(&self) -> Option<CompassRoseDirection> {
        match self {
            BlockFace::North => Some(CompassRoseDirection::North),
            BlockFace::South => Some(CompassRoseDirection::South),
            BlockFace::West => Some(CompassRoseDirection::West),
            BlockFace::East => Some(CompassRoseDirection::East),
            _ => None,
        }
    }

    pub fn get_edges(&self) -> HashSet<BlockFace> {
        let mut block_faces: HashSet<BlockFace> = HashSet::new();
        if self.get_axis().is_vertical() {
            Plane::Horizontal.get_faces().iter().for_each(|face| {
                block_faces.insert(face.clone());
            });
            return block_faces;
        }
        Plane::Vertical.get_faces().iter().for_each(|face| {
            block_faces.insert(face.clone());
        });

        let edge_axis = if self.get_axis() == Axis::X { Axis::Z } else { Axis::X };
        block_faces.insert(Self::from_axis(&AxisDirection::Negative, &edge_axis));
        block_faces.insert(Self::from_axis(&AxisDirection::Positive, &edge_axis));
        block_faces
    }

    pub fn from_index(index: usize) -> Result<BlockFace, InvalidIndexError> {
        match index {
            0 => Ok(BlockFace::Down),
            1 => Ok(BlockFace::Up),
            2 => Ok(BlockFace::North),
            3 => Ok(BlockFace::South),
            4 => Ok(BlockFace::West),
            5 => Ok(BlockFace::East),
            _ => Err(InvalidIndexError(index)),
        }
    }

    pub fn from_horizontal_index(index: usize) -> Result<BlockFace, InvalidIndexError> {
        match index {
            0 => Ok(BlockFace::South),
            1 => Ok(BlockFace::West),
            2 => Ok(BlockFace::North),
            3 => Ok(BlockFace::East),
            _ => Err(InvalidIndexError(index)),
        }
    }

    pub fn from_horizontal_angle(angle: f32) -> Result<BlockFace, InvalidIndexError> {
        Self::from_horizontal_index(f32::floor(angle / 90.0 + 0.5) as usize & 3)
    }

    pub fn from_axis(axis_direction: &AxisDirection, axis: &Axis) -> Self {
        match axis_direction {
            AxisDirection::Positive => match axis {
                Axis::Y => BlockFace::Up,
                Axis::Z => BlockFace::South,
                Axis::X => BlockFace::East,
            },
            AxisDirection::Negative => match axis {
                Axis::Y => BlockFace::Down,
                Axis::Z => BlockFace::North,
                Axis::X => BlockFace::West,
            },
        }
    }
}
