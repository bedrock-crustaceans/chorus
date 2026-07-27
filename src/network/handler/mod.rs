use crate::network::BedrockProtocol;
use crate::network::handler::block::{broadcast_block_destroy, handle_block_interaction};
use crate::network::handler::chat::broadcast_chat;
use crate::network::handler::chunks::{handle_sub_chunk_request, send_pending_chunks};
use crate::network::handler::handshake::handle_handshake;
use crate::network::handler::item::{broadcast_spawned_items, broadcast_taken_items, handle_item_pickup, tick_item_entities};
use crate::network::handler::login::handle_login;
use crate::network::handler::play::{broadcast_block_updates, handle_play, on_enter_play};
use crate::network::handler::request::handle_request;
use crate::network::handler::resource::handle_resource;
use crate::network::handler::setup::{handle_setup, on_enter_setup};
use bevy_app::{App, FixedUpdate, Plugin};
use bevy_ecs::prelude::{Entity, Message};
use bevy_ecs::schedule::IntoScheduleConfigs;

pub mod block;
pub mod chat;
pub mod chunks;
pub mod form;
pub mod handshake;
pub mod item;
pub mod login;
pub mod play;
pub mod request;
pub mod resource;
pub mod setup;

#[derive(Message)]
pub struct PacketReceivedMessage {
    pub entity: Entity,
    pub packet: BedrockProtocol,
}

pub struct PacketHandlers;

impl Plugin for PacketHandlers {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                handle_request,
                handle_login,
                handle_handshake,
                handle_resource,
                (on_enter_setup, handle_setup).chain(),
                (on_enter_play, handle_play, broadcast_chat).chain(),
                send_pending_chunks,
                handle_sub_chunk_request,
                handle_block_interaction,
                broadcast_block_updates,
                broadcast_block_destroy,
                broadcast_spawned_items,
                tick_item_entities,
                (handle_item_pickup, broadcast_taken_items).chain(),
            ),
        );
    }
}
