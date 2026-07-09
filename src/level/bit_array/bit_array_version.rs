#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitArrayVersion {
    V0,
    V1,
    V2,
    V3,
    V4,
    V5,
    V6,
    V8,
    V16,
}

impl BitArrayVersion {
    pub fn bits(self) -> usize {
        match self {
            Self::V0 => 0,
            Self::V1 => 1,
            Self::V2 => 2,
            Self::V3 => 3,
            Self::V4 => 4,
            Self::V5 => 5,
            Self::V6 => 6,
            Self::V8 => 8,
            Self::V16 => 16,
        }
    }

    pub fn entries(self) -> usize {
        match self {
            Self::V0 => 2,
            Self::V1 => 32,
            Self::V2 => 16,
            Self::V3 => 10,
            Self::V4 => 8,
            Self::V5 => 6,
            Self::V6 => 5,
            Self::V8 => 4,
            Self::V16 => 2,
        }
    }

    pub fn next(self) -> Option<Self> {
        match self {
            Self::V0 => Some(Self::V1),
            Self::V1 => Some(Self::V2),
            Self::V2 => Some(Self::V3),
            Self::V3 => Some(Self::V4),
            Self::V4 => Some(Self::V5),
            Self::V5 => Some(Self::V6),
            Self::V6 => Some(Self::V8),
            Self::V8 => Some(Self::V16),
            Self::V16 => None,
        }
    }

    pub fn is_padded(self) -> bool {
        matches!(self, Self::V3 | Self::V5 | Self::V6)
    }

    pub fn max_value(self) -> u32 {
        (1u32 << self.bits()) - 1
    }

    pub fn get_version(val: u8) -> Option<Self> {
        match val {
            0 => Some(Self::V0),
            1 => Some(Self::V1),
            2 => Some(Self::V2),
            3 => Some(Self::V3),
            4 => Some(Self::V4),
            5 => Some(Self::V5),
            6 => Some(Self::V6),
            8 => Some(Self::V8),
            16 => Some(Self::V16),
            _ => None,
        }
    }

    pub fn get_words_for_size(self, size: usize) -> i32 {
        (size as f32 / self.entries() as f32).ceil() as i32
    }
}
