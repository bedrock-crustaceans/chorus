use bevy_ecs::prelude::Component;
use chorus_core::permission::PermissionLevel;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandPermission(pub PermissionLevel);
