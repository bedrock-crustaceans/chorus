use crate::network::BedrockProtocol;
use crate::network::handler::PacketReceivedMessage;
use crate::network::session::Session;
use bedrock::protocol::v662::packets::{ClientCacheMissResponsePacket, MissingBlobEntry};
use bevy_ecs::message::MessageReader;
use bevy_ecs::prelude::{Commands, Component, Query};
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::Arc;
use tracing::debug;
use xxhash_rust::xxh64::xxh64;

/// Past this many unanswered blobs the session gets plain chunks until the client catches up,
/// so a client that never answers can't make us hold its whole view in memory.
const MAX_PENDING_BLOBS: usize = 16384;

/// Hashes a blob the way vanilla does. The client keeps its cache on disk across servers,
/// so the id has to be the real content hash and not something we made up.
pub fn blob_id(data: &[u8]) -> u64 {
    xxh64(data, 0)
}

/// Blobs a session was told about by id and may still ask for. Only sessions whose client
/// said it supports the cache have this component.
#[derive(Component, Default)]
pub struct ClientBlobCache {
    // the count is how many chunks still wait on the blob, two chunks can easily share one
    pending: HashMap<u64, (Arc<[u8]>, u32)>,
}

impl ClientBlobCache {
    /// Whether another chunk's worth of blobs fits. When it doesn't, send that chunk the old way.
    pub fn has_room(&self, blobs: usize) -> bool {
        self.pending.len() + blobs <= MAX_PENDING_BLOBS
    }

    /// Remembers the blobs of one chunk until the client says it has or misses them.
    /// The ids must be unique within the call.
    pub fn track(&mut self, blobs: &[(u64, Arc<[u8]>)]) {
        for (id, data) in blobs {
            self.pending.entry(*id).or_insert_with(|| (data.clone(), 0)).1 += 1;
        }
    }

    fn resolve(&mut self, id: u64) -> Option<Arc<[u8]>> {
        let Entry::Occupied(mut entry) = self.pending.entry(id) else {
            return None;
        };
        let data = entry.get().0.clone();
        entry.get_mut().1 -= 1;
        if entry.get().1 == 0 {
            entry.remove();
        }
        Some(data)
    }
}

/// The client says whether it has a blob cache right after logging in. Chunks only go out
/// by id to sessions that said yes.
pub fn handle_cache_status(mut reader: MessageReader<PacketReceivedMessage>, mut commands: Commands) {
    for ev in reader.read() {
        let BedrockProtocol::ClientCacheStatusPacket(packet) = &ev.packet else {
            continue;
        };

        debug!("client blob cache supported: {}", packet.is_cache_supported);
        if packet.is_cache_supported {
            commands.entity(ev.entity).insert(ClientBlobCache::default());
        } else {
            commands.entity(ev.entity).remove::<ClientBlobCache>();
        }
    }
}

/// Answers the client's report of which blobs it already had and which it needs.
pub fn handle_blob_status(mut reader: MessageReader<PacketReceivedMessage>, mut query: Query<(&mut Session, &mut ClientBlobCache)>) {
    for ev in reader.read() {
        let BedrockProtocol::ClientCacheBlobStatusPacket(packet) = &ev.packet else {
            continue;
        };
        let Ok((mut session, mut cache)) = query.get_mut(ev.entity) else {
            continue;
        };

        for id in &packet.obtained_blobs {
            cache.resolve(*id);
        }

        // ids we never sent or already answered are skipped, the client gets nothing for those
        let missing_blobs: Vec<MissingBlobEntry> = packet
            .missing_blobs
            .iter()
            .filter_map(|&blob_id| {
                Some(MissingBlobEntry {
                    blob_id,
                    blob_data: cache.resolve(blob_id)?.to_vec(),
                })
            })
            .collect();

        if !missing_blobs.is_empty() {
            session.send(BedrockProtocol::ClientCacheMissResponsePacket(ClientCacheMissResponsePacket { missing_blobs }.into()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blob(byte: u8) -> (u64, Arc<[u8]>) {
        let data: Arc<[u8]> = Arc::from(vec![byte; 4]);
        (blob_id(&data), data)
    }

    #[test]
    fn shared_blob_stays_until_every_chunk_resolves_it() {
        let mut cache = ClientBlobCache::default();
        let shared = blob(1);
        cache.track(&[shared.clone(), blob(2)]);
        cache.track(&[shared.clone()]);

        assert!(cache.resolve(shared.0).is_some());
        assert!(cache.resolve(shared.0).is_some());
        assert!(cache.resolve(shared.0).is_none());
        assert_eq!(cache.pending.len(), 1);
    }

    #[test]
    fn room_is_capped() {
        let mut cache = ClientBlobCache::default();
        assert!(cache.has_room(MAX_PENDING_BLOBS));
        cache.track(&[blob(1)]);
        assert!(!cache.has_room(MAX_PENDING_BLOBS));
    }
}
