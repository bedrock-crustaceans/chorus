use crate::server::{ServerState, TICK_RATE};
use crate::utils::rolling_avg::RollingAvg;
use bevy_ecs::prelude::{Res, ResMut, Resource};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

const SAMPLE_WINDOW: usize = 60;

/// Byte counters shared with the connection tasks.
#[derive(Default)]
pub struct BandwidthCounters {
    sent: AtomicU64,
    received: AtomicU64,
}

impl BandwidthCounters {
    pub fn add_sent(&self, bytes: u64) {
        self.sent.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn add_received(&self, bytes: u64) {
        self.received.fetch_add(bytes, Ordering::Relaxed);
    }

    fn sent(&self) -> u64 {
        self.sent.load(Ordering::Relaxed)
    }

    fn received(&self) -> u64 {
        self.received.load(Ordering::Relaxed)
    }
}

#[derive(Resource)]
pub struct BandwidthTracker {
    counters: Arc<BandwidthCounters>,

    last_sent: u64,
    last_received: u64,

    sent_avg: RollingAvg<f64>,
    received_avg: RollingAvg<f64>,
}

impl Default for BandwidthTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl BandwidthTracker {
    pub fn new() -> Self {
        Self {
            counters: Arc::new(BandwidthCounters::default()),

            last_sent: 0,
            last_received: 0,

            sent_avg: RollingAvg::new(SAMPLE_WINDOW),
            received_avg: RollingAvg::new(SAMPLE_WINDOW),
        }
    }

    pub fn counters(&self) -> Arc<BandwidthCounters> {
        self.counters.clone()
    }

    /// Average bytes sent per second over the last minute.
    pub fn average_sent(&self) -> f64 {
        self.sent_avg.get_avg()
    }

    /// Average bytes received per second over the last minute.
    pub fn average_received(&self) -> f64 {
        self.received_avg.get_avg()
    }

    /// Takes one sample per second, so the averages come out as bytes per second.
    pub fn sample(mut tracker: ResMut<Self>, state: Res<ServerState>) {
        if state.tick() % TICK_RATE as i64 != 0 {
            return;
        }

        let sent = tracker.counters.sent();
        let received = tracker.counters.received();

        let since_last_sent = sent.saturating_sub(tracker.last_sent);
        let since_last_received = received.saturating_sub(tracker.last_received);

        tracker.sent_avg.add(since_last_sent as f64);
        tracker.received_avg.add(since_last_received as f64);

        tracker.last_sent = sent;
        tracker.last_received = received;
    }
}
