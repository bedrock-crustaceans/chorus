use crate::console::{self, Console};
use crate::network::BedrockProtocol;
use crate::network::session::Session;
use crate::player::identity::PlayerIdentity;
use crate::registry::command_registry::CommandRegistry;
use bedrock::protocol::ProtoVersionPackets;
use bedrock::protocol::v924::enums::TextPacketType;
use bevy_ecs::component::Mutable;
use bevy_ecs::prelude::{Component, Entity, Mut, Resource, World};
use bevy_ecs::query::{ReleaseStateQueryData, SingleEntityQueryData};

type TextPacket = <BedrockProtocol as ProtoVersionPackets>::TextPacket;

pub struct CommandContext<'w> {
    world: &'w mut World,
    sender: Entity,
}

impl<'w> CommandContext<'w> {
    pub fn new(world: &'w mut World, sender: Entity) -> Self {
        Self { world, sender }
    }

    pub fn world(&self) -> &World {
        self.world
    }

    pub fn world_mut(&mut self) -> &mut World {
        self.world
    }

    pub fn resource<R: Resource>(&self) -> &R {
        self.world.resource::<R>()
    }

    pub fn registry(&self) -> &CommandRegistry {
        self.resource::<CommandRegistry>()
    }

    pub fn sender(&self) -> Entity {
        self.sender
    }

    pub fn sender_name(&self) -> &str {
        if self.is_console() {
            return "Server";
        }
        self.get::<PlayerIdentity>().map_or("", |identity| identity.name())
    }

    pub fn get<C: Component>(&self) -> Option<&C> {
        self.world.get::<C>(self.sender)
    }

    pub fn get_mut<C: Component<Mutability = Mutable>>(&mut self) -> Option<Mut<'_, C>> {
        self.world.get_mut::<C>(self.sender)
    }

    pub fn components_mut<Q: ReleaseStateQueryData + SingleEntityQueryData>(&mut self) -> Option<Q::Item<'_, 'static>> {
        self.world.get_entity_mut(self.sender).ok()?.into_components_mut::<Q>().ok()
    }

    pub fn is_console(&self) -> bool {
        self.get::<Console>().is_some()
    }

    pub fn reply(&mut self, message: impl Into<String>) {
        let Some(mut session) = self.get_mut::<Session>() else {
            if self.is_console() {
                console::reply(&message.into());
            }
            return;
        };

        session.send(BedrockProtocol::TextPacket(
            TextPacket {
                localize: false,
                message_type: TextPacketType::SystemMessage(message.into()),
                sender_xuid: String::new(),
                platform_id: String::new(),
                filtered_message: None,
            }
            .into(),
        ));
    }
}
