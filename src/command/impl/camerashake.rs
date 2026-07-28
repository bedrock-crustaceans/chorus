use crate::command::command_definition::CommandDefinition;
use crate::const_command;
use bedrock::protocol::v898::packets::CommandPermissionLevelString;

pub const CAMERASHAKE_COMMAND: CommandDefinition = const_command! {
    name: "camerashake",
    description: "Applies shaking to the players' camera with a specified intensity and duration.",
    aliases: [],
    permission: CommandPermissionLevelString::GameDirectors,
    overloads: [
        // TODO
    ],
    execute: |_, _, _| { Ok(()) },
};
