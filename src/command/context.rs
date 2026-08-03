use crate::registry::command_registry::CommandRegistry;
use bevy_ecs::prelude::{Resource, World};

/// Read-only view of the server handed to a command while it runs.
pub struct CommandContext<'w> {
    world: &'w World,
}

impl<'w> CommandContext<'w> {
    pub fn new(world: &'w World) -> Self {
        Self { world }
    }

    pub fn world(&self) -> &World {
        self.world
    }

    pub fn registry(&self) -> &CommandRegistry {
        self.resource::<CommandRegistry>()
    }

    pub fn resource<R: Resource>(&self) -> &R {
        self.world.resource::<R>()
    }
}
