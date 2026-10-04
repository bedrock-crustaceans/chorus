use crate::command::command_definition::CommandDefinition;
use bevy_app::AppExit;
use chorus_core::permission::PermissionLevel;

pub const STOP_COMMAND: CommandDefinition = CommandDefinition::new("stop", "Saves the level and stops the server", |context, _| {
    context.reply("Stopping the server");
    context.world_mut().write_message(AppExit::Success);
    Ok(())
})
.permission(PermissionLevel::Admin);
