use crate::command::context::CommandContext;
use crate::network::session::Session;
use crate::registry::command_registry::CommandRegistry;
use bevy_ecs::message::Messages;
use bevy_ecs::prelude::{Entity, Message, World};

#[derive(Message)]
pub struct CommandRequestedMessage {
    pub entity: Entity,
    pub line: String,
}

#[derive(Message, Clone, Debug)]
pub struct CommandPreprocessMessage {
    pub entity: Entity,
    pub line: String,
}

pub fn dispatch_commands(world: &mut World) {
    let requests: Vec<CommandRequestedMessage> = world.resource_mut::<Messages<CommandRequestedMessage>>().drain().collect();

    for request in requests {
        if world.get::<Session>(request.entity).is_none() {
            continue;
        }

        CommandRegistry::dispatch(&mut CommandContext::new(world, request.entity), &request.line);
    }
}
