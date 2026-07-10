use crate::network::BedrockProtocol;
use crate::network::session::Session;
use crate::network::session::state::SessionState;
use crate::player::identity::PlayerIdentity;
use bedrock::protocol::ProtoVersionPackets;
use bedrock::protocol::v924::enums::TextPacketType;
use bevy_ecs::message::{Message, MessageReader, MessageWriter};
use bevy_ecs::prelude::Query;
use tracing::info;

type TextPacket = <BedrockProtocol as ProtoVersionPackets>::TextPacket;

#[derive(Message)]
pub struct PlayerChatMessage {
    pub sender_name: String,
    pub sender_xuid: String,
    pub message: String,
}

pub fn handle_text(packet: &TextPacket, identity: &PlayerIdentity, chat_writer: &mut MessageWriter<PlayerChatMessage>) {
    let TextPacketType::Chat { message, .. } = &packet.message_type else {
        return;
    };

    let message = message.trim();
    if message.is_empty() {
        return;
    }

    info!("<{}> {}", identity.name(), message);

    chat_writer.write(PlayerChatMessage {
        sender_name: identity.name().to_string(),
        sender_xuid: identity.xuid().to_string(),
        message: message.to_string(),
    });
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
