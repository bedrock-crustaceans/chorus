use crate::command::dispatch::dispatch_commands;
use crate::network::BedrockProtocol;
use crate::network::handler::block::{broadcast_level_events, broadcast_level_sounds, handle_block_actions, update_block_breaking};
use crate::network::handler::chat::{broadcast_chat, broadcast_message};
use crate::network::handler::chunks::{handle_sub_chunk_request, send_pending_chunks, update_chunk_order};
use crate::network::handler::handshake::handle_handshake;
use crate::network::handler::inventory::{handle_inventory_packets, send_initial_inventory};
use crate::network::handler::login::handle_login;
use crate::network::handler::play::{broadcast_block_updates, handle_play, on_enter_play, on_quit};
use crate::network::handler::request::handle_request;
use crate::network::handler::resource::handle_resource;
use crate::network::handler::setup::{handle_setup, on_enter_setup};
use bevy_app::{App, Plugin, Update};
use bevy_ecs::prelude::{Entity, Message};
use bevy_ecs::schedule::IntoScheduleConfigs;

pub mod block;
pub mod chat;
pub mod chunks;
pub mod form;
pub mod handshake;
pub mod inventory;
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
            Update,
            // chained so that a state change reaches its entry logic within the same tick instead
            // of waiting for the next one - these all touch Session, so they never ran in parallel
            (
                (handle_request, handle_login, handle_handshake, handle_resource).chain(),
                (on_enter_setup, handle_setup).chain(),
                (on_enter_play, send_initial_inventory, handle_play).chain(),
                (handle_block_actions, update_block_breaking, handle_inventory_packets, dispatch_commands).chain(),
                (broadcast_chat, on_quit, broadcast_message).chain(),
                (update_chunk_order, send_pending_chunks, handle_sub_chunk_request).chain(),
                (broadcast_block_updates, broadcast_level_events, broadcast_level_sounds).chain(),
            )
                .chain(),
        );
    }
}
