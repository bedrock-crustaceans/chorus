use bevy_ecs::prelude::*;
use std::collections::{HashSet, VecDeque};

#[derive(Component)]
pub struct Player {
    unique_id: i64,
    runtime_id: u64,

    pub chunks_radius: i32,
    pub chunks_center: (i32, i32),
    pub chunks_pending: VecDeque<(i32, i32)>,
    pub chunks_sent: HashSet<(i32, i32)>,
}

impl Player {
    pub fn new(runtime_id: u64) -> Self {
        Self {
            unique_id: rand::random(),
            runtime_id,

            chunks_radius: 0,
            chunks_center: (0, 0),
            chunks_pending: VecDeque::new(),
            chunks_sent: HashSet::new(),
        }
    }

    pub fn unique_id(&self) -> i64 {
        self.unique_id
    }

    pub fn runtime_id(&self) -> u64 {
        self.runtime_id
    }
}
