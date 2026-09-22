use crate::command::command_result::CommandResult;
use crate::command::context::CommandContext;
use crate::command::parameter::CommandOverload;
use crate::command::sender::CommandSender;
use atomicow::CowArc;
use bedrock::protocol::v898::packets::CommandPermissionLevelString;

#[derive(Debug)]
pub struct CommandDefinition {
    pub name: CowArc<'static, str>,
    pub description: CowArc<'static, str>,
    pub aliases: CowArc<'static, [CowArc<'static, str>]>,
    pub permission: CommandPermissionLevelString,
    pub overloads: CowArc<'static, [CommandOverload]>,

    pub execute: fn(&CommandContext, &mut CommandSender, &[&str]) -> CommandResult,
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
