use crate::math::enums::block_face::BlockFace;
use rand::RngExt;
use strum_macros::{Display, EnumString, VariantNames};

#[derive(Clone, Debug, PartialEq, EnumString, VariantNames, Display)]
#[strum(serialize_all = "snake_case")]
pub enum Plane {
    Horizontal,
    Vertical,
}

impl Plane {
    pub fn get_faces(&self) -> Vec<BlockFace> {
        match self {
            Plane::Horizontal => vec![BlockFace::North, BlockFace::East, BlockFace::South, BlockFace::West],
            Plane::Vertical => vec![BlockFace::Up, BlockFace::Down],
        }
    }

    pub fn get_random_face(&self) -> BlockFace {
        let faces = self.get_faces();
        faces[rand::rng().random_range(0..faces.len() - 1)].clone()
    }
}
