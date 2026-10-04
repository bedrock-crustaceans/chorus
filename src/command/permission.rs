use bevy_ecs::prelude::Component;
use chorus_core::permission::PermissionLevel;

/// A player's command permission level, from `[permissions]` in the config.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandPermission(pub PermissionLevel);
