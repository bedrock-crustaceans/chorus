use crate::schedule::GameSet;
use crate::{Tick, TickSet};
use bevy_app::{App, Plugin};
use bevy_ecs::schedule::IntoScheduleConfigs;

pub mod item;

pub struct ActorPlugin;

impl Plugin for ActorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Tick,
            (
                item::spawn_block_drops,
                item::broadcast_spawned_items,
                item::show_items_to_new_viewers,
                item::tick_item_entities,
                item::merge_item_entities,
                item::handle_item_pickup,
                item::broadcast_taken_items,
            )
                .chain()
                .in_set(GameSet::Actors)
                .in_set(TickSet::Update),
        );
    }
}
