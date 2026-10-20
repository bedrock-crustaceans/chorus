use crate::world::chunks::ViewStats;
use glam::{Vec2, Vec3};

#[derive(Clone, Copy)]
pub struct Memory {
    pub resident_bytes: u64,
    pub virtual_bytes: u64,
}

#[derive(Clone, Copy, Default)]
pub struct ServerSample {
    pub tps: f64,
    pub mspt: f64,
    pub tick_usage: f64,
    pub memory: Option<Memory>,
    pub players: usize,
}

#[derive(Clone, Copy, Default)]
pub struct PlayerSample {
    pub position: Vec3,
    pub rotation: Vec2,
    pub view: ViewStats,
    pub loaded_chunks: usize,
}

impl PlayerSample {
    pub fn chunk(&self) -> (i32, i32) {
        (self.position.x.floor() as i32 >> 4, self.position.z.floor() as i32 >> 4)
    }
}

#[derive(Clone, Copy, Default)]
pub struct Sample {
    pub server: ServerSample,
    pub player: PlayerSample,
}

pub fn fraction(value: usize, total: usize) -> f32 {
    if total == 0 { 1.0 } else { (value as f32 / total as f32).clamp(0.0, 1.0) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fraction_of_empty_total_is_complete() {
        assert_eq!(fraction(0, 0), 1.0);
    }

    #[test]
    fn fraction_is_clamped() {
        assert_eq!(fraction(5, 10), 0.5);
        assert_eq!(fraction(20, 10), 1.0);
    }

    #[test]
    fn chunk_uses_floor_division() {
        let sample = PlayerSample {
            position: Vec3::new(-0.5, 70.0, 31.9),
            ..Default::default()
        };
        assert_eq!(sample.chunk(), (-1, 1));
    }
}
