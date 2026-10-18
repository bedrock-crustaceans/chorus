//! The host side of the plugin imports, implemented against the live Bevy `World`.

use bedrock::protocol::ProtoVersionPackets;
use bedrock::protocol::v924::enums::TextPacketType;
use bevy_ecs::prelude::World;
use chorus_entity::components::transform::Transform;
use chorus_plugin::bridge::HostBridge;
use chorus_plugin::chorus::plugin::command::Definition;
use chorus_plugin::chorus::plugin::common::{GameMode as WitGameMode, LogLevel, Position};
use chorus_plugin::chorus::plugin::player::Player as WitPlayer;
use chorus_plugin::state::WorldAccessError;
use tracing::{debug, error, info, trace, warn};

use crate::network::BedrockProtocol;
use crate::network::session::Session;
use crate::player::gamemode::Gamemode;
use crate::player::identity::PlayerIdentity;

type TextPacket = <BedrockProtocol as ProtoVersionPackets>::TextPacket;

/// Turns the erased pointer a dispatch installed back into the `World`.
///
/// This is the only cast in the codebase, on purpose. `chorus_plugin` carries the pointer because
/// it must not know about Bevy; this function is where it becomes a world again. It is sound only
/// because `PluginHost::call` sets the pointer immediately before entering guest code and clears
/// it immediately after, and plugin calls are synchronous, so no second `&mut World` can exist
/// while guest code runs.
pub fn with_world<R>(world: *mut (), f: impl FnOnce(&mut World) -> R) -> Result<R, WorldAccessError> {
    if world.is_null() {
        return Err(WorldAccessError);
    }

    // SAFETY: the pointer came from `&mut World` in the dispatch that is still on the stack, and
    // nothing else holds a reference to that world while guest code is running.
    let world = unsafe { &mut *world.cast::<World>() };
    Ok(f(world))
}

/// Implements every plugin import against this server.
pub struct ChorusBridge;

impl HostBridge for ChorusBridge {
    fn log(&self, _world: *mut (), plugin: &str, level: LogLevel, message: &str) {
        // the world is not needed here, so a log line still works outside a dispatch
        match level {
            LogLevel::Trace => trace!(plugin, "{message}"),
            LogLevel::Debug => debug!(plugin, "{message}"),
            LogLevel::Info => info!(plugin, "{message}"),
            LogLevel::Warn => warn!(plugin, "{message}"),
            LogLevel::Error => error!(plugin, "{message}"),
        }
    }

    fn player_by_name(&self, world: *mut (), name: &str) -> Option<WitPlayer> {
        with_world(world, |world| {
            let mut query = world.query::<(&PlayerIdentity, &Transform, &Gamemode)>();
            query
                .iter(world)
                .find(|(identity, _, _)| identity.name().eq_ignore_ascii_case(name))
                .map(|(identity, transform, gamemode)| snapshot(identity, transform, gamemode))
        })
        .ok()
        .flatten()
    }

    fn all_players(&self, world: *mut ()) -> Vec<WitPlayer> {
        with_world(world, |world| {
            let mut query = world.query::<(&PlayerIdentity, &Transform, &Gamemode)>();
            query.iter(world).map(|(identity, transform, gamemode)| snapshot(identity, transform, gamemode)).collect()
        })
        .unwrap_or_default()
    }

    fn send_message(&self, world: *mut (), player: &str, message: &str) -> Result<(), String> {
        with_world(world, |world| {
            let mut query = world.query::<(&PlayerIdentity, &mut Session)>();
            for (identity, mut session) in query.iter_mut(world) {
                if identity.name().eq_ignore_ascii_case(player) {
                    session.send(system_message(message));
                    return Ok(());
                }
            }
            Err(format!("no player named {player} is in game"))
        })
        .unwrap_or_else(|err| Err(err.to_string()))
    }

    fn broadcast(&self, world: *mut (), message: &str) {
        let _ = with_world(world, |world| {
            let mut query = world.query::<&mut Session>();
            for mut session in query.iter_mut(world) {
                session.send(system_message(message));
            }
        });
    }

    fn register_command(&self, world: *mut (), plugin: &str, definition: Definition) -> Result<u32, String> {
        with_world(world, |world| crate::plugin::command::register_plugin_command(world, plugin, definition)).unwrap_or_else(|err| Err(err.to_string()))
    }
}

/// One player, as a plugin sees them. A snapshot, not a handle.
pub fn snapshot(identity: &PlayerIdentity, transform: &Transform, gamemode: &Gamemode) -> WitPlayer {
    WitPlayer {
        xuid: identity.xuid().to_owned(),
        name: identity.name().to_owned(),
        position: Position {
            x: f64::from(transform.position.x),
            y: f64::from(transform.position.y),
            z: f64::from(transform.position.z),
        },
        game_mode: match gamemode {
            Gamemode::Survival => WitGameMode::Survival,
            Gamemode::Creative => WitGameMode::Creative,
            Gamemode::Adventure => WitGameMode::Adventure,
            Gamemode::Spectator => WitGameMode::Spectator,
        },
    }
}

/// The same raw system message `CommandContext::reply` sends, so plugin text looks like command
/// output rather than chat.
fn system_message(message: &str) -> BedrockProtocol {
    BedrockProtocol::TextPacket(
        TextPacket {
            localize: false,
            message_type: TextPacketType::SystemMessage(message.to_owned()),
            sender_xuid: String::new(),
            platform_id: String::new(),
            filtered_message: None,
        }
        .into(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chorus_entity::components::transform::Transform;
    use glam::Vec3;

    use crate::player::gamemode::Gamemode;
    use crate::player::identity::PlayerIdentity;

    fn world_with_steve() -> World {
        let mut world = World::new();
        world.spawn((
            PlayerIdentity::new(String::from("Steve"), String::from("2535400000000000")),
            Transform::at(Vec3::new(1.0, 2.0, 3.0)),
            Gamemode::Creative,
        ));
        world
    }

    fn erased(world: &mut World) -> *mut () {
        world as *mut World as *mut ()
    }

    #[test]
    fn a_null_world_pointer_is_reported() {
        assert!(with_world(std::ptr::null_mut(), |_| ()).is_err());
    }

    #[test]
    fn reads_a_player_by_name() {
        let mut world = world_with_steve();

        let player = ChorusBridge.player_by_name(erased(&mut world), "Steve").expect("Steve is in the world");

        assert_eq!(player.name, "Steve");
        assert_eq!(player.xuid, "2535400000000000");
        assert_eq!((player.position.x, player.position.y, player.position.z), (1.0, 2.0, 3.0));
        assert!(matches!(player.game_mode, WitGameMode::Creative));
    }

    #[test]
    fn a_name_lookup_ignores_case() {
        let mut world = world_with_steve();

        assert!(ChorusBridge.player_by_name(erased(&mut world), "steve").is_some());
    }

    #[test]
    fn an_unknown_name_is_none() {
        let mut world = world_with_steve();

        assert!(ChorusBridge.player_by_name(erased(&mut world), "Alex").is_none());
    }

    #[test]
    fn lists_every_player() {
        let mut world = world_with_steve();
        world.spawn((PlayerIdentity::new(String::from("Alex"), String::from("1")), Transform::default(), Gamemode::Survival));

        let mut names: Vec<String> = ChorusBridge.all_players(erased(&mut world)).into_iter().map(|player| player.name).collect();
        names.sort();

        assert_eq!(names, vec![String::from("Alex"), String::from("Steve")]);
    }

    #[test]
    fn sending_to_an_unknown_player_is_an_error() {
        let mut world = world_with_steve();

        assert!(ChorusBridge.send_message(erased(&mut world), "Alex", "hi").is_err());
    }

    #[test]
    fn a_player_with_no_session_cannot_be_messaged() {
        let mut world = world_with_steve();

        assert!(ChorusBridge.send_message(erased(&mut world), "Steve", "hi").is_err());
    }
}
