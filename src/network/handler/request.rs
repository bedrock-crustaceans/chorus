use crate::network::BedrockProtocol;
use crate::network::handler::PacketReceivedMessage;
use crate::network::session::Session;
use crate::network::session::state::{SessionState, SessionStateChangedMessage};
use bedrock::network::compression::Compression;
use bedrock::protocol::ProtoVersion;
use bedrock::protocol::v662::enums::{PacketCompressionAlgorithm, PlayStatus};
use bedrock::protocol::v662::packets::NetworkSettingsPacket;
use bevy_ecs::message::MessageReader;
use bevy_ecs::prelude::MessageWriter;
use bevy_ecs::system::Query;
use tracing::error;

pub fn handle_request(mut reader: MessageReader<PacketReceivedMessage>, mut writer: MessageWriter<SessionStateChangedMessage>, mut sessions: Query<&mut Session>) {
    for ev in reader.read() {
        if let Ok(mut session) = sessions.get_mut(ev.entity) {
            if session.get_state() != SessionState::Request {
                continue;
            }

            let BedrockProtocol::RequestNetworkSettingsPacket(packet) = &ev.packet else {
                continue;
            };

            let protocol = packet.client_network_version as u32;

            if protocol != BedrockProtocol::PROTOCOL_VERSION {
                session.send_play_status(
                    if protocol < BedrockProtocol::PROTOCOL_VERSION {
                        PlayStatus::LoginFailedClientOld
                    } else {
                        PlayStatus::LoginFailedServerOld
                    },
                    true,
                );

                session.close(if protocol < BedrockProtocol::PROTOCOL_VERSION {
                    Some("disconnectionScreen.outdatedClient")
                } else {
                    Some("disconnectionScreen.outdatedServer")
                });
            }

            let compression_threshold: u16 = 1;

            // TODO: IP Bans
            session.send_immediate(BedrockProtocol::NetworkSettingsPacket(
                NetworkSettingsPacket {
                    compression_threshold,
                    compression_algorithm: PacketCompressionAlgorithm::ZLib,
                    client_throttle_enabled: false,
                    client_throttle_threshold: 0,
                    client_throttle_scalar: 0.0,
                }
                .into(),
            ));

            session.set_compression(Some(Compression::Zlib {
                compression_level: 6,
                threshold: compression_threshold,
            }));

            session.set_state(SessionState::Login, &mut writer);
        } else {
            error!("received PacketReceivedMessage from entity without a Session!")
        }
    }
}
