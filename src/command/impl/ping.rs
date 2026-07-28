use crate::command::command_definition::CommandDefinition;
use crate::command::command_registry::CommandRegistry;
use crate::command::command_result::CommandResult;
use crate::command::sender::CommandSender;
use crate::const_command;
use bedrock::protocol::v898::packets::CommandPermissionLevelString;

pub const PING_COMMAND: CommandDefinition = const_command! {
    name: "ping",
    description: "Replies with pong",
    aliases: [],
    permission: CommandPermissionLevelString::Any,
    overloads: [],
    execute: execute,
};

fn execute(_registry: &CommandRegistry, sender: &mut CommandSender, _args: &[&str]) -> CommandResult {
    let name = sender.name().to_string();
    sender.reply(format!("Pong, {name}!"));
    Ok(())
}
