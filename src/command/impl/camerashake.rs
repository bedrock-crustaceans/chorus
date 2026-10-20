use crate::command::command_definition::CommandDefinition;
use chorus_core::permission::PermissionLevel;

pub static CAMERASHAKE_COMMAND: CommandDefinition =
    CommandDefinition::new("camerashake", "Applies shaking to the players' camera", |_, _| Err("/camerashake is not implemented yet".to_owned())).permission(PermissionLevel::Operator);
