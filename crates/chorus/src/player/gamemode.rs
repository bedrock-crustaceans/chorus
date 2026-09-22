use bedrock::protocol::v662::enums::GameType;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Gamemode {
    #[default]
    Survival,
    Creative,
    Adventure,
    Spectator,
}

impl Gamemode {
    pub fn from_alias(alias: &str) -> Option<Self> {
        match alias.to_lowercase().as_str() {
            "survival" | "s" | "0" => Some(Self::Survival),
            "creative" | "c" | "1" => Some(Self::Creative),
            "adventure" | "a" | "2" => Some(Self::Adventure),
            "spectator" | "v" | "view" | "3" => Some(Self::Spectator),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Survival => "Survival",
            Self::Creative => "Creative",
            Self::Adventure => "Adventure",
            Self::Spectator => "Spectator",
        }
    }

    pub fn game_type(&self) -> GameType {
        match self {
            Self::Survival => GameType::Survival,
            Self::Creative => GameType::Creative,
            Self::Adventure => GameType::Adventure,
            Self::Spectator => GameType::Spectator,
        }
    }
}
