use crate::chat::{BroadcastMessage, PlayerChatMessage};
use crate::command::dispatch::{CommandPreprocessMessage, CommandRequestedMessage};
use crate::item::ItemTakenMessage;
use crate::level::{BlockUpdatedMessage, LevelEventMessage, LevelSoundMessage};
use crate::network::bandwidth::BandwidthTracker;
use crate::network::handler::PacketHandlers;
use crate::network::handler::form::FormResponseMessage;
use crate::network::handler::inventory::{InventoryCloseMessage, InventoryOpenMessage, ItemDropMessage, PlayerItemHeldMessage};
use crate::network::handler::login::{PlayerLoginMessage, PlayerPreLoginMessage};
use crate::network::handler::play::{PlayerJoinedMessage, PlayerMoveMessage, PlayerQuitMessage, QuitAnnounced};
use crate::network::handler::resource::ResourcePackResponseMessage;
use crate::network::session::state::SessionState;
use crate::network::session::{Batch, Session};
use crate::network::transport::{DisconnectSessionMessage, SendBatchMessage};
use crate::network::{NetworkPlugin, NetworkSet, NetworkState};
use crate::server::ServerState;
use crate::world::block::{BlockBreakMessage, BlockPlaceMessage};
use crate::{Tick, TickSet};
use bevy_app::{App, Plugin, PostUpdate};
use bevy_ecs::prelude::*;
use bevy_tasks::ComputeTaskPool;
use std::collections::HashMap;

pub struct Network;

impl Plugin for Network {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<NetworkPlugin>() {
            app.add_plugins(NetworkPlugin);
        }

        app.add_plugins((PacketHandlers, crate::world::WorldPlugin, crate::actor::ActorPlugin, crate::chat::ChatPlugin))
            .add_systems(PostUpdate, Network::flush.in_set(NetworkSet::Flush))
            .add_systems(Tick, Network::sample_bandwidth.in_set(TickSet::Last))
            .add_message::<PlayerJoinedMessage>()
            .add_message::<PlayerQuitMessage>()
            .add_message::<PlayerLoginMessage>()
            .add_message::<PlayerPreLoginMessage>()
            .add_message::<PlayerMoveMessage>()
            .add_message::<ResourcePackResponseMessage>()
            .add_message::<BlockUpdatedMessage>()
            .add_message::<BlockBreakMessage>()
            .add_message::<BlockPlaceMessage>()
            .add_message::<ItemTakenMessage>()
            .add_message::<InventoryOpenMessage>()
            .add_message::<InventoryCloseMessage>()
            .add_message::<PlayerItemHeldMessage>()
            .add_message::<ItemDropMessage>()
            .add_message::<FormResponseMessage>()
            .add_message::<PlayerChatMessage>()
            .add_message::<CommandPreprocessMessage>()
            .add_message::<CommandRequestedMessage>()
            .add_message::<BroadcastMessage>()
            .add_message::<LevelEventMessage>()
            .add_message::<LevelSoundMessage>();
    }
}

impl Network {
    fn sample_bandwidth(mut tracker: ResMut<BandwidthTracker>, state: Res<ServerState>) {
        tracker.sample(state.tick());
    }

    /// Pushes everything the handlers queued this tick out, and reaps closed sessions.
    pub fn flush(
        mut sends: MessageWriter<SendBatchMessage>,
        mut disconnects: MessageWriter<DisconnectSessionMessage>,
        mut state: ResMut<NetworkState>,
        bandwidth: Res<BandwidthTracker>,
        mut query: Query<(Entity, &mut Session)>,
        announced: Query<(), With<QuitAnnounced>>,
        mut commands: Commands,
    ) {
        let mut sessions: Vec<(Entity, Mut<Session>)> = query.iter_mut().collect();

        let mut busy: Vec<usize> = Vec::new();
        for (index, (_, session)) in sessions.iter().enumerate() {
            if session.has_queued() {
                busy.push(index);
            }
        }
        let mut encoded: HashMap<usize, Vec<Batch>> = HashMap::new();
        if !busy.is_empty() {
            let mut selected: Vec<(usize, &mut Mut<Session>)> = Vec::new();
            for (index, (_, session)) in sessions.iter_mut().enumerate() {
                if busy.binary_search(&index).is_ok() {
                    selected.push((index, session));
                }
            }
            let results = ComputeTaskPool::get().scope(|scope| {
                for (index, session) in selected {
                    scope.spawn(async move { (index, session.take_outgoing()) });
                }
            });
            encoded.extend(results);
        }

        for (index, (entity, mut session)) in sessions.into_iter().enumerate() {
            let batches = match encoded.remove(&index) {
                Some(batches) => batches,
                None => session.take_outgoing(),
            };
            for batch in batches {
                bandwidth.counters().add_sent(batch.data.len() as u64);
                sends.write(SendBatchMessage { id: session.id.clone(), batch });
            }

            let announced = session.get_state() != SessionState::Play || announced.contains(entity);
            if session.is_closed() && announced {
                disconnects.write(DisconnectSessionMessage { id: session.id.clone() });
                state.forget(&session.id);
                commands.entity(entity).despawn();
            }
        }
    }
}
