use crate::command::permission::CommandPermission;
use crate::console::{self, Console};
use crate::network::BedrockProtocol;
use crate::network::session::Session;
use crate::player::identity::PlayerIdentity;
use crate::registry::command_registry::CommandRegistry;
use atomicow::CowArc;
use bedrock::protocol::ProtoVersionPackets;
use bedrock::protocol::v924::enums::TextPacketType;
use bevy_ecs::component::Mutable;
use bevy_ecs::prelude::{Component, Entity, Mut, Resource, World};
use bevy_ecs::query::{ReleaseStateQueryData, SingleEntityQueryData};
use chorus_core::permission::PermissionLevel;

type TextPacket = <BedrockProtocol as ProtoVersionPackets>::TextPacket;

pub struct CommandContext<'w> {
    world: &'w mut World,
    sender: Entity,
    command: CowArc<'static, str>,
}

impl<'w> CommandContext<'w> {
    pub fn new(world: &'w mut World, sender: Entity) -> Self {
        Self {
            world,
            sender,
            command: CowArc::Static(""),
        }
    }

    /// The name the sender actually typed, which is the alias when they used one.
    ///
    /// Built-in commands know their own name at compile time and never need this. Plugin commands
    /// do: they all share one executor, so the name is the only thing that says which plugin to
    /// call.
    pub fn command_name(&self) -> &str {
        &self.command
    }

    pub(crate) fn set_command_name(&mut self, name: &str) {
        self.command = CowArc::Owned(std::sync::Arc::from(name));
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

    pub fn permission_level(&self) -> PermissionLevel {
        if self.is_console() {
            return PermissionLevel::Owner;
        }
        self.get::<CommandPermission>().map_or(PermissionLevel::Member, |permission| permission.0)
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
