use crate::network::BedrockProtocol;
use crate::network::handler::PacketReceivedMessage;
use crate::network::session::Session;
use crate::network::session::state::{SessionState, SessionStateChangedMessage};
use bevy_ecs::message::MessageReader;
use bevy_ecs::prelude::{MessageWriter, Query};

pub fn handle_handshake(mut reader: MessageReader<PacketReceivedMessage>, mut writer: MessageWriter<SessionStateChangedMessage>, mut sessions: Query<&mut Session>) {
    for ev in reader.read() {
        let Ok(mut session) = sessions.get_mut(ev.entity) else {
            continue;
        };

        if session.get_state() != SessionState::Handshake {
            continue;
        };

        let BedrockProtocol::ClientToServerHandshakePacket(_) = &ev.packet else {
            continue;
        };

        session.set_state(SessionState::Resource, &mut writer);
    }
}
