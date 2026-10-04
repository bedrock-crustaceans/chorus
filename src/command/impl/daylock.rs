use crate::command::command_definition::CommandDefinition;
use chorus_core::permission::PermissionLevel;

pub const DAYLOCK_COMMAND: CommandDefinition =
    CommandDefinition::new("daylock", "Locks and unlocks the day-night cycle", |_, _| Err("/daylock is not implemented yet".to_owned())).permission(PermissionLevel::Operator);
