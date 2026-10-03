use strum_macros::{Display, EnumString, VariantNames};

#[derive(Clone, Debug, PartialEq, EnumString, VariantNames, Display)]
#[strum(serialize_all = "snake_case")]
pub enum AxisDirection {
    Positive,
    Negative,
}

impl AxisDirection {
    pub fn get_offset(&self) -> i8 {
        match self {
            AxisDirection::Positive => 1,
            AxisDirection::Negative => -1,
        }
    }

    pub fn get_description(&self) -> &'static str {
        match self {
            AxisDirection::Positive => "Towards positive",
            AxisDirection::Negative => "Towards negative",
        }
    }
}
