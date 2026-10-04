use crate::Tick;
use crate::command::dispatch::dispatch_commands;
use crate::network::BedrockProtocol;
use crate::network::handler::block::decode_block_actions;
use crate::network::handler::handshake::handle_handshake;
use crate::network::handler::inventory::{handle_inventory_packets, send_initial_inventory};
use crate::network::handler::login::handle_login;
use crate::network::handler::play::{handle_play, on_enter_play, on_quit};
use crate::network::handler::request::handle_request;
use crate::network::handler::resource::handle_resource;
use crate::network::handler::setup::{handle_setup, on_enter_setup};
use crate::registry::command_registry::CommandRegistry;
use crate::schedule::GameSet;
use bevy_app::{App, Plugin};
use bevy_ecs::prelude::{Entity, Message};
use bevy_ecs::schedule::IntoScheduleConfigs;

pub mod block;
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
            Tick,
            (
                (
                    (handle_request, handle_login, handle_handshake, handle_resource).chain(),
                    (on_enter_setup, handle_setup).chain(),
                    (on_enter_play, send_initial_inventory, handle_play, on_quit).chain(),
                )
                    .chain()
                    .in_set(GameSet::Connection),
                (decode_block_actions, handle_inventory_packets, dispatch_commands, CommandRegistry::broadcast_soft_enum_updates)
                    .chain()
                    .in_set(GameSet::Input),
            ),
        );
    }
}
