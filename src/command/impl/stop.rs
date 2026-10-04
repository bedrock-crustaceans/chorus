use crate::command::command_definition::CommandDefinition;
use crate::const_command;
use bedrock::protocol::v898::packets::CommandPermissionLevelString;
use bevy_app::AppExit;

pub const STOP_COMMAND: CommandDefinition = const_command! {
    name: "stop",
    description: "Saves the level and stops the server",
    aliases: [],
    permission: CommandPermissionLevelString::Admin,
    overloads: [],
    execute: |context, _| {
        context.reply("Stopping the server");
        context.world_mut().write_message(AppExit::Success);
        Ok(())
    },
};
