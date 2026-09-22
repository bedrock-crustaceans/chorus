use bevy_ecs::prelude::{Entity, Message};

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum SessionState {
    Request,
    Login,
    Handshake,
    Resource,
    Setup,
    Play,
}

#[derive(Message, Clone, Debug)]
pub struct SessionStateChangedMessage {
    pub entity: Entity,
    pub from: SessionState,
    pub to: SessionState,
}
