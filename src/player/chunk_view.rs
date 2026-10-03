use bevy_ecs::prelude::Component;
use std::collections::{HashSet, VecDeque};

#[derive(Component, Default)]
pub struct ChunkView {
    pub(crate) dimension: i32,
    pub(crate) dimension_changes: i32,
    pub radius: i32,
    pub center: Option<(i32, i32)>,
    pub pending: VecDeque<(i32, i32)>,
    pub requested: HashSet<(i32, i32)>,
    pub prioritized_center: Option<(i32, i32)>,
    pub sent: HashSet<(i32, i32)>,
}
