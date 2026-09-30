use crate::command::command_definition::CommandDefinition;
use crate::const_command;
use bedrock::protocol::v898::packets::CommandPermissionLevelString;

pub const PING_COMMAND: CommandDefinition = const_command! {
    name: "ping",
    description: "Replies with pong",
    aliases: [],
    permission: CommandPermissionLevelString::Any,
    overloads: [],
    execute: |context, _| {
        let name = context.sender_name().to_string();
        context.reply(format!("Pong, {name}!"));
        Ok(())
    },
};
