use crate::command::command_definition::CommandDefinition;
use crate::const_command;
use bedrock::protocol::v898::packets::CommandPermissionLevelString;

pub const AIMASSIST_COMMAND: CommandDefinition = const_command! {
    name: "aimassist",
    description: "Enable Aim Assist",
    aliases: [],
    permission: CommandPermissionLevelString::GameDirectors,
    overloads: [
        // TODO
    ],
    execute: |_, _, _| { Ok(()) },
};
