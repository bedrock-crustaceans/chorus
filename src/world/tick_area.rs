use crate::core::config::LevelConfig;
use crate::level::level::Level;
use crate::player::chunk_view::ChunkView;
use bevy_ecs::prelude::{Query, Res, ResMut};
use std::collections::HashMap;

pub fn update_tick_areas(mut level: ResMut<Level>, config: Res<LevelConfig>, views: Query<&ChunkView>) {
    let mut centres: HashMap<i32, Vec<(i32, i32)>> = HashMap::new();
    for view in &views {
        if let Some(center) = view.center {
            centres.entry(view.dimension).or_default().push(center);
        }
    }
    level.set_tick_areas(&centres, config.tick_distance);
}
