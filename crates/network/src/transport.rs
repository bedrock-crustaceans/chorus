use crate::session::Batch;
use bevy_ecs::message::Message;
use std::sync::Arc;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct SessionId {
    pub transport: &'static str,
    pub key: Arc<str>,
    pub encryption: bool,
}

impl SessionId {
    pub fn new(transport: &'static str, key: impl Into<Arc<str>>, encryption: bool) -> Self {
        Self {
            transport,
            key: key.into(),
            encryption,
        }
    }

    pub fn supports_encryption(&self) -> bool {
        self.encryption
    }
}

#[derive(Message)]
pub struct SendBatchMessage {
    pub id: SessionId,
    pub batch: Batch,
}

#[derive(Message)]
pub struct DisconnectSessionMessage {
    pub id: SessionId,
}
