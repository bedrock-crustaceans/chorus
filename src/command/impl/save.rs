use crate::command::command_definition::CommandDefinition;
use crate::level::Level;
use chorus_core::permission::PermissionLevel;

pub const SAVE_COMMAND: CommandDefinition = CommandDefinition::new("save", "Saves the level to disk", |context, _| {
    crate::actor::storage::save_all_actors(context.world_mut());
    let mut level = context.world_mut().get_resource_mut::<Level>().ok_or("the level is not loaded yet")?;
    if level.storage().is_none() {
        return Err("this level is not saved to disk".to_owned());
    }
    let saved = level.save();
    context.reply(format!("Saved {saved} chunks"));
    Ok(())
})
.permission(PermissionLevel::Admin);
