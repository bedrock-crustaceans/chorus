use crate::config::Config;
use crate::network::BedrockProtocol;
use crate::network::handler::PacketReceivedMessage;
use crate::network::session::Session;
use crate::network::session::state::{SessionState, SessionStateChangedMessage};
use crate::resource::ResourcePacks;
use bedrock::protocol::ProtoVersionPackets;
use bedrock::protocol::v662::enums::ResourcePackResponse;
use bedrock::protocol::v662::packets::ResourcePackChunkDataPacket;
use bedrock::protocol::v662::types::{BaseGameVersion, Experiments};
use bedrock::protocol::v818::packets::{ResourcePackEntry, ResourcePacksInfoPacket};
use bedrock::protocol::v898::packets::{PackEntry, ResourcePackStackPacket};
use bevy_ecs::message::MessageReader;
use bevy_ecs::prelude::{MessageWriter, ParamSet, Query, Res};
use tracing::warn;

pub fn handle_resource(
    config: Res<Config>,
    resource_packs: Res<ResourcePacks>,
    mut packet_reader: MessageReader<PacketReceivedMessage>,
    mut state_message_set: ParamSet<(MessageReader<SessionStateChangedMessage>, MessageWriter<SessionStateChangedMessage>)>,
    mut sessions: Query<&mut Session>,
) {
    for ev in state_message_set.p0().read() {
        if ev.to != SessionState::Resource {
            continue;
        };

        let Ok(mut session) = sessions.get_mut(ev.entity) else {
            continue;
        };

        let entries: Vec<ResourcePackEntry> = resource_packs
            .packs()
            .iter()
            .map(|pack| ResourcePackEntry {
                id: pack.uuid,
                version: pack.version.clone(),
                size: pack.size(),
                content_key: String::new(),
                sub_pack_name: String::new(),
                content_identity: String::new(),
                has_scripts: pack.has_scripts,
                is_addon: false,
                is_ray_tracing_capable: false,
                cdn_url: String::new(),
            })
            .collect();

        session.send(BedrockProtocol::ResourcePacksInfoPacket(
            ResourcePacksInfoPacket {
                resource_pack_required: config.force_accept_resource_packs,
                has_addon_packs: false,
                has_scripts: entries.iter().any(|e| e.has_scripts),
                force_disable_vibrant_visuals: config.force_disable_vibrant_visuals,
                world_template_uuid: Default::default(),
                world_template_version: String::new(),
                resource_packs: entries,
            }
            .into(),
        ));
    }

    for ev in packet_reader.read() {
        let Ok(mut session) = sessions.get_mut(ev.entity) else {
            continue;
        };

        match &ev.packet {
            BedrockProtocol::ResourcePackChunkRequestPacket(packet) => {
                handle_chunk_request(&mut session, &resource_packs, &packet.resource_name, packet.chunk);
            }
            BedrockProtocol::ResourcePackClientResponsePacket(packet) => {
                handle_client_response(&config, &mut session, &resource_packs, packet, &mut state_message_set.p1());
            }
            _ => continue,
        }
    }
}

fn handle_chunk_request(session: &mut Session, resource_packs: &ResourcePacks, resource_name: &str, chunk_id: u32) {
    let Some(pack) = resource_packs.get(resource_name) else {
        warn!("Chunk request for unknown pack: {resource_name}");
        return;
    };

    if chunk_id >= pack.chunk_count() {
        warn!("Chunk {chunk_id} out of range for pack {resource_name} (count={})", pack.chunk_count());
        return;
    }

    let byte_offset = chunk_id as u64 * crate::resource::resource_pack::CHUNK_SIZE;
    let chunk_data = pack.get_chunk(chunk_id).to_vec();

    session.send(BedrockProtocol::ResourcePackChunkDataPacket(
        ResourcePackChunkDataPacket {
            resource_name: resource_name.to_string(),
            chunk_id,
            byte_offset,
            chunk_data,
        }
        .into(),
    ));
}

fn handle_client_response(
    config: &Config,
    session: &mut Session,
    resource_packs: &ResourcePacks,
    packet: &<BedrockProtocol as ProtoVersionPackets>::ResourcePackClientResponsePacket,
    state_writer: &mut MessageWriter<SessionStateChangedMessage>,
) {
    match packet.response {
        ResourcePackResponse::Cancel => {
            session.close(Some("disconnectionScreen.noReason"));
        }
        ResourcePackResponse::Downloading => {
            // Client has begun downloading; chunk requests will follow.
        }
        ResourcePackResponse::DownloadingFinished => {
            let addon_list: Vec<PackEntry> = resource_packs
                .packs()
                .iter()
                .map(|pack| PackEntry {
                    id: pack.uuid.to_string(),
                    version: pack.version.clone(),
                    sub_pack_name: String::new(),
                })
                .collect();

            session.send(BedrockProtocol::ResourcePackStackPacket(
                ResourcePackStackPacket {
                    texture_pack_required: config.force_accept_resource_packs,
                    addon_list,
                    base_game_version: BaseGameVersion("*".to_string()),
                    experiments: Experiments {
                        experiments: vec![],
                        ever_toggled: false,
                    },
                    include_editor_packs: false,
                }
                .into(),
            ));
        }
        ResourcePackResponse::ResourcePackStackFinished => {
            session.set_state(SessionState::Setup, state_writer);
        }
    }
}
