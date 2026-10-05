use crate::network::session::Batch;
use bedrock::network::info::RAKNET_GAMEPACKET_ID;
use bevy_ecs::prelude::ResMut;
use bevy_ecs::system::SystemParam;
use bevy_nethernet::prelude::{NetherHttpServer, NetherServer, NetherSessionId};
use bevy_raknet::prelude::{RakPriority, RakReliability, RakServer, RakSessionId};
use tracing::warn;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum SessionId {
    RakNet(RakSessionId),
    NetherNetLan(NetherSessionId),
    NetherNetHttp(NetherSessionId),
}

impl SessionId {
    /// NetherNet already carries WebRTC/DTLS encryption, so layering the login handshake's
    /// own encryption on top would be redundant, and vanilla clients never complete that
    /// handshake over NetherNet in the first place.
    pub fn supports_encryption(&self) -> bool {
        matches!(self, Self::RakNet(_))
    }
}

#[derive(SystemParam)]
pub struct Transports<'w> {
    rak: Option<ResMut<'w, RakServer>>,
    nether_lan: Option<ResMut<'w, NetherServer>>,
    nether_http: Option<ResMut<'w, NetherHttpServer>>,
}

impl Transports<'_> {
    pub fn recv(&mut self) -> Option<(SessionId, Box<[u8]>)> {
        if let Some(server) = self.rak.as_mut() {
            while let Some((id, buf)) = server.recv() {
                match buf.split_first() {
                    Some((&RAKNET_GAMEPACKET_ID, rest)) => return Some((SessionId::RakNet(id), rest.into())),
                    _ => warn!("dropping RakNet datagram with missing/invalid game packet header"),
                }
            }
        }
        if let Some((id, buf)) = self.nether_lan.as_mut().and_then(|server| server.recv()) {
            return Some((SessionId::NetherNetLan(id), buf));
        }
        self.nether_http.as_mut().and_then(|server| server.recv()).map(|(id, buf)| (SessionId::NetherNetHttp(id), buf))
    }

    pub fn send(&mut self, id: &SessionId, batch: Batch) {
        match id {
            SessionId::RakNet(id) => {
                if let Some(server) = self.rak.as_mut() {
                    let mut buf = Vec::with_capacity(batch.data.len() + 1);
                    buf.push(RAKNET_GAMEPACKET_ID);
                    buf.extend_from_slice(&batch.data);
                    let priority = if batch.immediate { RakPriority::Immediate } else { RakPriority::Normal };
                    let _ = server.send(*id, buf, RakReliability::ReliableOrdered, priority);
                }
            }
            SessionId::NetherNetLan(id) => {
                if let Some(server) = self.nether_lan.as_mut() {
                    let _ = server.send(id, &batch.data);
                }
            }
            SessionId::NetherNetHttp(id) => {
                if let Some(server) = self.nether_http.as_mut() {
                    let _ = server.send(id, &batch.data);
                }
            }
        }
    }

    pub fn disconnect(&mut self, id: &SessionId) {
        match id {
            SessionId::RakNet(id) => self.rak.as_mut().map(|server| server.disconnect(*id)),
            SessionId::NetherNetLan(id) => self.nether_lan.as_mut().map(|server| server.disconnect(id)),
            SessionId::NetherNetHttp(id) => self.nether_http.as_mut().map(|server| server.disconnect(id)),
        };
    }
}
