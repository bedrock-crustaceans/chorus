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

    pub const fn min_sub_chunk_y(self) -> i8 {
        match self {
            Self::Overworld => -4,
            Self::Nether | Self::End => 0,
        }
    }

    pub const fn max_sub_chunk_y(self) -> i8 {
        match self {
            Self::Overworld => 19,
            Self::Nether => 7,
            Self::End => 15,
        }
    }

    pub const fn sub_chunk_count(self) -> usize {
        (self.max_sub_chunk_y() as i32 - self.min_sub_chunk_y() as i32 + 1) as usize
    }
}
