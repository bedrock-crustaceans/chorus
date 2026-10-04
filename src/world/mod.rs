use crate::Tick;
use crate::schedule::GameSet;
use bevy_app::{App, Plugin};
use bevy_ecs::schedule::IntoScheduleConfigs;

pub mod block;
pub mod chunks;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<block::BlockActionMessage>().add_systems(
            Tick,
            (
                (block::handle_block_actions, block::update_block_breaking).chain().in_set(GameSet::World),
                (chunks::update_chunk_order, chunks::send_pending_chunks, chunks::unload_distant_chunks, chunks::handle_sub_chunk_request)
                    .chain()
                    .in_set(GameSet::Chunks),
                (block::broadcast_block_updates, block::broadcast_level_events, block::broadcast_level_sounds)
                    .chain()
                    .in_set(GameSet::Broadcast),
            ),
        );
    }
}
