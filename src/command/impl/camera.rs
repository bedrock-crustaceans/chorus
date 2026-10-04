use crate::command::command_definition::CommandDefinition;
use chorus_core::permission::PermissionLevel;

pub const CAMERA_COMMAND: CommandDefinition =
    CommandDefinition::new("camera", "Issues a camera instruction", |_, _| Err("/camera is not implemented yet".to_owned())).permission(PermissionLevel::Operator);
