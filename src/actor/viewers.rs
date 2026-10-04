use crate::entity::components::actor_id::ActorId;
use crate::entity::components::transform::Transform;
use crate::level::DimensionId;
use crate::network::BedrockProtocol;
use crate::network::session::Session;
use crate::network::session::state::SessionState;
use crate::player::chunk_view::ChunkView;
use bedrock::protocol::v662::packets::{MoveActorAbsolutePacket, RemoveActorPacket};
use bedrock::protocol::v662::types::{ActorRuntimeID, ActorUniqueID, MoveActorAbsoluteData};
use bevy_ecs::message::{Message, MessageWriter};
use bevy_ecs::prelude::{Changed, Commands, Component, Entity, Query, With, Without};
use std::collections::HashSet;

/// The players that currently have this entity spawned on their client. Entities with this component
/// are shown to players whose view reaches their chunk and hidden again when it stops reaching.
#[derive(Component, Default, Debug)]
pub struct Viewers(pub HashSet<Entity>);

/// How far above an entity's position the client expects it, for entities whose position is their
/// bottom but whose network position is their centre.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct NetworkOffset(pub f32);

/// An entity was just shown to a player; the system for that entity kind sends its spawn packet.
#[derive(Message, Clone, Copy, Debug)]
pub struct ActorShown {
    pub actor: Entity,
    pub viewer: Entity,
}

/// Marks an entity to be removed from its viewers' clients and despawned at the end of the tick.
#[derive(Component, Debug)]
pub struct Despawn;

/// Sends a packet to every player viewing an entity.
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

/// Works out which players should see each entity, showing and hiding it as players move, join,
/// leave or change dimension.
pub fn update_viewers(
    mut actors: Query<(Entity, &Transform, &DimensionId, &ActorId, &mut Viewers), Without<Despawn>>,
    mut sessions: Query<&mut Session>,
    views: Query<(Entity, &ChunkView)>,
    mut shown: MessageWriter<ActorShown>,
) {
    let players: Vec<(Entity, i32, (i32, i32), i32)> = views
        .iter()
        .filter(|(entity, _)| sessions.get(*entity).is_ok_and(|session| session.get_state() == SessionState::Play))
        .filter_map(|(entity, view)| Some((entity, view.dimension, view.center?, view.radius)))
        .collect();

    for (actor_entity, transform, dimension, actor, mut viewers) in &mut actors {
        let chunk = ((transform.position.x.floor() as i32) >> 4, (transform.position.z.floor() as i32) >> 4);
        let wanted: HashSet<Entity> = players
            .iter()
            .filter(|(_, player_dimension, center, radius)| *player_dimension == dimension.0 && (chunk.0 - center.0).pow(2) + (chunk.1 - center.1).pow(2) <= radius * radius)
            .map(|(entity, ..)| *entity)
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

/// Sends the position of every entity that moved this tick to its viewers.
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
                    rotation_x: 0,
                    rotation_y: 0,
                    rotation_y_head: 0,
                },
            }
            .into(),
        );
        send_to_viewers(viewers, &mut sessions, &packet);
    }
}

/// Removes entities marked with [`Despawn`] from their viewers' clients and despawns them.
pub fn despawn_actors(despawning: Query<(Entity, &ActorId, Option<&Viewers>), With<Despawn>>, mut sessions: Query<&mut Session>, mut commands: Commands) {
    for (entity, actor, viewers) in &despawning {
        if let Some(viewers) = viewers {
            send_to_viewers(viewers, &mut sessions, &remove_packet(actor));
        }
        commands.entity(entity).despawn();
    }
}
