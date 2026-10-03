use crate::math::enums::plane::Plane;
use strum_macros::{Display, EnumString, VariantNames};

#[derive(Clone, Debug, PartialEq, EnumString, VariantNames, Display)]
#[strum(serialize_all = "snake_case")]
pub enum Axis {
    Y,
    Z,
    X,
}

impl Axis {
    pub fn get_plane(&self) -> Plane {
        match self {
            Axis::Y => Plane::Vertical,
            Axis::Z => Plane::Horizontal,
            Axis::X => Plane::Horizontal,
        }
    }

    pub fn is_vertical(&self) -> bool {
        self.get_plane() == Plane::Vertical
    }

    pub fn is_horizontal(&self) -> bool {
        self.get_plane() == Plane::Horizontal
    }
}
