use bedrock::protocol::v662::enums::CommandPermissionLevel;
use bedrock::protocol::v898::packets::CommandPermissionLevelString;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub enum PermissionLevel {
    #[default]
    #[serde(alias = "any")]
    Member,
    #[serde(alias = "game_directors", alias = "gamedirectors", alias = "op")]
    Operator,
    Admin,
    Host,
    Owner,
    Internal,
}

impl PermissionLevel {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Member => "member",
            Self::Operator => "operator",
            Self::Admin => "admin",
            Self::Host => "host",
            Self::Owner => "owner",
            Self::Internal => "internal",
        }
    }
}

impl From<PermissionLevel> for CommandPermissionLevelString {
    fn from(level: PermissionLevel) -> Self {
        match level {
            PermissionLevel::Member => Self::Any,
            PermissionLevel::Operator => Self::GameDirectors,
            PermissionLevel::Admin => Self::Admin,
            PermissionLevel::Host => Self::Host,
            PermissionLevel::Owner => Self::Owner,
            PermissionLevel::Internal => Self::Internal,
        }
    }
}

impl From<PermissionLevel> for CommandPermissionLevel {
    fn from(level: PermissionLevel) -> Self {
        match level {
            PermissionLevel::Member => Self::Any,
            PermissionLevel::Operator => Self::GameDirectors,
            PermissionLevel::Admin => Self::Admin,
            PermissionLevel::Host => Self::Host,
            PermissionLevel::Owner => Self::Owner,
            PermissionLevel::Internal => Self::Internal,
        }
    }
}
