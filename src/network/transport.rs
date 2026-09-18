use bedrock::network::info::RAKNET_GAMEPACKET_ID;
use bevy_ecs::prelude::ResMut;
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

/// Whichever transport server resource(s) are active this tick, so [`Network`](super::network::Network)'s
/// systems get one call site instead of a RakNet/NetherNet match at every send/recv/disconnect.
/// NetherNet runs its LAN and HTTP signalers side by side, since one discovers local peers and
/// the other accepts remote ones. Connect/disconnect events aren't handled here: the *ServerPlugin
/// systems already drain them into `MessageWriter<*ServerEvent>` each tick, so by the time this runs
/// the servers' own event queues are empty - read the message types directly instead.
pub enum ActiveTransport<'a> {
    RakNet(&'a mut RakServer),
    NetherNet { lan: &'a mut NetherServer, http: &'a mut NetherHttpServer },
}

impl<'a> ActiveTransport<'a> {
    pub fn from_resources(rak: &'a mut Option<ResMut<RakServer>>, nether_lan: &'a mut Option<ResMut<NetherServer>>, nether_http: &'a mut Option<ResMut<NetherHttpServer>>) -> Option<Self> {
        if let Some(server) = rak.as_deref_mut() {
            return Some(Self::RakNet(server));
        }

        if let (Some(lan), Some(http)) = (nether_lan.as_deref_mut(), nether_http.as_deref_mut()) {
            return Some(Self::NetherNet { lan, http });
        }

        None
    }

    pub fn recv(&mut self) -> Option<(SessionId, Box<[u8]>)> {
        match self {
            // Bedrock prefixes every RakNet-carried batch with RAKNET_GAMEPACKET_ID (0xFE) to tell
            // it apart from RakNet's own internal packet IDs - NetherNet has no such ambiguity to
            // resolve, so it carries the batch as-is.
            Self::RakNet(server) => {
                while let Some((id, buf)) = server.recv() {
                    match buf.split_first() {
                        Some((&RAKNET_GAMEPACKET_ID, rest)) => return Some((SessionId::RakNet(id), rest.into())),
                        _ => warn!("dropping RakNet datagram with missing/invalid game packet header"),
                    }
                }

                None
            }
            Self::NetherNet { lan, http } => {
                if let Some((id, buf)) = lan.recv() {
                    return Some((SessionId::NetherNetLan(id), buf));
                }

                http.recv().map(|(id, buf)| (SessionId::NetherNetHttp(id), buf))
            }
        }
    }

    pub fn send(&mut self, id: &SessionId, data: Vec<u8>) {
        match (self, id) {
            (Self::RakNet(server), SessionId::RakNet(id)) => {
                let mut buf = Vec::with_capacity(data.len() + 1);
                buf.push(RAKNET_GAMEPACKET_ID);
                buf.extend_from_slice(&data);

                let _ = server.send(*id, buf, RakReliability::ReliableOrdered, RakPriority::Immediate);
            }
            (Self::NetherNet { lan, .. }, SessionId::NetherNetLan(id)) => {
                let _ = lan.send(id, &data);
            }
            (Self::NetherNet { http, .. }, SessionId::NetherNetHttp(id)) => {
                let _ = http.send(id, &data);
            }
            _ => {}
        }
    }

    pub fn disconnect(&mut self, id: &SessionId) {
        match (self, id) {
            (Self::RakNet(server), SessionId::RakNet(id)) => server.disconnect(*id),
            (Self::NetherNet { lan, .. }, SessionId::NetherNetLan(id)) => lan.disconnect(id),
            (Self::NetherNet { http, .. }, SessionId::NetherNetHttp(id)) => http.disconnect(id),
            _ => {}
        }
    }
}
