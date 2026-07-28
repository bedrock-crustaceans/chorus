use crate::command::command_definition::CommandDefinition;
use crate::const_command;
use bedrock::protocol::v898::packets::CommandPermissionLevelString;

pub const DAYLOCK_COMMAND: CommandDefinition = const_command! {
    name: "daylock",
    description: "Locks and unlocks the day-night cycle.",
    aliases: ["alwaysday"],
    permission: CommandPermissionLevelString::GameDirectors,
    overloads: [
        // TODO
    ],
    execute: |_, _, _| { Ok(()) },
};
