use crate::util::constants::{AUTOFLUSH, AUTOFLUSH_INTERVAL_MS, MAX_ORDERING_CHANNELS, MAX_QUEUED_BYTES};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct RakSessionConfig {
    pub ordering_channels: i32,
    pub autoflush: bool,
    pub autoflush_interval_ms: Duration,
    pub max_queued_bytes: i32,
}

impl Default for RakSessionConfig {
    fn default() -> Self {
        Self {
            ordering_channels: MAX_ORDERING_CHANNELS,
            autoflush: AUTOFLUSH,
            autoflush_interval_ms: Duration::from_millis(AUTOFLUSH_INTERVAL_MS as u64),
            max_queued_bytes: MAX_QUEUED_BYTES,
        }
    }
}
