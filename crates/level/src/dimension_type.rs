use crate::dimension_definition::DimensionDefinition;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DimensionType {
    Overworld,
    Nether,
    End,
}

impl DimensionType {
    pub const fn from_id(id: i32) -> Option<Self> {
        match id {
            0 => Some(Self::Overworld),
            1 => Some(Self::Nether),
            2 => Some(Self::End),
            _ => None,
        }
    }

    pub const fn id(self) -> i32 {
        match self {
            Self::Overworld => 0,
            Self::Nether => 1,
            Self::End => 2,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Overworld => "overworld",
            Self::Nether => "nether",
            Self::End => "the_end",
        }
    }

    pub const fn definition(self) -> DimensionDefinition {
        match self {
            Self::Overworld => DimensionDefinition::OVERWORLD,
            Self::Nether => DimensionDefinition::NETHER,
            Self::End => DimensionDefinition::END,
        }
    }

    pub const fn min_sub_chunk_y(self) -> i8 {
        self.definition().min_sub_chunk_y()
    }

    pub const fn max_sub_chunk_y(self) -> i8 {
        self.definition().max_sub_chunk_y()
    }

    pub const fn sub_chunk_count(self) -> usize {
        self.definition().sub_chunk_count()
    }
}
