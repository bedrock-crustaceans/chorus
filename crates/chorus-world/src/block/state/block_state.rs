use atomicow::CowArc;
use std::collections::HashSet;
use std::hash::Hash;
use std::sync::Arc;

#[derive(Debug)]
pub enum BlockStateDefinition {
    Bool {
        identifier: CowArc<'static, str>,
        default: bool,
    },
    Int {
        identifier: CowArc<'static, str>,
        min: i32,
        max: i32,
    },
    Enum {
        identifier: CowArc<'static, str>,
        values: CowArc<'static, [CowArc<'static, str>]>,
    },
}

#[derive(Eq, PartialEq, Clone, Debug, Hash)]
pub enum BlockState {
    Bool(bool),
    Int(i32),
    Enum(CowArc<'static, str>),
}

impl BlockStateDefinition {
    pub fn new_int(identifier: impl AsRef<str>, min: i32, max: i32) -> BlockStateDefinition {
        let (min, max) = if min < max { (min, max) } else { (max, min) };
        BlockStateDefinition::Int {
            identifier: CowArc::Owned(Arc::from(identifier.as_ref())),
            min,
            max,
        }
    }

    pub fn new_bool(identifier: impl AsRef<str>, default: bool) -> BlockStateDefinition {
        BlockStateDefinition::Bool {
            identifier: CowArc::Owned(Arc::from(identifier.as_ref())),
            default,
        }
    }

    pub fn new_enum(identifier: impl AsRef<str>, values: impl IntoIterator<Item = impl AsRef<str>>) -> BlockStateDefinition {
        BlockStateDefinition::Enum {
            identifier: CowArc::Owned(Arc::from(identifier.as_ref())),
            values: CowArc::Owned(values.into_iter().map(|v| CowArc::Owned(Arc::from(v.as_ref()))).collect::<Vec<_>>().into()),
        }
    }

    pub fn identifier(&self) -> &CowArc<'static, str> {
        match self {
            BlockStateDefinition::Int { identifier, .. } => identifier,
            BlockStateDefinition::Bool { identifier, .. } => identifier,
            BlockStateDefinition::Enum { identifier, .. } => identifier,
        }
    }

    pub fn index_of(&self, state: &BlockState) -> i32 {
        match (self, state) {
            (BlockStateDefinition::Int { min, .. }, BlockState::Int(val)) => *val - *min,
            (BlockStateDefinition::Bool { default, .. }, BlockState::Bool(val)) => {
                if *val == *default {
                    0
                } else {
                    1
                }
            }
            (BlockStateDefinition::Enum { values, .. }, BlockState::Enum(val)) => match values.iter().position(|x| x == val) {
                Some(pos) => pos as i32,
                _ => -1,
            },
            _ => -1,
        }
    }

    pub fn values_len(&self) -> usize {
        match self {
            BlockStateDefinition::Bool { .. } => 2,
            BlockStateDefinition::Int { min, max, .. } => (*max - *min + 1) as usize,
            BlockStateDefinition::Enum { values, .. } => values.len(),
        }
    }

    pub fn get_values(&self) -> Vec<BlockState> {
        match self {
            BlockStateDefinition::Bool { default, .. } => {
                vec![BlockState::Bool(*default), BlockState::Bool(!*default)]
            }
            BlockStateDefinition::Int { min, max, .. } => (*min..=*max).map(BlockState::Int).collect(),
            BlockStateDefinition::Enum { values, .. } => values.iter().map(|v| BlockState::Enum(v.clone())).collect(),
        }
    }

    pub fn get_int_values(&self) -> Option<Vec<i32>> {
        match self {
            BlockStateDefinition::Int { min, max, .. } => Some((*min..=*max).collect()),
            _ => None,
        }
    }

    pub fn get_bool_values(&self) -> Option<Vec<bool>> {
        match self {
            BlockStateDefinition::Bool { default, .. } => Some(vec![*default, !*default]),
            _ => None,
        }
    }

    pub fn get_enum_values(&self) -> Option<Vec<CowArc<'static, str>>> {
        match self {
            BlockStateDefinition::Enum { values, .. } => Some(values.to_vec()),
            _ => None,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        let valid_values = self.get_values();

        let mut set = HashSet::<BlockState>::with_capacity(valid_values.len());
        if !valid_values.into_iter().all(|v| set.insert(v)) {
            return Err(format!("{:?} must have no duplicate values", self.identifier()));
        }

        if set.len() < 2 {
            return Err(format!("{:?} must have at least 2 values", self.identifier()));
        }

        Ok(())
    }
}

impl PartialEq<bool> for BlockState {
    fn eq(&self, other: &bool) -> bool {
        matches!(self, BlockState::Bool(val) if val == other)
    }
}

impl PartialEq<i32> for BlockState {
    fn eq(&self, other: &i32) -> bool {
        matches!(self, BlockState::Int(val) if val == other)
    }
}

impl PartialEq<&str> for BlockState {
    fn eq(&self, other: &&str) -> bool {
        matches!(self, BlockState::Enum(val) if val.as_ref() == *other)
    }
}

impl PartialEq<String> for BlockState {
    fn eq(&self, other: &String) -> bool {
        matches!(self, BlockState::Enum(val) if val.as_ref() == other)
    }
}

impl From<bool> for BlockState {
    fn from(val: bool) -> Self {
        BlockState::Bool(val)
    }
}

impl From<i32> for BlockState {
    fn from(val: i32) -> Self {
        BlockState::Int(val)
    }
}

impl From<&str> for BlockState {
    fn from(val: &str) -> Self {
        BlockState::Enum(CowArc::Owned(Arc::from(val)))
    }
}

impl From<String> for BlockState {
    fn from(val: String) -> Self {
        BlockState::Enum(CowArc::Owned(Arc::from(val)))
    }
}

impl From<Arc<str>> for BlockState {
    fn from(val: Arc<str>) -> Self {
        BlockState::Enum(CowArc::Owned(val))
    }
}
