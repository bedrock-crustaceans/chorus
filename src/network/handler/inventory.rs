use crate::item::item_stack::ItemStack;
use crate::level::level::Level;
use crate::network::BedrockProtocol;
use crate::network::handler::PacketReceivedMessage;
use crate::network::session::Session;
use crate::network::session::state::{SessionState, SessionStateChangedMessage};
use crate::player::Player;
use crate::player::gamemode::Gamemode;
use crate::player::inventory::Inventory;
use crate::registry::block_registry::BlockRegistry;
use crate::registry::item_registry::ItemRegistry;
use bedrock::protocol::v662::enums::{ContainerID, ContainerType};
use bedrock::protocol::v662::packets::ContainerOpenPacket;
use bedrock::protocol::v662::types::{ActorRuntimeID, ActorUniqueID};
use bedrock::protocol::v685::packets::ContainerClosePacket;
use bedrock::protocol::v729::types::FullContainerName;
use bedrock::protocol::v898::packets::InteractPacketAction;
use bedrock::protocol::v944::enums::ContainerEnumName;
use bedrock::protocol::v944::types::NetworkBlockPosition;
use bedrock::protocol::v975::packets::MobEquipmentPacket;
use bedrock::protocol::v1001::packets::InventoryContentPacket;
use bevy_ecs::message::MessageReader;
use bevy_ecs::prelude::{Query, Res};
use tracing::debug;

const PLAYER_WINDOW: ContainerID = ContainerID::First;

pub fn send_initial_inventory(mut sessions: Query<(&mut Session, &mut Player)>, mut state_reader: MessageReader<SessionStateChangedMessage>) {
    for ev in state_reader.read() {
        if ev.to != SessionState::Play {
            continue;
        }

        let Ok((mut session, mut player)) = sessions.get_mut(ev.entity) else {
            continue;
        };

        for container in [ContainerID::Inventory, ContainerID::Offhand, ContainerID::Armor] {
            send_content(&mut session, &mut player, container);
        }

        send_held_item(&mut session, &player);
    }
}

pub fn handle_inventory_packets(
    mut packet_reader: MessageReader<PacketReceivedMessage>,
    blocks: Res<BlockRegistry>,
    items: Res<ItemRegistry>,
    level: Res<Level>,
    mut query: Query<(&mut Session, &mut Player)>,
) {
    for ev in packet_reader.read() {
        let Ok((mut session, mut player)) = query.get_mut(ev.entity) else {
            continue;
        };
        if session.get_state() != SessionState::Play {
            continue;
        }

        match &ev.packet {
            BedrockProtocol::InteractPacket(packet) => {
                if !matches!(packet.action, InteractPacketAction::OpenInventory) {
                    continue;
                }

                debug!("opening inventory for {}", player.unique_id());

                session.send(BedrockProtocol::ContainerOpenPacket(
                    ContainerOpenPacket {
                        container_id: PLAYER_WINDOW,
                        container_type: ContainerType::Inventory,
                        position: NetworkBlockPosition { x: 0, y: 0, z: 0 },
                        target_actor_id: ActorUniqueID(player.unique_id()),
                    }
                    .into(),
                ));
            }
            // the client expects an ACK for every close, otherwise it refuses to open
            // another window afterwards. interesting
            BedrockProtocol::ContainerClosePacket(packet) => {
                session.send(BedrockProtocol::ContainerClosePacket(
                    ContainerClosePacket {
                        container_id: packet.container_id.clone(),
                        container_type: packet.container_type.clone(),
                        server_initiated_close: false,
                    }
                    .into(),
                ));
            }
            BedrockProtocol::MobEquipmentPacket(packet) => {
                // the offhand uses the same packet, only the main inventory changes the held slot
                if !matches!(packet.container_id, ContainerID::Inventory) {
                    continue;
                }

                let slot = packet.selected_slot as u8;
                if !player.inventory.set_held_slot(slot) {
                    send_held_item(&mut session, &player);
                    continue;
                }

                // creative picks are the only way items enter an inventory right now, and they
                // arrive as item stack requests chorus cannot read yet
                let item = ItemStack {
                    id: packet.item.id,
                    count: packet.item.stack_size,
                    meta: packet.item.aux_value,
                    block_runtime_id: packet.item.block_runtime_id as i32,
                };

                let known = item.is_empty() || blocks.get_permutation(item.block_runtime_id).is_some();
                if !known {
                    debug!("refusing unknown item {} in slot {}", item.id, slot);
                    send_held_item(&mut session, &player);
                    continue;
                }

                player.inventory.main_mut().set(slot as usize, item);
            }
            // middle click - the client asks the server to hand it the block it is looking at
            BedrockProtocol::BlockPickRequestPacket(packet) => {
                let position = &packet.position;
                let Some(block_id) = level.get_block(0, position.x, position.y, position.z, 0) else {
                    continue;
                };

                if Some(block_id) == blocks.get_block_id("minecraft:air") {
                    continue;
                }

                let Some(item) = picked_item(&blocks, &items, block_id) else {
                    debug!("no item for block {}", block_id);
                    continue;
                };

                // only creative players get a stack they do not own yet
                let allow_new = player.gamemode() == Gamemode::Creative;
                if !player.inventory.pick_item(item, allow_new) {
                    continue;
                }

                send_content(&mut session, &mut player, ContainerID::Inventory);
                send_held_item(&mut session, &player);
            }
            _ => {}
        }
    }
}

/// Resolves the item a block hands out when it is picked. Block and item share the identifier.
fn picked_item(blocks: &BlockRegistry, items: &ItemRegistry, block_id: i32) -> Option<ItemStack> {
    let permutation = blocks.get_permutation(block_id)?;
    let id = items.get(permutation.get_identifier())?;

    Some(ItemStack {
        id,
        count: 1,
        meta: 0,
        block_runtime_id: block_id,
    })
}

fn send_content(session: &mut Session, player: &mut Player, container: ContainerID) {
    let size = inventory(player, &container).size();
    let mut slots = Vec::with_capacity(size);

    for slot in 0..size {
        let item = inventory(player, &container).get(slot).copied().unwrap_or_else(ItemStack::air);
        let net_id = (!item.is_empty()).then(|| player.inventory.next_stack_id());

        slots.push(item.to_descriptor(net_id));
    }

    session.send(BedrockProtocol::InventoryContentPacket(
        InventoryContentPacket {
            inventory_id: container as u32,
            slots,
            container_name_data: FullContainerName {
                container: ContainerEnumName::AnvilInputContainer,
                dynamic_id: None,
            },
            storage_item: ItemStack::air().to_descriptor(None),
        }
        .into(),
    ));
}

fn send_held_item(session: &mut Session, player: &Player) {
    let held_slot = player.inventory.held_slot() as i8;

    session.send(BedrockProtocol::MobEquipmentPacket(
        MobEquipmentPacket {
            target_runtime_id: ActorRuntimeID(player.runtime_id()),
            item: player.inventory.held_item().to_descriptor(None),
            slot: held_slot,
            selected_slot: held_slot,
            container_id: ContainerID::Inventory,
        }
        .into(),
    ));
}

fn inventory<'a>(player: &'a Player, container: &ContainerID) -> &'a Inventory {
    match container {
        ContainerID::Offhand => player.inventory.offhand(),
        ContainerID::Armor => player.inventory.armor(),
        _ => player.inventory.main(),
    }
}
