use crate::command::args::CommandArgs;
use crate::command::command_result::CommandResult;
use crate::command::context::CommandContext;
use crate::command::parameter::{ChainedSubcommand, CommandOverload};
use atomicow::CowArc;
use chorus_core::permission::PermissionLevel;

pub type CommandExecutor = fn(&mut CommandContext, &CommandArgs) -> CommandResult;

/// Hints for the client about where a command may be used and how it is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CommandFlags(pub(crate) u16);

impl CommandFlags {
    pub const NONE: Self = Self(0);
    pub const TEST_USAGE: Self = Self(0x1);
    pub const HIDDEN_FROM_COMMAND_BLOCKS: Self = Self(0x2);
    pub const HIDDEN_FROM_PLAYERS: Self = Self(0x4);
    pub const HIDDEN_FROM_AUTOMATION: Self = Self(0x8);
    pub const LOCAL_SYNC: Self = Self(0x10);
    pub const EXECUTE_DISALLOWED: Self = Self(0x20);
    pub const MESSAGE_TYPE: Self = Self(0x40);
    /// Usable without cheats enabled.
    pub const NOT_CHEAT: Self = Self(0x80);
    pub const ASYNC: Self = Self(0x100);

    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

/// A command, built in a const:
///
/// ```ignore
/// pub const PING_COMMAND: CommandDefinition = CommandDefinition::new("ping", "Replies with pong", |context, _| { ... })
///     .aliases(values!["p"])
///     .permission(PermissionLevel::Member);
/// ```
#[derive(Debug, Clone)]
pub struct CommandDefinition {
    pub name: CowArc<'static, str>,
    pub description: CowArc<'static, str>,
    pub aliases: CowArc<'static, [CowArc<'static, str>]>,
    pub permission: PermissionLevel,
    pub flags: CommandFlags,
    pub overloads: CowArc<'static, [CommandOverload]>,
    /// Chained subcommands the command's chaining overloads lead into.
    pub chained_subcommands: CowArc<'static, [ChainedSubcommand]>,
    pub execute: CommandExecutor,
}

impl CommandDefinition {
    pub const fn new(name: &'static str, description: &'static str, execute: CommandExecutor) -> Self {
        Self {
            name: CowArc::Static(name),
            description: CowArc::Static(description),
            aliases: CowArc::Static(&[]),
            permission: PermissionLevel::Member,
            flags: CommandFlags::NOT_CHEAT,
            overloads: CowArc::Static(&[]),
            chained_subcommands: CowArc::Static(&[]),
            execute,
        }
    }

    pub const fn chained_subcommands(mut self, chained: &'static [ChainedSubcommand]) -> Self {
        std::mem::forget(std::mem::replace(&mut self.chained_subcommands, CowArc::Static(chained)));
        self
    }

    pub const fn aliases(mut self, aliases: &'static [CowArc<'static, str>]) -> Self {
        std::mem::forget(std::mem::replace(&mut self.aliases, CowArc::Static(aliases)));
        self
    }

    pub const fn permission(mut self, permission: PermissionLevel) -> Self {
        self.permission = permission;
        self
    }

    pub const fn flags(mut self, flags: CommandFlags) -> Self {
        self.flags = flags;
        self
    }

    pub const fn overloads(mut self, overloads: &'static [CommandOverload]) -> Self {
        std::mem::forget(std::mem::replace(&mut self.overloads, CowArc::Static(overloads)));
        self
    }

    pub fn usage(&self) -> String {
        if self.overloads.is_empty() {
            return format!("/{}", self.name);
        }
        self.overloads
            .iter()
            .map(|overload| format!("/{} {}", self.name, overload.usage()).trim_end().to_owned())
            .collect::<Vec<_>>()
            .join("\n")
    }
}
