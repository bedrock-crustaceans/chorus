use bevy_ecs::prelude::Component;
use std::collections::{HashSet, VecDeque};

const LOAD_MARGIN: i32 = 1;

#[derive(Component, Default)]
pub struct ChunkView {
    pub(crate) dimension: i32,
    pub(crate) dimension_changes: i32,
    pub radius: i32,
    pub center: Option<(i32, i32)>,
    pub pending: VecDeque<(i32, i32)>,
    pub requested: HashSet<(i32, i32)>,
    pub sent: HashSet<(i32, i32)>,
}

impl ChunkView {
    pub fn load_radius(&self) -> i32 {
        self.radius + LOAD_MARGIN
    }
}
