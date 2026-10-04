use crate::command::command_definition::CommandDefinition;
use crate::const_command;
use crate::level::Level;
use bedrock::protocol::v898::packets::CommandPermissionLevelString;

pub const SAVE_COMMAND: CommandDefinition = const_command! {
    name: "save",
    description: "Saves the level to disk",
    aliases: [],
    permission: CommandPermissionLevelString::Admin,
    overloads: [],
    execute: |context, _| {
        let Some(mut level) = context.world_mut().get_resource_mut::<Level>() else {
            return Err("the level is not loaded yet".to_owned());
        };
        if level.storage.is_none() {
            return Err("this level is not saved to disk".to_owned());
        }
        let saved = level.save();
        context.reply(format!("Saved {saved} chunks"));
        Ok(())
    },
};
