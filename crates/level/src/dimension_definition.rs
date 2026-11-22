use serde::{Deserialize, Serialize};
use std::fmt;

const SECTION: i32 = 16;
const LOWEST_MIN_Y: i32 = -2032;
const HIGHEST_TOP: i32 = 2032;
const HIGHEST_HEIGHT: i32 = 4064;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DimensionDefinition {
    pub min_y: i32,
    pub height: i32,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DefinitionError(String);

impl fmt::Display for DefinitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid dimension definition: {}", self.0)
    }
}

impl std::error::Error for DefinitionError {}

impl DimensionDefinition {
    pub const OVERWORLD: Self = Self { min_y: -64, height: 384 };
    pub const NETHER: Self = Self { min_y: 0, height: 128 };
    pub const END: Self = Self { min_y: 0, height: 256 };

    pub const fn top(self) -> i32 {
        self.min_y + self.height
    }

    pub const fn max_y(self) -> i32 {
        self.top() - 1
    }

    pub const fn min_sub_chunk_y(self) -> i8 {
        (self.min_y >> 4) as i8
    }

    pub const fn max_sub_chunk_y(self) -> i8 {
        ((self.top() >> 4) - 1) as i8
    }

    pub const fn sub_chunk_count(self) -> usize {
        (self.height >> 4) as usize
    }

    pub fn validate(self) -> Result<Self, DefinitionError> {
        let fail = |reason: String| Err(DefinitionError(reason));
        if self.min_y % SECTION != 0 || self.height % SECTION != 0 {
            return fail(format!("min_y {} and height {} must be multiples of {SECTION}", self.min_y, self.height));
        }
        if self.min_y < LOWEST_MIN_Y {
            return fail(format!("min_y {} is below {LOWEST_MIN_Y}", self.min_y));
        }
        if self.height < SECTION || self.height > HIGHEST_HEIGHT {
            return fail(format!("height {} must be between {SECTION} and {HIGHEST_HEIGHT}", self.height));
        }
        if self.top() > HIGHEST_TOP {
            return fail(format!("the top {} is above {HIGHEST_TOP}", self.top()));
        }
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_vanilla_definitions_are_valid() {
        for definition in [DimensionDefinition::OVERWORLD, DimensionDefinition::NETHER, DimensionDefinition::END] {
            assert_eq!(definition.validate(), Ok(definition));
        }
    }

    #[test]
    fn sections_follow_the_range() {
        let overworld = DimensionDefinition::OVERWORLD;
        assert_eq!((overworld.min_sub_chunk_y(), overworld.max_sub_chunk_y(), overworld.sub_chunk_count()), (-4, 19, 24));
        assert_eq!((overworld.top(), overworld.max_y()), (320, 319));
        let deep = DimensionDefinition { min_y: -2032, height: 4064 };
        assert_eq!(deep.validate(), Ok(deep));
        assert_eq!((deep.min_sub_chunk_y(), deep.max_sub_chunk_y(), deep.sub_chunk_count()), (-127, 126, 254));
    }

    #[test]
    fn bad_ranges_are_rejected() {
        for (min_y, height) in [(-60, 384), (-64, 380), (-2048, 400), (0, 0), (0, 4080), (1024, 2048), (-64, -16)] {
            assert!(DimensionDefinition { min_y, height }.validate().is_err(), "{min_y} {height}");
        }
    }
}
