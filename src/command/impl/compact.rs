use crate::command::command_definition::CommandDefinition;
use crate::level::Level;
use chorus_core::permission::PermissionLevel;

pub const COMPACT_COMMAND: CommandDefinition = CommandDefinition::new("compact", "Compacts the level database in the background", |context, _| {
    let storage = context.world().get_resource::<Level>().and_then(|level| level.storage().cloned()).ok_or("this level is not saved to disk")?;
    if !storage.schedule_compaction() {
        return Err("the level database is already being compacted".to_owned());
    }
    context.reply("Compacting the level database in the background");
    Ok(())
})
.permission(PermissionLevel::Admin);
