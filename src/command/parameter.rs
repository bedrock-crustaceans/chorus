use atomicow::CowArc;
use bedrock::protocol::v898::packets::{OverloadsEntry, ParameterDataEntry};
use chorus_core::permission::PermissionLevel;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArgumentType {
    Int,
    Float,
    Value,
    WildcardInt,
    Operator,
    CompareOperator,
    Target,
    WildcardTarget,
    FilePath,
    IntegerRange,
    EquipmentSlot,
    String,
    BlockPosition,
    Position,
    Message,
    RawText,
    Json,
    BlockStates,
    Command,
}

impl ArgumentType {
    pub(crate) const fn id(self) -> u32 {
        match self {
            Self::Int => 1,
            Self::Float => 3,
            Self::Value => 4,
            Self::WildcardInt => 5,
            Self::Operator => 6,
            Self::CompareOperator => 7,
            Self::Target => 8,
            Self::WildcardTarget => 10,
            Self::FilePath => 17,
            Self::IntegerRange => 23,
            Self::EquipmentSlot => 47,
            Self::String => 56,
            Self::BlockPosition => 64,
            Self::Position => 65,
            Self::Message => 67,
            Self::RawText => 70,
            Self::Json => 74,
            Self::BlockStates => 84,
            Self::Command => 87,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Int | Self::WildcardInt => "int",
            Self::Float => "float",
            Self::Value => "value",
            Self::Operator => "operator",
            Self::CompareOperator => "compare operator",
            Self::Target | Self::WildcardTarget => "target",
            Self::FilePath => "filepath",
            Self::IntegerRange => "range",
            Self::EquipmentSlot => "slot",
            Self::String => "string",
            Self::BlockPosition | Self::Position => "x y z",
            Self::Message => "message",
            Self::RawText => "text",
            Self::Json => "json",
            Self::BlockStates => "block states",
            Self::Command => "command",
        }
    }

    pub const fn is_greedy(self) -> bool {
        matches!(self, Self::Message | Self::RawText | Self::Json | Self::Command)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandEnum {
    pub name: CowArc<'static, str>,
    pub values: CowArc<'static, [CowArc<'static, str>]>,
    pub constraints: CowArc<'static, [ConstrainedValue]>,
}

impl CommandEnum {
    pub const fn new(name: &'static str, values: &'static [CowArc<'static, str>]) -> Self {
        Self {
            name: CowArc::Static(name),
            values: CowArc::Static(values),
            constraints: CowArc::Static(&[]),
        }
    }

    pub fn owned(name: impl Into<String>, values: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            name: CowArc::Owned(name.into().into()),
            values: CowArc::Owned(values.into_iter().map(|value| CowArc::Owned(value.into().into())).collect()),
            constraints: CowArc::Static(&[]),
        }
    }

    pub const fn constrained(mut self, constraints: &'static [ConstrainedValue]) -> Self {
        std::mem::forget(std::mem::replace(&mut self.constraints, CowArc::Static(constraints)));
        self
    }

    pub fn find(&self, input: &str) -> Option<&str> {
        self.values.iter().map(|value| value.as_ref()).find(|value| value.eq_ignore_ascii_case(input))
    }

    pub fn constraints_of(&self, value: &str) -> EnumConstraints {
        self.constraints
            .iter()
            .filter(|constrained| constrained.value.eq_ignore_ascii_case(value))
            .fold(EnumConstraints::NONE, |all, constrained| all.with(constrained.constraints))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct EnumConstraints(u8);

impl EnumConstraints {
    pub const NONE: Self = Self(0);
    pub const CHEATS_ENABLED: Self = Self(1);
    pub const OPERATOR_PERMISSIONS: Self = Self(2);
    pub const HOST_PERMISSIONS: Self = Self(4);

    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub(crate) fn ids(self) -> Vec<i8> {
        (0..3).filter(|bit| self.0 & (1 << bit) != 0).map(|bit| bit as i8).collect()
    }

    pub fn required_level(self) -> PermissionLevel {
        if self.0 & Self::HOST_PERMISSIONS.0 != 0 {
            PermissionLevel::Host
        } else if self.0 & Self::OPERATOR_PERMISSIONS.0 != 0 {
            PermissionLevel::Operator
        } else {
            PermissionLevel::Member
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConstrainedValue {
    pub value: CowArc<'static, str>,
    pub constraints: EnumConstraints,
}

impl ConstrainedValue {
    pub const fn new(value: &'static str, constraints: EnumConstraints) -> Self {
        Self {
            value: CowArc::Static(value),
            constraints,
        }
    }
}

#[macro_export]
macro_rules! constraints {
    ($(($value:expr, $constraints:expr)),* $(,)?) => {{
        const CONSTRAINTS: &[$crate::command::parameter::ConstrainedValue] = &[$($crate::command::parameter::ConstrainedValue::new($value, $constraints)),*];
        CONSTRAINTS
    }};
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChainedSubcommand {
    pub name: CowArc<'static, str>,
    pub values: CowArc<'static, [(CowArc<'static, str>, CowArc<'static, str>)]>,
}

impl ChainedSubcommand {
    pub const fn new(name: &'static str, values: &'static [(CowArc<'static, str>, CowArc<'static, str>)]) -> Self {
        Self {
            name: CowArc::Static(name),
            values: CowArc::Static(values),
        }
    }
}

#[macro_export]
macro_rules! chained {
    ($(($first:expr, $second:expr)),* $(,)?) => {{
        const VALUES: &[(atomicow::CowArc<'static, str>, atomicow::CowArc<'static, str>)] = &[$((atomicow::CowArc::Static($first), atomicow::CowArc::Static($second))),*];
        VALUES
    }};
}

#[macro_export]
macro_rules! values {
    ($($value:expr),* $(,)?) => {{
        const VALUES: &[atomicow::CowArc<'static, str>] = &[$(atomicow::CowArc::Static($value)),*];
        VALUES
    }};
}

#[macro_export]
macro_rules! overloads {
    ($([$($parameter:expr),* $(,)?]),* $(,)?) => {{
        const OVERLOADS: &[$crate::command::parameter::CommandOverload] = &[$($crate::command::parameter::CommandOverload::new({
            const PARAMETERS: &[$crate::command::parameter::CommandParameter] = &[$($parameter),*];
            PARAMETERS
        })),*];
        OVERLOADS
    }};
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParameterKind {
    Argument(ArgumentType),
    Enum(CommandEnum),
    Literal(CowArc<'static, str>),
    SoftEnum(CowArc<'static, str>),
    Postfix(CowArc<'static, str>),
    CommandName,
}

pub const COMMAND_NAME_ENUM: &str = "CommandName";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ParameterOptions(pub(crate) u8);

impl ParameterOptions {
    pub const NONE: Self = Self(0);
    pub const COLLAPSE_ENUM: Self = Self(1);
    pub const HAS_SEMANTIC_CONSTRAINT: Self = Self(2);
    pub const AS_CHAINED_COMMAND: Self = Self(4);

    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandParameter {
    pub name: CowArc<'static, str>,
    pub kind: ParameterKind,
    pub optional: bool,
    pub options: ParameterOptions,
}

impl CommandParameter {
    pub const fn new(name: &'static str, kind: ArgumentType) -> Self {
        Self::of(name, ParameterKind::Argument(kind))
    }

    pub const fn enumeration(name: &'static str, values: CommandEnum) -> Self {
        Self::of(name, ParameterKind::Enum(values))
    }

    pub const fn literal(word: &'static str) -> Self {
        Self::of(word, ParameterKind::Literal(CowArc::Static(word)))
    }

    pub const fn soft_enum(name: &'static str, soft_enum: &'static str) -> Self {
        Self::of(name, ParameterKind::SoftEnum(CowArc::Static(soft_enum)))
    }

    pub const fn postfix(name: &'static str, postfix: &'static str) -> Self {
        Self::of(name, ParameterKind::Postfix(CowArc::Static(postfix)))
    }

    pub const fn command_name(name: &'static str) -> Self {
        Self::of(name, ParameterKind::CommandName)
    }

    const fn of(name: &'static str, kind: ParameterKind) -> Self {
        Self {
            name: CowArc::Static(name),
            kind,
            optional: false,
            options: ParameterOptions::NONE,
        }
    }

    pub const fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    pub const fn options(mut self, options: ParameterOptions) -> Self {
        self.options = options;
        self
    }

    pub fn usage_token(&self) -> String {
        let inner = match &self.kind {
            ParameterKind::Argument(kind) => format!("{}: {}", self.name, kind.label()),
            ParameterKind::Enum(values) => format!("{}: {}", self.name, values.values.join("|")),
            ParameterKind::Literal(word) => return if self.optional { format!("[{word}]") } else { word.to_string() },
            ParameterKind::SoftEnum(name) => format!("{}: {name}", self.name),
            ParameterKind::Postfix(postfix) => format!("{}: int{postfix}", self.name),
            ParameterKind::CommandName => format!("{}: {COMMAND_NAME_ENUM}", self.name),
        };
        if self.optional { format!("[{inner}]") } else { format!("<{inner}>") }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandOverload {
    pub parameters: CowArc<'static, [CommandParameter]>,
    pub chaining: bool,
}

impl CommandOverload {
    pub const fn new(parameters: &'static [CommandParameter]) -> Self {
        Self {
            parameters: CowArc::Static(parameters),
            chaining: false,
        }
    }

    pub fn owned(parameters: Vec<CommandParameter>) -> Self {
        Self {
            parameters: CowArc::Owned(parameters.into()),
            chaining: false,
        }
    }

    pub const fn chaining(mut self) -> Self {
        self.chaining = true;
        self
    }

    pub fn usage(&self) -> String {
        self.parameters.iter().map(CommandParameter::usage_token).collect::<Vec<_>>().join(" ")
    }

    pub(crate) fn to_entry(&self, mut symbol: impl FnMut(&CommandParameter) -> u32) -> OverloadsEntry {
        OverloadsEntry {
            is_chaining: self.chaining,
            parameter_data: self
                .parameters
                .iter()
                .map(|parameter| ParameterDataEntry {
                    name: parameter.name.to_string(),
                    parse_symbol: symbol(parameter),
                    is_optional: parameter.optional,
                    options: parameter.options.0 as i8,
                })
                .collect(),
        }
    }
}
