use bedrock::protocol::v898::packets::{OverloadsEntry, ParameterDataEntry};

const ARG_FLAG_VALID: u32 = 0x100000;

pub enum CommandParameterType {
    Int,
    Float,
    Value,
    Target,
    String,
    Position,
    Message,
    RawText,
    Json,
    Command,
}

impl CommandParameterType {
    fn id(&self) -> u32 {
        match self {
            Self::Int => 1,
            Self::Float => 3,
            Self::Value => 4,
            Self::Target => 8,
            Self::String => 56,
            Self::Position => 65,
            Self::Message => 67,
            Self::RawText => 70,
            Self::Json => 74,
            Self::Command => 87,
        }
    }

    pub fn parse_symbol(&self) -> u32 {
        ARG_FLAG_VALID | self.id()
    }

    pub fn label(&self) -> &str {
        match self {
            Self::Int => "int",
            Self::Float => "float",
            Self::Value => "value",
            Self::Target => "target",
            Self::String => "string",
            Self::Position => "x y z",
            Self::Message => "message",
            Self::RawText => "text",
            Self::Json => "json",
            Self::Command => "command",
        }
    }
}

pub struct CommandParameter {
    name: String,
    kind: CommandParameterType,
    optional: bool,
}

impl CommandParameter {
    pub fn new(name: impl Into<String>, kind: CommandParameterType, optional: bool) -> Self {
        Self { name: name.into(), kind, optional }
    }

    fn to_entry(&self) -> ParameterDataEntry {
        ParameterDataEntry {
            name: self.name.clone(),
            parse_symbol: self.kind.parse_symbol(),
            is_optional: self.optional,
            options: 0,
        }
    }

    pub fn usage_token(&self) -> String {
        let inner = format!("{}: {}", self.name, self.kind.label());
        if self.optional { format!("[{inner}]") } else { format!("<{inner}>") }
    }
}

pub struct CommandOverload {
    parameters: Vec<CommandParameter>,
}

impl CommandOverload {
    pub fn new(parameters: Vec<CommandParameter>) -> Self {
        Self { parameters }
    }

    pub fn to_entry(&self) -> OverloadsEntry {
        OverloadsEntry {
            is_chaining: false,
            parameter_data: self.parameters.iter().map(CommandParameter::to_entry).collect(),
        }
    }

    pub fn usage(&self) -> String {
        self.parameters.iter().map(CommandParameter::usage_token).collect::<Vec<_>>().join(" ")
    }
}

impl Default for CommandOverload {
    fn default() -> Self {
        Self::new(vec![CommandParameter::new("args", CommandParameterType::RawText, true)])
    }
}
