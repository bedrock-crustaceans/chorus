use crate::Tick;
use crate::schedule::GameSet;
use bevy_app::{App, Plugin};
use bevy_ecs::schedule::IntoScheduleConfigs;

pub mod item;
pub mod physics;
pub mod player;
pub mod storage;
pub mod viewers;

pub struct ActorPlugin;

impl Plugin for ActorPlugin {
    fn build(&self, app: &mut App) {
        let mut registry = storage::ActorRegistry::default();
        registry.register(item::ITEM_IDENTIFIER, item::save_item, item::load_item);
        app.insert_resource(registry).add_message::<viewers::ActorShown>().add_systems(
            Tick,
            (
                storage::load_chunk_actors,
                storage::save_unloaded_chunk_actors,
                player::handle_player_joins,
                player::handle_player_quits,
                item::spawn_block_drops,
                item::spawn_player_drops,
                item::tick_item_entities,
                physics::apply_physics,
                item::merge_item_entities,
                item::handle_item_pickup,
                viewers::update_viewers,
                item::send_item_spawns,
                player::send_player_spawns,
                player::broadcast_gamemode_changes,
                viewers::broadcast_movement,
                viewers::despawn_actors,
            )
                .chain()
                .in_set(GameSet::Actors),
        );
    }
}
