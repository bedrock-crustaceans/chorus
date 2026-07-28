use crate::network::BedrockProtocol;
use crate::network::session::Session;
use crate::player::Player;
use bedrock::protocol::ProtoVersionPackets;
use bedrock::protocol::v924::enums::TextPacketType;

type TextPacket = <BedrockProtocol as ProtoVersionPackets>::TextPacket;

pub struct CommandSender<'a> {
    session: &'a mut Session,
    name: String,
    player: Option<&'a mut Player>,
}

impl<'a> CommandSender<'a> {
    pub fn new(session: &'a mut Session, name: String, player: Option<&'a mut Player>) -> Self {
        Self { session, name, player }
    }

    pub fn session(&self) -> &Session {
        self.session
    }

    pub fn session_mut(&mut self) -> &mut Session {
        self.session
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn player(&self) -> Option<&Player> {
        self.player.as_deref()
    }

    pub fn player_mut(&mut self) -> Option<&mut Player> {
        self.player.as_deref_mut()
    }

    pub fn split(&mut self) -> (&mut Session, Option<&mut Player>) {
        (self.session, self.player.as_deref_mut())
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
