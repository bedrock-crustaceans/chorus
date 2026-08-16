use crate::command::context::CommandContext;
use crate::command::sender::CommandSender;
use crate::network::session::Session;
use crate::player::Player;
use crate::player::identity::PlayerIdentity;
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

/// Exclusive so that a command can read the whole world. The sender needs the session and the
/// player mutably, so both are taken out of the entity for the duration of the call and put back
/// right after.
pub fn dispatch_commands(world: &mut World) {
    let requests: Vec<CommandRequestedMessage> = world.resource_mut::<Messages<CommandRequestedMessage>>().drain().collect();

    for request in requests {
        let Ok(mut entity) = world.get_entity_mut(request.entity) else {
            continue;
        };

        let Some(name) = entity.get::<PlayerIdentity>().map(|identity| identity.name().to_string()) else {
            continue;
        };
        let Some(mut session) = entity.take::<Session>() else {
            continue;
        };
        let mut player = entity.take::<Player>();

        {
            let context = CommandContext::new(world);
            let mut sender = CommandSender::new(&mut session, name, player.as_mut());

            world.resource::<CommandRegistry>().dispatch(&context, &request.line, &mut sender);
        }

        let mut entity = world.entity_mut(request.entity);

        entity.insert(session);
        if let Some(player) = player {
            entity.insert(player);
        }
    }
}
