use crate::network::BedrockProtocol;
use crate::network::session::Session;
use bedrock::protocol::ProtoVersionPackets;
use bedrock::protocol::v924::enums::TextPacketType;

type TextPacket = <BedrockProtocol as ProtoVersionPackets>::TextPacket;

pub struct CommandSender<'a> {
    session: &'a mut Session,
    name: String,
}

impl<'a> CommandSender<'a> {
    pub fn new(session: &'a mut Session, name: String) -> Self {
        Self { session, name }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn reply(&mut self, message: impl Into<String>) {
        self.session.send(BedrockProtocol::TextPacket(
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
