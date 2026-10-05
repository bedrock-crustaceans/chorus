use crate::entity::components::actor_id::ActorId;
use crate::entity::components::transform::Transform;
use crate::level::DimensionId;
use crate::network::BedrockProtocol;
use crate::network::session::Session;
use crate::network::session::state::SessionState;
use crate::player::chunk_view::ChunkView;
use crate::player::gamemode::Gamemode;
use bedrock::protocol::v662::packets::{MoveActorAbsolutePacket, RemoveActorPacket};
use bedrock::protocol::v662::types::{ActorRuntimeID, ActorUniqueID, MoveActorAbsoluteData};
use bevy_ecs::message::{Message, MessageWriter};
use bevy_ecs::prelude::{Changed, Commands, Component, Entity, Query, With, Without};
use std::collections::HashSet;

#[derive(Component, Default, Debug)]
pub struct Viewers(pub HashSet<Entity>);

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct NetworkOffset(pub f32);

#[derive(Message, Clone, Copy, Debug)]
pub struct ActorShown {
    pub actor: Entity,
    pub viewer: Entity,
}

#[derive(Component, Debug)]
pub struct Despawn;

pub fn send_to_viewers(viewers: &Viewers, sessions: &mut Query<&mut Session>, packet: &BedrockProtocol) {
    for &viewer in &viewers.0 {
        if let Ok(mut session) = sessions.get_mut(viewer) {
            session.send(packet.clone());
        }
    }
}

fn remove_packet(actor: &ActorId) -> BedrockProtocol {
    BedrockProtocol::RemoveActorPacket(
        RemoveActorPacket {
            target_actor_id: ActorUniqueID(actor.unique_id),
        }
        .into(),
    )
}

fn angle(degrees: f32) -> i8 {
    (degrees.rem_euclid(360.0) / 360.0 * 256.0) as i32 as i8
}

struct Watcher {
    entity: Entity,
    dimension: i32,
    center: (i32, i32),
    radius: i32,
    // only spectators get to see other spectators
    sees_hidden: bool,
}

pub fn update_viewers(
    mut actors: Query<(Entity, &Transform, &DimensionId, &ActorId, &mut Viewers), Without<Despawn>>,
    mut sessions: Query<&mut Session>,
    views: Query<(Entity, &ChunkView, &Gamemode)>,
    mut shown: MessageWriter<ActorShown>,
) {
    let watchers: Vec<Watcher> = views
        .iter()
        .filter(|(entity, ..)| sessions.get(*entity).is_ok_and(|session| session.get_state() == SessionState::Play))
        .filter_map(|(entity, view, gamemode)| {
            Some(Watcher {
                entity,
                dimension: view.dimension,
                center: view.center?,
                radius: view.radius,
                sees_hidden: !gamemode.visible(),
            })
        })
        .collect();

    for (actor_entity, transform, dimension, actor, mut viewers) in &mut actors {
        let chunk = ((transform.position.x.floor() as i32) >> 4, (transform.position.z.floor() as i32) >> 4);
        let visible = views.get(actor_entity).map_or(true, |(.., gamemode)| gamemode.visible());
        let wanted: HashSet<Entity> = watchers
            .iter()
            .filter(|watcher| {
                watcher.entity != actor_entity
                    && (visible || watcher.sees_hidden)
                    && watcher.dimension == dimension.0
                    && (chunk.0 - watcher.center.0).pow(2) + (chunk.1 - watcher.center.1).pow(2) <= watcher.radius * watcher.radius
            })
            .map(|watcher| watcher.entity)
            .collect();
        if wanted == viewers.0 {
            continue;
        }
        for &left in viewers.0.difference(&wanted) {
            if let Ok(mut session) = sessions.get_mut(left) {
                session.send(remove_packet(actor));
            }
        }
        for &viewer in wanted.difference(&viewers.0) {
            shown.write(ActorShown { actor: actor_entity, viewer });
        }
        viewers.0 = wanted;
    }
}

pub fn broadcast_movement(moved: Query<(&ActorId, &Transform, &Viewers, Option<&NetworkOffset>, Option<&crate::actor::physics::Physics>), Changed<Transform>>, mut sessions: Query<&mut Session>) {
    for (actor, transform, viewers, offset, physics) in &moved {
        if viewers.0.is_empty() {
            continue;
        }
        let position = transform.position;
        let packet = BedrockProtocol::MoveActorAbsolutePacket(
            MoveActorAbsolutePacket {
                move_data: MoveActorAbsoluteData {
                    actor_runtime_id: ActorRuntimeID(actor.runtime_id),
                    header: physics.is_some_and(|physics| physics.on_ground) as i8,
                    position: (position.x, position.y + offset.map_or(0.0, |offset| offset.0), position.z),
                    rotation_x: angle(transform.rotation.x),
                    rotation_y: angle(transform.rotation.y),
                    rotation_y_head: angle(transform.rotation.y),
                },
            }
            .into(),
        );
        send_to_viewers(viewers, &mut sessions, &packet);
    }
}

pub fn despawn_actors(despawning: Query<(Entity, &ActorId, Option<&Viewers>), With<Despawn>>, mut sessions: Query<&mut Session>, mut commands: Commands) {
    for (entity, actor, viewers) in &despawning {
        if let Some(viewers) = viewers {
            send_to_viewers(viewers, &mut sessions, &remove_packet(actor));
        }
        commands.entity(entity).despawn();
    }
}
