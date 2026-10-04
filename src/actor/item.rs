use crate::actor::physics::Physics;
use crate::actor::viewers::{ActorShown, Despawn, NetworkOffset, Viewers, send_to_viewers};
use crate::entity::components::actor_id::ActorId;
use crate::entity::components::transform::Transform;
use crate::item::ItemTakenMessage;
use crate::item::item_entity::{ItemEntity, within_pickup_reach};
use crate::level::DimensionId;
use crate::network::BedrockProtocol;
use crate::network::handler::inventory::{picked_item, send_content};
use crate::network::session::Session;
use crate::network::session::state::SessionState;
use crate::player::PLAYER_EYE_HEIGHT;
use crate::player::chunk_view::ChunkView;
use crate::player::gamemode::Gamemode;
use crate::player::inventory::{MAX_STACK_SIZE, PlayerInventory};
use crate::registry::block_registry::BlockRegistry;
use crate::registry::item_registry::ItemRegistry;
use crate::server::ServerState;
use crate::world::block::BlockBreakMessage;
use bedrock::protocol::ProtoVersionPackets;
use bedrock::protocol::v662::enums::ContainerID;
use bedrock::protocol::v662::packets::{AddItemActorPacket, TakeItemActorPacket};
use bedrock::protocol::v662::types::{ActorRuntimeID, ActorUniqueID};
use bedrock::protocol::v975::enums::ActorEvent;
use bevy_ecs::message::{MessageReader, MessageWriter};
use bevy_ecs::prelude::{Commands, Entity, Local, Query, Res, ResMut, Without};
use glam::Vec3;
use std::collections::HashSet;

type ActorEventPacket = <BedrockProtocol as ProtoVersionPackets>::ActorEventPacket;

const MERGE_INTERVAL_TICKS: u32 = 10;
/// How far apart two item boxes may be horizontally and still merge, like vanilla.
const MERGE_REACH: f32 = 0.5;

/// Spawns a dropped item with vanilla's small random offset and pop.
pub fn spawn_item(commands: &mut Commands, server_state: &mut ServerState, item: ItemEntity, center: Vec3, dimension: i32) -> Entity {
    let physics = Physics::item();
    let runtime_id = server_state.get_runtime_id();
    let jitter = || rand::random::<f32>() * 0.5 - 0.25;
    let position = center + Vec3::new(jitter(), jitter() - physics.height / 2.0, jitter());
    let velocity = Vec3::new(rand::random::<f32>() * 0.2 - 0.1, 0.2, rand::random::<f32>() * 0.2 - 0.1);
    commands
        .spawn((
            item,
            ActorId {
                unique_id: runtime_id as i64,
                runtime_id,
            },
            Transform {
                position,
                velocity,
                ..Transform::default()
            },
            DimensionId(dimension),
            physics,
            NetworkOffset(physics.height / 2.0),
            Viewers::default(),
        ))
        .id()
}

pub fn spawn_block_drops(
    mut reader: MessageReader<BlockBreakMessage>,
    players: Query<(&Gamemode, &ChunkView)>,
    blocks: Res<BlockRegistry>,
    items: Res<ItemRegistry>,
    mut server_state: ResMut<ServerState>,
    mut commands: Commands,
) {
    for msg in reader.read() {
        let Ok((gamemode, view)) = players.get(msg.entity) else { continue };
        if *gamemode == Gamemode::Creative {
            continue;
        }
        if let Some(stack) = picked_item(&blocks, &items, msg.block_id) {
            spawn_item(&mut commands, &mut server_state, ItemEntity::new(stack), msg.position.as_vec3() + Vec3::splat(0.5), view.dimension);
        }
    }
}

/// Sends the spawn packet of dropped items to players that just started seeing them.
pub fn send_item_spawns(mut reader: MessageReader<ActorShown>, items: Query<(&ItemEntity, &ActorId, &Transform, &NetworkOffset)>, mut sessions: Query<&mut Session>) {
    for shown in reader.read() {
        let (Ok((item, actor, transform, offset)), Ok(mut session)) = (items.get(shown.actor), sessions.get_mut(shown.viewer)) else {
            continue;
        };
        let position = transform.position;
        session.send(BedrockProtocol::AddItemActorPacket(
            AddItemActorPacket {
                target_actor_id: ActorUniqueID(actor.unique_id),
                target_runtime_id: ActorRuntimeID(actor.runtime_id),
                item: item.stack().to_actor_descriptor(),
                position: (position.x, position.y + offset.0, position.z),
                velocity: (transform.velocity.x, transform.velocity.y, transform.velocity.z),
                entity_data: vec![],
                from_fishing: false,
            }
            .into(),
        ));
    }
}

/// Counts down pickup delays and despawns items after five minutes.
pub fn tick_item_entities(mut items: Query<(Entity, &mut ItemEntity), Without<Despawn>>, mut commands: Commands) {
    for (entity, mut item) in &mut items {
        item.tick_pickup_delay();
        if item.tick_age() {
            commands.entity(entity).insert(Despawn);
        }
    }
}

type MergeItem<'a> = (Entity, &'a mut ItemEntity, &'a ActorId, &'a Transform, &'a DimensionId, &'a Viewers);

/// Merges nearby stacks of the same item into the larger one.
pub fn merge_item_entities(mut items: Query<MergeItem, Without<Despawn>>, mut sessions: Query<&mut Session>, mut commands: Commands, mut ticks: Local<u32>) {
    *ticks += 1;
    if *ticks < MERGE_INTERVAL_TICKS {
        return;
    }
    *ticks = 0;

    let (size, reach) = (Physics::item().height, Physics::item().width + MERGE_REACH);
    let mut removed = HashSet::new();
    let mut pairs = items.iter_combinations_mut();
    while let Some([first, second]) = pairs.fetch_next() {
        let (
            (first_entity, mut first_item, first_actor, first_transform, first_dimension, first_viewers),
            (second_entity, mut second_item, second_actor, second_transform, second_dimension, second_viewers),
        ) = (first, second);
        if first_dimension.0 != second_dimension.0 || removed.contains(&first_entity) || removed.contains(&second_entity) {
            continue;
        }
        let (a, b) = (first_transform.position, second_transform.position);
        if (a.x - b.x).abs() > reach || (a.z - b.z).abs() > reach || (a.y - b.y).abs() >= size || !first_item.stack().is_same(&second_item.stack()) {
            continue;
        }

        let first_is_target = first_item.stack().count >= second_item.stack().count;
        let (target, source, target_actor, source_actor, source_entity, target_viewers, source_viewers) = if first_is_target {
            (&mut *first_item, &mut *second_item, first_actor, second_actor, second_entity, first_viewers, second_viewers)
        } else {
            (&mut *second_item, &mut *first_item, second_actor, first_actor, first_entity, second_viewers, first_viewers)
        };
        if target.absorb(source, MAX_STACK_SIZE) == 0 {
            continue;
        }
        send_to_viewers(target_viewers, &mut sessions, &stack_size_packet(target_actor, target.stack().count));
        if source.stack().count == 0 {
            removed.insert(source_entity);
            commands.entity(source_entity).insert(Despawn);
        } else {
            send_to_viewers(source_viewers, &mut sessions, &stack_size_packet(source_actor, source.stack().count));
        }
    }
}

fn stack_size_packet(actor: &ActorId, count: u16) -> BedrockProtocol {
    BedrockProtocol::ActorEventPacket(
        ActorEventPacket {
            target_runtime_id: ActorRuntimeID(actor.runtime_id),
            event_id: ActorEvent::UpdateStackSize,
            data: count as i32,
            fire_at_position: None,
        }
        .into(),
    )
}

pub fn handle_item_pickup(
    mut players: Query<(Entity, &mut PlayerInventory, &ActorId, &Transform, &ChunkView)>,
    items: Query<(Entity, &ItemEntity, &ActorId, &Transform, &DimensionId, &Viewers), Without<Despawn>>,
    mut sessions: Query<&mut Session>,
    mut writer: MessageWriter<ItemTakenMessage>,
    mut commands: Commands,
) {
    let mut claimed = HashSet::new();

    for (player, mut inventory, player_actor, player_transform, view) in &mut players {
        if sessions.get(player).is_ok_and(|session| session.get_state() != SessionState::Play) {
            continue;
        }

        let feet = player_transform.position - Vec3::new(0.0, PLAYER_EYE_HEIGHT, 0.0);
        let mut changed = false;

        for (entity, item, item_actor, item_transform, dimension, viewers) in &items {
            if claimed.contains(&entity) || dimension.0 != view.dimension || !item.can_be_picked_up() || !within_pickup_reach(item_transform.position, feet) {
                continue;
            }
            if !inventory.main_mut().add(item.stack()) {
                continue;
            }

            claimed.insert(entity);
            changed = true;
            let take = BedrockProtocol::TakeItemActorPacket(
                TakeItemActorPacket {
                    item_runtime_id: ActorRuntimeID(item_actor.runtime_id),
                    actor_runtime_id: ActorRuntimeID(player_actor.runtime_id),
                }
                .into(),
            );
            send_to_viewers(viewers, &mut sessions, &take);
            commands.entity(entity).insert(Despawn);
            writer.write(ItemTakenMessage {
                item_entity: entity,
                dimension_id: dimension.0,
                item_unique_id: item_actor.unique_id,
                item_runtime_id: item_actor.runtime_id,
                player_runtime_id: player_actor.runtime_id,
            });
        }

        if changed && let Ok(mut session) = sessions.get_mut(player) {
            send_content(&mut session, &mut inventory, ContainerID::Inventory);
        }
    }
}
