use crate::core::config::Config;
use crate::network::BedrockProtocol;
use crate::network::handler::PacketReceivedMessage;
use crate::network::session::Session;
use crate::network::session::state::{SessionState, SessionStateChangedMessage};
use crate::resource::ResourcePacks;
use bedrock::protocol::ProtoVersionPackets;
use bedrock::protocol::v662::packets::{ResourcePackChunkDataPacket, ResourcePackDataInfoPacket};
use bedrock::protocol::v662::types::{BaseGameVersion, Experiments};
use bedrock::protocol::v898::packets::{PackEntry, ResourcePackStackPacket};
use bedrock::protocol::v2168::packets::{ResourcePackClientResponsePacket, ResourcePackEntry, ResourcePacksInfoPacket};
use bevy_ecs::message::{Message, MessageReader};
use bevy_ecs::prelude::{Entity, Local, MessageWriter, ParamSet, Query, Res};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};
use tracing::warn;

/// Gap between two chunks sent to one client. Clients after 1.26.30 can fail the download when
/// chunks arrive too fast, so this matches gophertunnel's default pacing.
const CHUNK_SEND_DELAY: Duration = Duration::from_millis(200);

/// A session's pack download: packs still to announce, and chunk requests still to answer.
#[derive(Default)]
pub struct PackDownload {
    /// Packs the client asked for that have not been announced yet. Packs go one at a time, the
    /// next one announced after the last chunk of the current one.
    queued: VecDeque<String>,
    /// Chunk requests (pack name, chunk index) in the order the client sent them.
    requests: VecDeque<(String, u32)>,
    next_send: Option<Instant>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourcePackResponseKind {
    Cancelled,
    Downloading,
    DownloadingFinished,
    StackFinished,
}

#[derive(Message, Clone, Debug)]
pub struct ResourcePackResponseMessage {
    pub entity: Entity,
    pub response: ResourcePackResponseKind,
}

pub fn handle_resource(
    config: Res<Config>,
    resource_packs: Res<ResourcePacks>,
    mut packet_reader: MessageReader<PacketReceivedMessage>,
    mut state_message_set: ParamSet<(MessageReader<SessionStateChangedMessage>, MessageWriter<SessionStateChangedMessage>)>,
    mut response_writer: MessageWriter<ResourcePackResponseMessage>,
    mut sessions: Query<&mut Session>,
    mut downloads: Local<HashMap<Entity, PackDownload>>,
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
                resource_pack_required: config.packs.force_accept,
                has_addon_packs: false,
                has_scripts: entries.iter().any(|e| e.has_scripts),
                force_disable_vibrant_visuals: config.packs.force_disable_vibrant_visuals,
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
                downloads.entry(ev.entity).or_default().requests.push_back((packet.resource_name.clone(), packet.chunk));
            }
            BedrockProtocol::ResourcePackClientResponsePacket(packet) => {
                handle_client_response(
                    ev.entity,
                    &config,
                    &mut session,
                    &resource_packs,
                    packet,
                    &mut downloads,
                    &mut state_message_set.p1(),
                    &mut response_writer,
                );
            }
            _ => continue,
        }
    }

    send_chunks(&mut downloads, &mut sessions, &resource_packs);
}

/// Answers at most one chunk request per session every [`CHUNK_SEND_DELAY`], and announces the
/// next queued pack once the current one's last chunk is out.
fn send_chunks(downloads: &mut HashMap<Entity, PackDownload>, sessions: &mut Query<&mut Session>, resource_packs: &ResourcePacks) {
    let now = Instant::now();

    downloads.retain(|&entity, download| {
        let Ok(mut session) = sessions.get_mut(entity) else {
            return false;
        };
        if session.is_closed() || session.get_state() != SessionState::Resource {
            return false;
        }
        if download.next_send.is_some_and(|at| now < at) {
            return true;
        }
        let Some((resource_name, chunk_id)) = download.requests.pop_front() else {
            return true;
        };

        if let Some(last_chunk) = send_chunk(&mut session, resource_packs, &resource_name, chunk_id) {
            download.next_send = Some(now + CHUNK_SEND_DELAY);
            if last_chunk {
                announce_next(&mut session, resource_packs, download);
            }
        }
        true
    });
}

/// Sends the next queued pack's size, chunk layout and hash, which the client needs before it
/// requests that pack's chunks. Unknown packs are skipped.
fn announce_next(session: &mut Session, resource_packs: &ResourcePacks, download: &mut PackDownload) {
    let pack = loop {
        let Some(pack_id) = download.queued.pop_front() else {
            return;
        };
        match resource_packs.get(&pack_id) {
            Some(pack) => break pack,
            None => warn!("Client requested unknown pack: {pack_id}"),
        }
    };

    session.send(BedrockProtocol::ResourcePackDataInfoPacket(
        ResourcePackDataInfoPacket {
            resource_name: pack.pack_id(),
            chunk_size: crate::resource::resource_pack::CHUNK_SIZE as u32,
            chunk_amount: pack.chunk_count(),
            file_size: pack.size(),
            file_hash: pack.hash.clone(),
            is_premium: false,
            pack_type: pack.pack_type.clone(),
        }
        .into(),
    ));
}

/// Sends one chunk, returning whether it was the pack's last, or `None` if the request was invalid.
fn send_chunk(session: &mut Session, resource_packs: &ResourcePacks, resource_name: &str, chunk_id: u32) -> Option<bool> {
    let Some(pack) = resource_packs.get(resource_name) else {
        warn!("Chunk request for unknown pack: {resource_name}");
        return None;
    };

    if chunk_id >= pack.chunk_count() {
        warn!("Chunk {chunk_id} out of range for pack {resource_name} (count={})", pack.chunk_count());
        return None;
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

    Some(chunk_id + 1 == pack.chunk_count())
}

#[allow(clippy::too_many_arguments)]
fn handle_client_response(
    entity: Entity,
    config: &Config,
    session: &mut Session,
    resource_packs: &ResourcePacks,
    packet: &<BedrockProtocol as ProtoVersionPackets>::ResourcePackClientResponsePacket,
    downloads: &mut HashMap<Entity, PackDownload>,
    state_writer: &mut MessageWriter<SessionStateChangedMessage>,
    response_writer: &mut MessageWriter<ResourcePackResponseMessage>,
) {
    match packet {
        ResourcePackClientResponsePacket::Cancel => {
            response_writer.write(ResourcePackResponseMessage {
                entity,
                response: ResourcePackResponseKind::Cancelled,
            });

            downloads.remove(&entity);
            session.close(Some("disconnectionScreen.noReason"));
        }
        ResourcePackClientResponsePacket::Downloading(pack_ids) => {
            // The client only requests chunks after it has been told a pack's size and hash.
            // Packs are announced one at a time, like gophertunnel does.
            let download = downloads.entry(entity).or_default();
            download.queued.extend(pack_ids.iter().cloned());
            announce_next(session, resource_packs, download);

            response_writer.write(ResourcePackResponseMessage {
                entity,
                response: ResourcePackResponseKind::Downloading,
            });
        }
        ResourcePackClientResponsePacket::DownloadingFinished => {
            response_writer.write(ResourcePackResponseMessage {
                entity,
                response: ResourcePackResponseKind::DownloadingFinished,
            });
            downloads.remove(&entity);

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
                    texture_pack_required: config.packs.force_accept,
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
        ResourcePackClientResponsePacket::ResourcePackStackFinished => {
            response_writer.write(ResourcePackResponseMessage {
                entity,
                response: ResourcePackResponseKind::StackFinished,
            });

            session.set_state(SessionState::Setup, state_writer);
        }
    }
}
