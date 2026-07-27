use crate::entity::entity::Entity as PlayerEntity;
use crate::item::ItemTakenMessage;
use crate::item::inventory::Inventory;
use crate::item::item_entity::ItemEntity;
use crate::item::item_stack::ItemStack;
use crate::network::BedrockProtocol;
use crate::network::session::Session;
use crate::player::Player;
use crate::server::ServerState;
use bedrock::protocol::v662::packets::{AddItemActorPacket, RemoveActorPacket, TakeItemActorPacket};
use bedrock::protocol::v662::types::{ActorRuntimeID, ActorUniqueID};
use bedrock::protocol::v729::types::FullContainerName;
use bedrock::protocol::v944::enums::ContainerEnumName;
use bedrock::protocol::v975::types::NetworkItemStackDescriptorV2;
use bedrock::protocol::v1001::packets::InventoryContentPacket;
use bevy_ecs::message::{MessageReader, MessageWriter};
use bevy_ecs::prelude::{Added, Commands, Entity, Query};
use std::collections::HashSet;
use vek::Vec3;

const PLAYER_INVENTORY_ID: u32 = 0;

pub fn spawn_block_drop(commands: &mut Commands, server_state: &mut ServerState, block_id: i32, x: i32, y: i32, z: i32) {
    let runtime_id = server_state.get_runtime_id();
    let position = Vec3::new(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5);
    let stack = ItemStack::new(block_id, 1);
    commands.spawn(ItemEntity::new(runtime_id as i64, runtime_id, position, stack));
}

pub fn broadcast_spawned_items(new_items: Query<&ItemEntity, Added<ItemEntity>>, mut sessions: Query<&mut Session>) {
    for item in &new_items {
        let position = item.position();
        for mut session in &mut sessions {
            session.send(BedrockProtocol::AddItemActorPacket(
                AddItemActorPacket {
                    target_actor_id: ActorUniqueID(item.unique_id()),
                    target_runtime_id: ActorRuntimeID(item.runtime_id()),
                    item: item.stack().to_descriptor(),
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

pub fn handle_item_pickup(mut players: Query<(&PlayerEntity, &Player, &mut Inventory, &mut Session)>, items: Query<(Entity, &ItemEntity)>, mut writer: MessageWriter<ItemTakenMessage>) {
    let mut claimed: HashSet<Entity> = HashSet::new();

    for (player_entity, player, mut inventory, mut session) in &mut players {
        for (item_bevy, item) in &items {
            if claimed.contains(&item_bevy) || !item.can_be_picked_up() || !item.is_within_reach(player_entity.position) {
                continue;
            }
            if !inventory.add(item.stack()) {
                continue;
            }

            claimed.insert(item_bevy);
            writer.write(ItemTakenMessage {
                item_entity: item_bevy,
                item_unique_id: item.unique_id(),
                item_runtime_id: item.runtime_id(),
                player_runtime_id: player.runtime_id(),
            });
            session.send(BedrockProtocol::InventoryContentPacket(inventory_content_packet(&inventory).into()));
        }
    }
}

pub fn broadcast_taken_items(mut commands: Commands, mut reader: MessageReader<ItemTakenMessage>, mut sessions: Query<&mut Session>) {
    for msg in reader.read() {
        for mut session in &mut sessions {
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

fn inventory_content_packet(inventory: &Inventory) -> InventoryContentPacket<BedrockProtocol> {
    let slots = inventory
        .slots()
        .iter()
        .map(|slot| slot.map(|stack| stack.to_descriptor_v2()).unwrap_or_else(empty_descriptor_v2))
        .collect();

    InventoryContentPacket {
        inventory_id: PLAYER_INVENTORY_ID,
        slots,
        container_name_data: FullContainerName {
            container: ContainerEnumName::CombinedHotbarAndInventoryContainer,
            dynamic_id: None,
        },
        storage_item: empty_descriptor_v2(),
    }
}

fn empty_descriptor_v2() -> NetworkItemStackDescriptorV2 {
    NetworkItemStackDescriptorV2 {
        id: 0,
        stack_size: 0,
        aux_value: 0,
        net_id: None,
        block_runtime_id: 0,
        user_data_buffer: vec![],
    }
}
