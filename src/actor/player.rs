use crate::actor::viewers::{ActorShown, Viewers, send_to_viewers};
use crate::command::permission::CommandPermission;
use crate::entity::components::actor_id::ActorId;
use crate::entity::components::transform::Transform;
use crate::network::BedrockProtocol;
use crate::network::handler::play::{PlayerJoinedMessage, PlayerQuitMessage};
use crate::network::session::Session;
use crate::network::session::state::SessionState;
use crate::player::gamemode::Gamemode;
use crate::player::identity::PlayerIdentity;
use crate::player::inventory::PlayerInventory;
use crate::player::skin::PlayerAppearance;
use bedrock::protocol::ProtoCodec;
use bedrock::protocol::v662::enums::BuildPlatform;
use bedrock::protocol::v662::packets::{AddPlayerPacket, RemoveActorPacket};
use bedrock::protocol::v662::types::{ActorRuntimeID, ActorUniqueID, PropertySyncData};
use bedrock::protocol::v671::packets::UpdatePlayerGameTypePacket;
use bedrock::protocol::v776::types::{SerializedAbilitiesData, SerializedAbilitiesLayer, SerializedLayer};
use bedrock::protocol::v800::types::Color;
use bedrock::protocol::v2168::packets::{AddPlayerListEntry, PlayerListEntry, PlayerListPacket};
use bevy_ecs::message::MessageReader;
use bevy_ecs::prelude::{Changed, Commands, Entity, Query, With};

fn list_entry(identity: &PlayerIdentity, actor: &ActorId, appearance: &PlayerAppearance) -> PlayerListEntry<BedrockProtocol> {
    PlayerListEntry::Add(AddPlayerListEntry {
        uuid: appearance.uuid,
        target_actor_id: ActorUniqueID(actor.unique_id),
        player_name: identity.name().to_owned(),
        xbl_xuid: identity.xuid().to_owned(),
        platform_chat_id: String::new(),
        build_platform: BuildPlatform::Unknown(-1),
        serialized_skin: appearance.skin.clone(),
        is_teacher: false,
        is_host: false,
        is_sub_client: false,
        color: Color::deserialize(&mut [0u8; 4].as_slice()).expect("four bytes make a colour"),
    })
}

fn list_packet(entries: Vec<PlayerListEntry<BedrockProtocol>>) -> BedrockProtocol {
    BedrockProtocol::PlayerListPacket(PlayerListPacket { entries }.into())
}

pub fn handle_player_joins(
    mut joins: MessageReader<PlayerJoinedMessage>,
    players: Query<(Entity, &PlayerIdentity, &ActorId, &PlayerAppearance)>,
    mut sessions: Query<&mut Session>,
    mut commands: Commands,
) {
    for join in joins.read() {
        let Ok((_, identity, actor, appearance)) = players.get(join.entity) else { continue };
        commands.entity(join.entity).insert(Viewers::default());
        let joined = list_packet(vec![list_entry(identity, actor, appearance)]);
        for (other, ..) in &players {
            if other != join.entity
                && let Ok(mut session) = sessions.get_mut(other)
                && session.get_state() == SessionState::Play
            {
                session.send(joined.clone());
            }
        }
        let others = players
            .iter()
            .filter(|(other, ..)| *other != join.entity)
            .map(|(_, identity, actor, appearance)| list_entry(identity, actor, appearance))
            .collect();
        if let Ok(mut session) = sessions.get_mut(join.entity) {
            session.send(list_packet(others));
        }
    }
}

pub fn send_player_spawns(
    mut reader: MessageReader<ActorShown>,
    players: Query<(&PlayerIdentity, &ActorId, &Transform, &PlayerAppearance, &Gamemode, &PlayerInventory, Option<&CommandPermission>)>,
    mut sessions: Query<&mut Session>,
) {
    for shown in reader.read() {
        let (Ok((identity, actor, transform, appearance, gamemode, inventory, permission)), Ok(mut session)) = (players.get(shown.actor), sessions.get_mut(shown.viewer)) else {
            continue;
        };
        let (position, rotation) = (transform.position, transform.rotation);
        session.send(BedrockProtocol::AddPlayerPacket(
            AddPlayerPacket {
                uuid: appearance.uuid,
                player_name: identity.name().to_owned(),
                target_runtime_id: ActorRuntimeID(actor.runtime_id),
                platform_chat_id: String::new(),
                position: (position.x, position.y, position.z),
                velocity: (transform.velocity.x, transform.velocity.y, transform.velocity.z),
                rotation: (rotation.x, rotation.y),
                y_head_rotation: rotation.y,
                carried_item: inventory.held_item().to_actor_descriptor(),
                player_game_type: gamemode.game_type(),
                entity_data: vec![],
                synced_properties: PropertySyncData {
                    int_entries_list: vec![],
                    float_entries_list: vec![],
                },
                abilities_data: SerializedAbilitiesData {
                    target_player_raw_id: actor.unique_id,
                    player_permissions: 1,
                    command_permissions: permission.map_or(Default::default(), |permission| permission.0).into(),
                    layers: vec![SerializedLayer {
                        serialized_layer: SerializedAbilitiesLayer::Base,
                        abilities_set: 0,
                        ability_values: 0,
                        fly_speed: 0.05,
                        vertical_fly_speed: 1.0,
                        walk_speed: 0.1,
                    }],
                },
                actor_links: vec![],
                device_id: appearance.device_id.clone(),
                build_platform: BuildPlatform::Unknown(-1),
            }
            .into(),
        ));
    }
}

/// Tells everyone still watching a player that its game mode changed. Viewers who should no longer
/// see it are already gone by now, `update_viewers` takes care of that.
pub fn broadcast_gamemode_changes(changed: Query<(&ActorId, &Gamemode, &Viewers), Changed<Gamemode>>, mut sessions: Query<&mut Session>) {
    for (actor, gamemode, viewers) in &changed {
        let packet = BedrockProtocol::UpdatePlayerGameTypePacket(
            UpdatePlayerGameTypePacket {
                player_game_type: gamemode.game_type(),
                target_player: ActorUniqueID(actor.unique_id),
                tick: 0,
            }
            .into(),
        );
        send_to_viewers(viewers, &mut sessions, &packet);
    }
}

pub fn handle_player_quits(
    mut quits: MessageReader<PlayerQuitMessage>,
    leaving: Query<(&ActorId, &PlayerAppearance, Option<&Viewers>)>,
    players: Query<Entity, With<PlayerIdentity>>,
    mut sessions: Query<&mut Session>,
) {
    for quit in quits.read() {
        let Ok((actor, appearance, viewers)) = leaving.get(quit.entity) else { continue };
        if let Some(viewers) = viewers {
            let remove = BedrockProtocol::RemoveActorPacket(
                RemoveActorPacket {
                    target_actor_id: ActorUniqueID(actor.unique_id),
                }
                .into(),
            );
            send_to_viewers(viewers, &mut sessions, &remove);
        }
        let unlisted = list_packet(vec![PlayerListEntry::Remove { uuid: appearance.uuid }]);
        for player in &players {
            if player != quit.entity
                && let Ok(mut session) = sessions.get_mut(player)
                && session.get_state() == SessionState::Play
            {
                session.send(unlisted.clone());
            }
        }
    }
}
