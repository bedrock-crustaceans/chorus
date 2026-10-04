use crate::schedule::GameSet;
use crate::{Tick, TickSet};
use bevy_app::{App, Plugin};
use bevy_ecs::schedule::IntoScheduleConfigs;

pub mod item;
pub mod physics;
pub mod viewers;

/// Non-player entities: physics, which players see them, and the item entity behaviour.
pub struct ActorPlugin;

impl Plugin for ActorPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<viewers::ActorShown>().add_systems(
            Tick,
            (
                item::spawn_block_drops,
                item::tick_item_entities,
                physics::apply_physics,
                item::merge_item_entities,
                item::handle_item_pickup,
                viewers::update_viewers,
                item::send_item_spawns,
                viewers::broadcast_movement,
                viewers::despawn_actors,
            )
                .chain()
                .in_set(GameSet::Actors)
                .in_set(TickSet::Update),
        );
    }
}
