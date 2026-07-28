use crate::command::command_result::CommandResult;
use crate::command::parameter::CommandOverload;
use crate::command::sender::CommandSender;
use crate::registry::command_registry::CommandRegistry;
use atomicow::CowArc;
use bedrock::protocol::v898::packets::CommandPermissionLevelString;

#[derive(Debug)]
pub struct CommandDefinition {
    pub name: CowArc<'static, str>,
    pub description: CowArc<'static, str>,
    pub aliases: CowArc<'static, [CowArc<'static, str>]>,
    pub permission: CommandPermissionLevelString,
    pub overloads: CowArc<'static, [CommandOverload]>,

    pub execute: fn(&CommandRegistry, &mut CommandSender, &[&str]) -> CommandResult,
}

impl CommandDefinition {
    pub fn usage(&self) -> String {
        let overloads = &self.overloads;
        if overloads.is_empty() {
            return format!("/{}", self.name);
        }

        overloads.iter().map(|overload| format!("/{} {}", self.name, overload.usage())).collect::<Vec<_>>().join("\n")
    }
}

#[macro_export]
macro_rules! const_command {
    (
        name: $name:expr,
        description: $description:expr,
        aliases: [$($alias:expr),* $(,)?],
        permission: $permission:expr,
        overloads: [$($overload:expr),* $(,)?],
        execute: $execute:expr$(,)?
    ) => {{
        $crate::command::command_definition::CommandDefinition {
            name: atomicow::CowArc::Static($name),
            description: atomicow::CowArc::Static($description),
            aliases: atomicow::CowArc::Static(&[
                $(atomicow::CowArc::Static(&$alias)),*
            ]),
            permission: $permission,
            overloads: atomicow::CowArc::Static(&[$($overload),*]),
            execute: $execute,
        }
    }};
}
