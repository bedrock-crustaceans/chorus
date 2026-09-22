use crate::command::command_definition::CommandDefinition;
use crate::const_command;
use bedrock::protocol::v898::packets::CommandPermissionLevelString;

pub const CAMERA_COMMAND: CommandDefinition = const_command! {
    name: "camera",
    description: "Issues a camera instruction",
    aliases: [],
    permission: CommandPermissionLevelString::GameDirectors,
    overloads: [
        // TODO
    ],
    execute: |_, _, _| { Ok(()) },
};
