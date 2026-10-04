use crate::command::command_definition::CommandDefinition;
use chorus_core::permission::PermissionLevel;

pub const AIMASSIST_COMMAND: CommandDefinition =
    CommandDefinition::new("aimassist", "Enable Aim Assist", |_, _| Err("/aimassist is not implemented yet".to_owned())).permission(PermissionLevel::Operator);
