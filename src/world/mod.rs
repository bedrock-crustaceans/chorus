use crate::schedule::GameSet;
use crate::{Priority, Scheduler, Tick, TickSet};
use bevy_app::{App, Plugin};
use bevy_ecs::schedule::IntoScheduleConfigs;

pub mod blob_cache;
pub mod block;
pub mod chunks;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        Scheduler::register::<chunks::ChunkSendTask, _>(app.world_mut(), Priority::Io, chunks::transmit_pending_chunks);
        app.add_message::<block::BlockActionMessage>()
            .add_message::<block::BlockEditRejected>()
            .init_resource::<chunks::ChunkSerializer>()
            .add_systems(
                Tick,
                (
                    blob_cache::handle_cache_status.in_set(GameSet::Connection),
                    (block::handle_block_actions, block::update_block_breaking).chain().in_set(GameSet::World),
                    (
                        blob_cache::handle_blob_status,
                        chunks::update_chunk_order,
                        chunks::request_view_chunks,
                        chunks::unload_distant_chunks,
                        chunks::handle_sub_chunk_request,
                    )
                        .chain()
                        .in_set(GameSet::Chunks),
                ),
            )
            .add_systems(
                Tick,
                (
                    block::resync_rejected_edits,
                    block::broadcast_block_updates,
                    block::broadcast_level_events,
                    block::broadcast_level_sounds,
                )
                    .chain()
                    .in_set(TickSet::Last),
            );
    }
}
