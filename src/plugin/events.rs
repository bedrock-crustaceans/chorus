//! Handing game events to plugins.
//!
//! Collection and dispatch are two systems on purpose. Reading messages wants a normal system with
//! `MessageReader`s; calling into a plugin wants `&mut World`, because a plugin can call straight
//! back into the server. So the first system snapshots what happened into a queue and the second
//! drains it.

use bevy_ecs::message::MessageReader;
use bevy_ecs::prelude::{Entity, Mut, Query, ResMut, Resource, World};
use chorus_entity::components::transform::Transform;
use chorus_plugin::chorus::plugin::common::{BlockPos, GameMode, Position};
use chorus_plugin::chorus::plugin::event::{BlockBreakData, BlockPlaceData, Event, PlayerChatData, PlayerJoinData, PlayerLeaveData};
use chorus_plugin::chorus::plugin::player::Player as WitPlayer;
use chorus_plugin::event::EventId;
use glam::IVec3;
use tracing::warn;

use crate::chat::PlayerChatMessage;
use crate::network::handler::play::{PlayerJoinedMessage, PlayerQuitMessage};
use crate::player::gamemode::Gamemode;
use crate::player::identity::PlayerIdentity;
use crate::plugin::PluginHostResource;
use crate::plugin::host_impl::snapshot;
use crate::world::block::{BlockBreakMessage, BlockPlaceMessage};

/// Observed events waiting to be handed out, in the order they happened.
#[derive(Resource, Default)]
pub struct PendingPluginEvents(Vec<Event>);

impl PendingPluginEvents {
    pub fn queue(&mut self, event: Event) {
        self.0.push(event);
    }
}

/// Snapshots this tick's observed events.
///
/// The player is snapshotted here rather than at dispatch time because a leaving player's entity
/// may be gone by then. When the components are already missing, the plugin still gets the name,
/// which is the field a quit handler actually wants.
pub fn collect_observed_events(
    mut joins: MessageReader<PlayerJoinedMessage>,
    mut quits: MessageReader<PlayerQuitMessage>,
    mut breaks: MessageReader<BlockBreakMessage>,
    mut places: MessageReader<BlockPlaceMessage>,
    players: Query<(&PlayerIdentity, &Transform, &Gamemode)>,
    mut pending: ResMut<PendingPluginEvents>,
) {
    let look_up = |entity: Entity, fallback: &str| match players.get(entity) {
        Ok((identity, transform, gamemode)) => snapshot(identity, transform, gamemode),
        Err(_) => name_only(fallback),
    };

    for message in joins.read() {
        pending.0.push(Event::PlayerJoin(PlayerJoinData {
            player: look_up(message.entity, &message.name),
        }));
    }

    for message in quits.read() {
        pending.0.push(Event::PlayerLeave(PlayerLeaveData {
            player: look_up(message.entity, &message.name),
        }));
    }

    for message in breaks.read() {
        pending.0.push(Event::BlockBreak(BlockBreakData {
            player: look_up(message.entity, ""),
            position: block_pos(message.position),
            block_id: message.block_id,
        }));
    }

    for message in places.read() {
        pending.0.push(Event::BlockPlace(BlockPlaceData {
            player: look_up(message.entity, ""),
            position: block_pos(message.position),
            block_id: message.block_id,
            face: message.face,
        }));
    }
}

/// Hands every queued event to every running plugin, in load order.
///
/// Observed events ignore what the plugin returns, but a mismatched variant case is still worth a
/// log line: it means the plugin's handler is confused about which event it got.
pub fn dispatch_observed_events(world: &mut World) {
    let events = std::mem::take(&mut world.resource_mut::<PendingPluginEvents>().0);
    if events.is_empty() {
        return;
    }

    for event in events {
        let id = EventId::of(&event);
        dispatch(world, id, event, |_, _| {});
    }
}

/// Runs one event past every running plugin, letting `apply` read each returned event.
///
/// `apply` is how a blocking event gets its changes back. For observed events it does nothing.
pub(crate) fn dispatch(world: &mut World, id: EventId, mut event: Event, mut apply: impl FnMut(&mut Event, &Event)) {
    if !world.contains_resource::<PluginHostResource>() {
        return;
    }

    world.resource_scope(|world, host: Mut<PluginHostResource>| {
        let mut host = host.host();
        for name in host.running_names() {
            let erased = world as *mut World as *mut ();
            let returned = host.call(&name, erased, |store, bindings| bindings.call_handle_event(store, id.id(), &event));
            let Some(returned) = returned else { continue };

            if !id.matches(&returned) {
                warn!(plugin = %name, "handler returned the wrong event case, keeping the original");
                continue;
            }

            apply(&mut event, &returned);
        }
    });
}

/// Runs one chat message past every plugin.
///
/// `None` means there was nobody to ask, which is how `chat.rs` tells "no plugins" apart from "the
/// plugins looked at it and left it alone". `Some((text, cancelled))` is what the last plugin left.
pub fn moderate(world: &mut World, message: &PlayerChatMessage) -> Option<(String, bool)> {
    let any = world.get_resource::<PluginHostResource>().is_some_and(|host| !host.host().running_names().is_empty());
    if !any {
        return None;
    }

    let player = player_for(world, message.entity, &message.sender_name, &message.sender_xuid);
    let event = Event::PlayerChat(PlayerChatData {
        player,
        message: message.message.clone(),
        cancelled: false,
    });

    let mut outcome = (message.message.clone(), false);
    dispatch(world, EventId::PlayerChat, event, |current, returned| {
        let (Event::PlayerChat(current), Event::PlayerChat(returned)) = (current, returned) else { return };
        // each plugin sees what the last one left, so the running event is updated too
        current.message = returned.message.clone();
        current.cancelled = returned.cancelled;
        outcome = (returned.message.clone(), returned.cancelled);
    });

    Some(outcome)
}

fn player_for(world: &mut World, entity: Entity, name: &str, xuid: &str) -> WitPlayer {
    let mut query = world.query::<(&PlayerIdentity, &Transform, &Gamemode)>();
    match query.get(world, entity) {
        Ok((identity, transform, gamemode)) => snapshot(identity, transform, gamemode),
        Err(_) => name_only_with_xuid(name, xuid),
    }
}

fn name_only_with_xuid(name: &str, xuid: &str) -> WitPlayer {
    WitPlayer {
        xuid: xuid.to_owned(),
        name: name.to_owned(),
        position: Position { x: 0.0, y: 0.0, z: 0.0 },
        game_mode: GameMode::Survival,
    }
}

/// A player whose components are already gone. Only the name is real.
fn name_only(name: &str) -> WitPlayer {
    WitPlayer {
        xuid: String::new(),
        name: name.to_owned(),
        position: Position { x: 0.0, y: 0.0, z: 0.0 },
        game_mode: GameMode::Survival,
    }
}

fn block_pos(position: IVec3) -> BlockPos {
    BlockPos {
        x: position.x,
        y: position.y,
        z: position.z,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_queue_with_nothing_in_it_does_not_need_a_host() {
        let mut world = World::new();
        world.init_resource::<PendingPluginEvents>();

        dispatch_observed_events(&mut world);

        assert!(world.resource::<PendingPluginEvents>().0.is_empty());
    }
}
