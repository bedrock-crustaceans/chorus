use crate::network::BedrockProtocol;
use crate::network::handler::play::{PlayerJoinedMessage, PlayerQuitMessage};
use crate::network::session::Session;
use crate::network::session::state::SessionState;
use crate::player::identity::PlayerIdentity;
use bedrock::protocol::ProtoVersionPackets;
use bedrock::protocol::v924::enums::TextPacketType;
use bevy_ecs::message::{Message, MessageReader, MessageWriter, Messages};
use bevy_ecs::prelude::{Entity, Query, ResMut, Resource, World};
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

#[derive(Message, Clone)]
pub struct PlayerChatMessage {
    pub entity: Entity,
    pub sender_name: String,
    pub sender_xuid: String,
    pub message: String,
}

/// A chat message that has been past the plugins and is cleared to go out.
///
/// `PlayerChatMessage` is still written for anything that wants to observe chat; this is what
/// actually reaches players. The split exists because a plugin can cancel or rewrite a message,
/// and a Bevy message cannot be edited once written.
#[derive(Message, Clone)]
pub struct ChatApprovedMessage {
    pub entity: Entity,
    pub sender_name: String,
    pub sender_xuid: String,
    pub message: String,
}

/// Chat waiting on the plugin pass. Filled by `collect_chat`, drained by `moderate_chat`.
#[derive(Resource, Default)]
pub struct PendingChat(Vec<PlayerChatMessage>);

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

/// Copies this tick's chat into the pending queue, without consuming it, so other readers of
/// `PlayerChatMessage` still see everything.
pub fn collect_chat(mut reader: MessageReader<PlayerChatMessage>, mut pending: ResMut<PendingChat>) {
    for message in reader.read() {
        pending.0.push(message.clone());
    }
}

/// Runs each pending message past the plugins and writes the survivors out.
///
/// With no plugins loaded this is a plain forwarder, which is the point: the chat path must not
/// depend on the plugin host existing. Each plugin sees what the previous one left, and a plugin
/// that traps leaves the message as it was.
pub fn moderate_chat(world: &mut World) {
    let messages = std::mem::take(&mut world.resource_mut::<PendingChat>().0);

    for message in messages {
        let Some((text, cancelled)) = crate::plugin::events::moderate(world, &message) else {
            forward(world, message, None);
            continue;
        };
        if cancelled {
            continue;
        }
        forward(world, message, Some(text));
    }
}

fn forward(world: &mut World, message: PlayerChatMessage, rewritten: Option<String>) {
    world.resource_mut::<Messages<ChatApprovedMessage>>().write(ChatApprovedMessage {
        entity: message.entity,
        sender_name: message.sender_name,
        sender_xuid: message.sender_xuid,
        message: rewritten.unwrap_or(message.message),
    });
}

pub fn broadcast_chat(mut reader: MessageReader<ChatApprovedMessage>, mut sessions: Query<&mut Session>) {
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

pub fn announce_join_quit(mut join_reader: MessageReader<PlayerJoinedMessage>, mut quit_reader: MessageReader<PlayerQuitMessage>, mut broadcast_writer: MessageWriter<BroadcastMessage>) {
    for ev in join_reader.read() {
        broadcast_writer.write(BroadcastMessage::translate("§e%multiplayer.player.joined", vec![ev.name.clone()]));
    }

    for ev in quit_reader.read() {
        broadcast_writer.write(BroadcastMessage::translate("§e%multiplayer.player.left", vec![ev.name.clone()]));
    }
}

pub struct ChatPlugin;

impl bevy_app::Plugin for ChatPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        use bevy_ecs::schedule::IntoScheduleConfigs;
        app.init_resource::<PendingChat>().add_message::<ChatApprovedMessage>().add_systems(
            crate::Tick,
            (collect_chat, moderate_chat, broadcast_chat, announce_join_quit, broadcast_message)
                .chain()
                .in_set(crate::schedule::GameSet::Chat),
        );
    }
}
