use crate::entity::components::actor_id::ActorId;
use crate::entity::components::transform::Transform;
use crate::item::ItemTakenMessage;
use crate::item::item_entity::{ItemEntity, within_pickup_reach};
use crate::level::DimensionId;
use crate::network::BedrockProtocol;
use crate::network::handler::block::BlockBreakMessage;
use crate::network::handler::inventory::{picked_item, send_content};
use crate::network::session::Session;
use crate::network::session::state::SessionState;
use crate::player::PLAYER_EYE_HEIGHT;
use crate::player::chunk_view::ChunkView;
use crate::player::gamemode::Gamemode;
use crate::player::inventory::PlayerInventory;
use crate::registry::block_registry::BlockRegistry;
use crate::registry::item_registry::ItemRegistry;
use crate::server::ServerState;
use bedrock::protocol::v662::enums::ContainerID;
use bedrock::protocol::v662::packets::{AddItemActorPacket, RemoveActorPacket, TakeItemActorPacket};
use bedrock::protocol::v662::types::{ActorRuntimeID, ActorUniqueID};
use bevy_ecs::message::{MessageReader, MessageWriter};
use bevy_ecs::prelude::{Added, Commands, Entity, Query, Res, ResMut};
use glam::Vec3;
use std::collections::HashSet;

pub fn spawn_block_drops(
    mut reader: MessageReader<BlockBreakMessage>,
    players: Query<(&Gamemode, &ChunkView)>,
    blocks: Res<BlockRegistry>,
    items: Res<ItemRegistry>,
    mut server_state: ResMut<ServerState>,
    mut commands: Commands,
) {
    for msg in reader.read() {
        let Ok((gamemode, view)) = players.get(msg.entity) else {
            continue;
        };
        if *gamemode == Gamemode::Creative {
            continue;
        }

        let Some(stack) = picked_item(&blocks, &items, msg.block_id) else {
            continue;
        };

        let runtime_id = server_state.get_runtime_id();
        commands.spawn((
            ItemEntity::new(stack),
            ActorId {
                unique_id: runtime_id as i64,
                runtime_id,
            },
            Transform::at(msg.position.as_vec3() + Vec3::splat(0.5)),
            DimensionId(view.dimension),
        ));
    }
}

pub fn broadcast_spawned_items(new_items: Query<(&ItemEntity, &ActorId, &Transform, &DimensionId), Added<ItemEntity>>, mut sessions: Query<(&mut Session, &ChunkView)>) {
    for (item, actor, transform, dimension) in &new_items {
        let stack = item.stack();
        let position = transform.position;

        for (mut session, view) in &mut sessions {
            if session.get_state() != SessionState::Play || view.dimension != dimension.0 {
                continue;
            }

            session.send(BedrockProtocol::AddItemActorPacket(
                AddItemActorPacket {
                    target_actor_id: ActorUniqueID(actor.unique_id),
                    target_runtime_id: ActorRuntimeID(actor.runtime_id),
                    item: stack.to_actor_descriptor(),
                    position: (position.x, position.y, position.z),
                    velocity: (0.0, 0.0, 0.0),
                    entity_data: vec![],
                    from_fishing: false,
                }
                .into(),
            ));
        }
    }
}

pub fn tick_item_entities(mut items: Query<&mut ItemEntity>) {
    for mut item in &mut items {
        item.tick_pickup_delay();
    }
}

pub fn handle_item_pickup(
    mut players: Query<(&mut Session, &mut PlayerInventory, &ActorId, &Transform, &ChunkView)>,
    items: Query<(Entity, &ItemEntity, &ActorId, &Transform, &DimensionId)>,
    mut writer: MessageWriter<ItemTakenMessage>,
) {
    let mut claimed = HashSet::new();

    for (mut session, mut inventory, player_actor, player_transform, view) in &mut players {
        if session.get_state() != SessionState::Play {
            continue;
        }

        let feet = player_transform.position - Vec3::new(0.0, PLAYER_EYE_HEIGHT, 0.0);
        let mut changed = false;

        for (entity, item, item_actor, item_transform, dimension) in &items {
            if claimed.contains(&entity) || dimension.0 != view.dimension || !item.can_be_picked_up() || !within_pickup_reach(item_transform.position, feet) {
                continue;
            }
            if !inventory.main_mut().add(item.stack()) {
                continue;
            }

            claimed.insert(entity);
            changed = true;

            writer.write(ItemTakenMessage {
                item_entity: entity,
                dimension_id: dimension.0,
                item_unique_id: item_actor.unique_id,
                item_runtime_id: item_actor.runtime_id,
                player_runtime_id: player_actor.runtime_id,
            });
        }

        if changed {
            send_content(&mut session, &mut inventory, ContainerID::Inventory);
        }
    }
}

pub fn broadcast_taken_items(mut reader: MessageReader<ItemTakenMessage>, mut sessions: Query<(&mut Session, &ChunkView)>, mut commands: Commands) {
    for msg in reader.read() {
        for (mut session, view) in &mut sessions {
            if session.get_state() != SessionState::Play || view.dimension != msg.dimension_id {
                continue;
            }

            session.send(BedrockProtocol::TakeItemActorPacket(
                TakeItemActorPacket {
                    item_runtime_id: ActorRuntimeID(msg.item_runtime_id),
                    actor_runtime_id: ActorRuntimeID(msg.player_runtime_id),
                }
                .into(),
            ));
            session.send(BedrockProtocol::RemoveActorPacket(
                RemoveActorPacket {
                    target_actor_id: ActorUniqueID(msg.item_unique_id),
                }
                .into(),
            ));
        }

        commands.entity(msg.item_entity).despawn();
    }
}
