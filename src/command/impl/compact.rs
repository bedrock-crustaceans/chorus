use crate::command::command_definition::CommandDefinition;
use crate::const_command;
use crate::level::Level;
use bedrock::protocol::v898::packets::CommandPermissionLevelString;

pub const COMPACT_COMMAND: CommandDefinition = const_command! {
    name: "compact",
    description: "Compacts the level database in the background",
    aliases: [],
    permission: CommandPermissionLevelString::Admin,
    overloads: [],
    execute: |context, _| {
        let Some(storage) = context.world().get_resource::<Level>().and_then(|level| level.storage().cloned()) else {
            return Err("this level is not saved to disk".to_owned());
        };
        if !storage.schedule_compaction() {
            return Err("the level database is already being compacted".to_owned());
        }
        context.reply("Compacting the level database in the background");
        Ok(())
    },
};
