use crate::network::BedrockProtocol;
use crate::network::session::Session;
use crate::network::session::state::SessionState;
use crate::player::identity::PlayerIdentity;
use bedrock::protocol::ProtoVersionPackets;
use bedrock::protocol::v924::enums::TextPacketType;
use bevy_ecs::message::{Message, MessageReader, MessageWriter};
use bevy_ecs::prelude::{Entity, Query};
use tracing::info;

type TextPacket = <BedrockProtocol as ProtoVersionPackets>::TextPacket;

/// A system message shown to every player in game, the equivalent of PocketMine's
/// Server::broadcastMessage. The client resolves any `%key` in the message against its own
/// language files and fills the parameters in, so the text stays in the player's language.
#[derive(Message)]
pub struct BroadcastMessage {
    pub message: String,
    pub parameters: Vec<String>,
}

impl BroadcastMessage {
    pub fn translate(message: impl Into<String>, parameters: Vec<String>) -> Self {
        Self { message: message.into(), parameters }
    }
}

#[derive(Message)]
pub struct PlayerChatMessage {
    pub entity: Entity,
    pub sender_name: String,
    pub sender_xuid: String,
    pub message: String,
}

pub fn handle_text(entity: Entity, packet: &TextPacket, identity: &PlayerIdentity, chat_writer: &mut MessageWriter<PlayerChatMessage>) {
    let TextPacketType::Chat { message, .. } = &packet.message_type else {
        return;
    };

    let message = message.trim();
    if message.is_empty() {
        return;
    }

    info!("<{}> {}", identity.name(), message);

    chat_writer.write(PlayerChatMessage {
        entity,
        sender_name: identity.name().to_string(),
        sender_xuid: identity.xuid().to_string(),
        message: message.to_string(),
    });
}

pub fn broadcast_message(mut reader: MessageReader<BroadcastMessage>, mut sessions: Query<&mut Session>) {
    for msg in reader.read() {
        for mut session in &mut sessions {
            if session.get_state() != SessionState::Play {
                continue;
            }

            session.send(BedrockProtocol::TextPacket(
                TextPacket {
                    localize: true,
                    message_type: TextPacketType::Translate {
                        message: msg.message.clone(),
                        parameter_list: msg.parameters.clone(),
                    },
                    sender_xuid: String::new(),
                    platform_id: String::new(),
                    filtered_message: None,
                }
                .into(),
            ));
        }
    }
}

pub fn broadcast_chat(mut reader: MessageReader<PlayerChatMessage>, mut sessions: Query<&mut Session>) {
    for msg in reader.read() {
        for mut session in &mut sessions {
            if session.get_state() != SessionState::Play {
                continue;
            }

            session.send(BedrockProtocol::TextPacket(
                TextPacket {
                    localize: false,
                    message_type: TextPacketType::Chat {
                        player_name: msg.sender_name.clone(),
                        message: msg.message.clone(),
                    },
                    sender_xuid: msg.sender_xuid.clone(),
                    platform_id: String::new(),
                    filtered_message: None,
                }
                .into(),
            ));
        }
    }
}
