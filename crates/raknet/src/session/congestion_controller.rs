use crate::util::constants::{CC_ADDITIONAL_VARIANCE, CC_MAX_THRESHOLD};
use std::cmp::max;
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

#[derive(Clone, Debug)]
pub struct RakCongestionController {
    mtu: usize,

    congestion_window: f64,
    congestion_recovery_sequence: Option<u32>,

    slow_start_threshold: f64,

    rtt_estimate_ms: f64,
    rtt_deviation_ms: f64,

    bytes_not_acknowledged: usize,

    sent_times: HashMap<u32, SystemTime>,
}

impl RakCongestionController {
    pub fn new(mtu: usize) -> Self {
        Self {
            mtu,
            congestion_window: mtu as f64,
            congestion_recovery_sequence: None,

            slow_start_threshold: 0.0,

            rtt_estimate_ms: f64::INFINITY,
            rtt_deviation_ms: f64::INFINITY,

            bytes_not_acknowledged: 0,

            sent_times: HashMap::new(),
        }
    }

    pub fn transmission_bandwidth(&self) -> usize {
        let cwnd = self.congestion_window as isize;
        let used = self.bytes_not_acknowledged as isize;
        max(0, cwnd - used) as usize
    }

    pub fn retransmission_bandwidth(&self) -> usize {
        self.bytes_not_acknowledged
    }

    pub fn retransmission_timeout(&self) -> Duration {
        if self.rtt_estimate_ms.is_infinite() {
            return Duration::from_millis(CC_MAX_THRESHOLD as u64);
        }

        let threshold_ms = 2.0 * self.rtt_estimate_ms + 4.0 * self.rtt_deviation_ms + CC_ADDITIONAL_VARIANCE as f64;

        Duration::from_millis(threshold_ms.min(CC_MAX_THRESHOLD as f64) as u64)
    }

    pub fn rtt(&self) -> Duration {
        if self.rtt_estimate_ms.is_infinite() {
            Duration::from_millis(0)
        } else {
            Duration::from_millis(self.rtt_estimate_ms as u64)
        }
    }

    pub fn slow_start(&self) -> bool {
        self.congestion_window <= self.slow_start_threshold || self.slow_start_threshold == 0.0
    }

    pub fn resent(&mut self, sequence: u32) {
        if self.congestion_recovery_sequence.is_none() {
            self.slow_start_threshold = (self.congestion_window * 0.5).max(self.mtu as f64);
            self.congestion_window = self.mtu as f64;

            self.congestion_recovery_sequence = Some(sequence);
        }
    }

    pub fn nacked(&mut self) {
        if self.congestion_recovery_sequence.is_some() {
            self.slow_start_threshold = self.congestion_window * 0.75;
        }
    }

    pub fn acked(&mut self, now: SystemTime, seq: u32, size: usize, last_sequence: u32) {
        if let Some(sent_at) = self.sent_times.remove(&seq) {
            let rtt_ms = now.duration_since(sent_at).unwrap().as_secs_f64() * 1000.0;
            self.bytes_not_acknowledged -= size;

            if self.rtt_estimate_ms.is_infinite() {
                self.rtt_estimate_ms = rtt_ms;
                self.rtt_deviation_ms = rtt_ms;
            } else {
                let d = 0.05;
                let diff = rtt_ms - self.rtt_estimate_ms;

                self.rtt_estimate_ms += d * diff;
                self.rtt_deviation_ms += d * (diff.abs() - self.rtt_deviation_ms);
            }

            let in_recovery_period = match self.congestion_recovery_sequence {
                Some(rec_seq) => seq < rec_seq,
                None => false,
            };

            if in_recovery_period {
                self.congestion_recovery_sequence = Some(last_sequence);
            }

            if self.slow_start() {
                self.congestion_window += self.mtu as f64;

                if self.congestion_window > self.slow_start_threshold && self.slow_start_threshold != 0.0 {
                    self.congestion_window = self.slow_start_threshold + (self.mtu as f64).powi(2) / self.congestion_window;
                }
            } else if in_recovery_period {
                self.congestion_window += (self.mtu as f64).powi(2) / self.congestion_window;
            }
        }
    }

    pub fn sent(&mut self, seq: u32, size: usize, now: SystemTime) {
        self.bytes_not_acknowledged += size;
        self.sent_times.insert(seq, now);
    }
}
