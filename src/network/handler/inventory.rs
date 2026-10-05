use crate::entity::components::actor_id::ActorId;
use crate::item::item_stack::ItemStack;
use crate::level::level::Level;
use crate::network::BedrockProtocol;
use crate::network::handler::PacketReceivedMessage;
use crate::network::session::Session;
use crate::network::session::state::{SessionState, SessionStateChangedMessage};
use crate::player::chunk_view::ChunkView;
use crate::player::gamemode::Gamemode;
use crate::player::inventory::{HOTBAR_SIZE, Inventory, MAIN_SIZE, PlayerInventory};
use crate::registry::block_registry::BlockRegistry;
use crate::registry::item_registry::ItemRegistry;
use bedrock::protocol::v662::enums::{ContainerID, ContainerType};
use bedrock::protocol::v662::packets::{ContainerOpenPacket, ItemStackResponsePacket, RequestsEntry};
use bedrock::protocol::v662::types::{ActorRuntimeID, ActorUniqueID};
use bedrock::protocol::v685::packets::ContainerClosePacket;
use bedrock::protocol::v712::types::ItemStackResponseContainerInfo;
use bedrock::protocol::v729::types::FullContainerName;
use bedrock::protocol::v898::packets::InteractPacketAction;
use bedrock::protocol::v944::enums::ContainerEnumName;
use bedrock::protocol::v944::types::NetworkBlockPosition;
use bedrock::protocol::v975::packets::MobEquipmentPacket;
use bedrock::protocol::v1001::packets::InventoryContentPacket;
use bedrock::protocol::v2168::enums::{ItemStackNetResult, ItemStackRequestActionType};
use bedrock::protocol::v2168::types::RedactableString;
use bedrock::protocol::v2193::types::{ItemStackResponseInfo, ItemStackResponseSlotInfo};
use bevy_ecs::message::{Message, MessageReader, MessageWriter};
use bevy_ecs::prelude::{Entity, Query, Res};
use tracing::debug;

const PLAYER_WINDOW: ContainerID = ContainerID::First;

#[derive(Message, Clone, Debug)]
pub struct InventoryOpenMessage {
    pub entity: Entity,
    pub container_id: u32,
}

#[derive(Message, Clone, Debug)]
pub struct InventoryCloseMessage {
    pub entity: Entity,
    pub container_id: u32,
}

/// A player threw items out of their inventory. They are already gone from it, whoever reads
/// this puts them in the world.
#[derive(Message, Clone, Debug)]
pub struct ItemDropMessage {
    pub entity: Entity,
    pub stack: ItemStack,
    /// Scatter the item in a random direction instead of throwing it where the player looks.
    pub randomly: bool,
}

#[derive(Message, Clone, Debug)]
pub struct PlayerItemHeldMessage {
    pub entity: Entity,
    pub slot: u8,
}

pub fn send_initial_inventory(mut sessions: Query<(&mut Session, &mut PlayerInventory, &ActorId)>, mut state_reader: MessageReader<SessionStateChangedMessage>) {
    for ev in state_reader.read() {
        if ev.to != SessionState::Play {
            continue;
        }

        let Ok((mut session, mut inventory, actor)) = sessions.get_mut(ev.entity) else {
            continue;
        };

        for container in [ContainerID::Inventory, ContainerID::Offhand, ContainerID::Armor] {
            send_content(&mut session, &mut inventory, container);
        }

        send_held_item(&mut session, &inventory, actor);
    }
}

pub fn handle_inventory_packets(
    mut packet_reader: MessageReader<PacketReceivedMessage>,
    blocks: Res<BlockRegistry>,
    items: Res<ItemRegistry>,
    level: Res<Level>,
    mut query: Query<(&mut Session, &mut PlayerInventory, &ActorId, &Gamemode, &ChunkView)>,
    mut open_writer: MessageWriter<InventoryOpenMessage>,
    mut close_writer: MessageWriter<InventoryCloseMessage>,
    mut held_writer: MessageWriter<PlayerItemHeldMessage>,
    mut drop_writer: MessageWriter<ItemDropMessage>,
) {
    for ev in packet_reader.read() {
        let Ok((mut session, mut inventory, actor, gamemode, view)) = query.get_mut(ev.entity) else {
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

                debug!("opening inventory for {}", actor.unique_id);

                session.send(BedrockProtocol::ContainerOpenPacket(
                    ContainerOpenPacket {
                        container_id: PLAYER_WINDOW,
                        container_type: ContainerType::Inventory,
                        position: NetworkBlockPosition { x: 0, y: 0, z: 0 },
                        target_actor_id: ActorUniqueID(actor.unique_id),
                    }
                    .into(),
                ));

                open_writer.write(InventoryOpenMessage {
                    entity: ev.entity,
                    container_id: PLAYER_WINDOW as u32,
                });
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

                close_writer.write(InventoryCloseMessage {
                    entity: ev.entity,
                    container_id: packet.container_id.clone() as u32,
                });
            }
            BedrockProtocol::MobEquipmentPacket(packet) => {
                // the offhand uses the same packet, only the main inventory changes the held slot
                if !matches!(packet.container_id, ContainerID::Inventory) {
                    continue;
                }

                let slot = packet.selected_slot as u8;
                if !inventory.set_held_slot(slot) {
                    send_held_item(&mut session, &inventory, actor);
                    continue;
                }

                held_writer.write(PlayerItemHeldMessage { entity: ev.entity, slot });

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
                    send_held_item(&mut session, &inventory, actor);
                    continue;
                }

                inventory.main_mut().set(slot as usize, item);
            }
            // middle click - the client asks the server to hand it the block it is looking at
            BedrockProtocol::BlockPickRequestPacket(packet) => {
                if !gamemode.allows_interaction() {
                    continue;
                }
                let position = &packet.position;
                let Some(block_id) = level.get_block(view.dimension, position.x, position.y, position.z, 0) else {
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
                let allow_new = *gamemode == Gamemode::Creative;
                if !inventory.pick_item(item, allow_new) {
                    continue;
                }

                send_content(&mut session, &mut inventory, ContainerID::Inventory);
                send_held_item(&mut session, &inventory, actor);
            }
            BedrockProtocol::ItemStackRequestPacket(packet) => {
                let mut responses = Vec::new();

                for request in &packet.requests {
                    // only drops for now. anything else stays unanswered so the client keeps its own
                    // prediction, creative picking still leans on that
                    let only_drops = !request.actions.is_empty() && request.actions.iter().all(|action| matches!(action, ItemStackRequestActionType::Drop { .. }));
                    if !only_drops {
                        continue;
                    }

                    let dropped = if gamemode.allows_interaction() { drop_items(&mut inventory, request) } else { None };
                    let response = match dropped {
                        Some(dropped) => {
                            for (stack, randomly) in dropped {
                                drop_writer.write(ItemDropMessage { entity: ev.entity, stack, randomly });
                            }
                            dropped_response(&mut inventory, request)
                        }
                        None => {
                            debug!("rejecting drop request {} from {}", request.client_request_id, actor.unique_id);
                            ItemStackResponseInfo {
                                result: ItemStackNetResult::Error,
                                client_request_id: request.client_request_id,
                                containers: None,
                            }
                        }
                    };
                    responses.push(response);
                }

                if !responses.is_empty() {
                    session.send(BedrockProtocol::ItemStackResponsePacket(ItemStackResponsePacket { responses }.into()));
                }
            }
            _ => {}
        }
    }
}

/// Takes every dropped stack out of the main inventory. It is all or nothing: if one action points
/// at an empty slot or asks for more than the slot holds, nothing changes and `None` comes back.
fn drop_items(inventory: &mut PlayerInventory, request: &RequestsEntry<BedrockProtocol>) -> Option<Vec<(ItemStack, bool)>> {
    let mut slots = inventory.main().slots().to_vec();
    let mut dropped = Vec::with_capacity(request.actions.len());

    for action in &request.actions {
        let ItemStackRequestActionType::Drop { amount, source, randomly } = action else {
            return None;
        };
        let slot = main_slot(&source.container_name, source.slot)?;
        let amount = u16::try_from(*amount).ok().filter(|amount| *amount > 0)?;
        let stack = &mut slots[slot];
        if stack.is_empty() || stack.count < amount {
            return None;
        }

        stack.count -= amount;
        dropped.push((ItemStack { count: amount, ..*stack }, *randomly));
        if stack.count == 0 {
            *stack = ItemStack::air();
        }
    }

    for (slot, stack) in slots.into_iter().enumerate() {
        inventory.main_mut().set(slot, stack);
    }
    Some(dropped)
}

/// Tells the client what is left in each slot it dropped from, with fresh stack ids.
fn dropped_response(inventory: &mut PlayerInventory, request: &RequestsEntry<BedrockProtocol>) -> ItemStackResponseInfo<BedrockProtocol> {
    let mut containers = Vec::with_capacity(request.actions.len());

    for action in &request.actions {
        let ItemStackRequestActionType::Drop { source, .. } = action else { continue };
        let Some(slot) = main_slot(&source.container_name, source.slot) else { continue };
        let stack = inventory.main().get(slot).copied().unwrap_or_else(ItemStack::air);
        let net_id = (!stack.is_empty()).then(|| inventory.next_stack_id());

        containers.push(ItemStackResponseContainerInfo {
            container_name: source.container_name.clone(),
            slots: vec![ItemStackResponseSlotInfo {
                requested_slot: source.slot,
                slot: source.slot,
                amount: stack.count.min(i8::MAX as u16) as i8,
                item_stack_net_id: net_id,
                custom_name: RedactableString {
                    unredacted: String::new(),
                    redacted: None,
                },
                durability_correction: 0,
            }],
        });
    }

    ItemStackResponseInfo {
        result: ItemStackNetResult::Success,
        client_request_id: request.client_request_id,
        containers: Some(containers),
    }
}

/// Maps a request's container and slot onto the main inventory. The hotbar is its first nine slots,
/// so both containers share the same indices.
fn main_slot(container: &FullContainerName<BedrockProtocol>, slot: i8) -> Option<usize> {
    let slot = usize::try_from(slot).ok()?;
    let slots = match container.container {
        ContainerEnumName::HotbarContainer => 0..HOTBAR_SIZE,
        ContainerEnumName::InventoryContainer => HOTBAR_SIZE..MAIN_SIZE,
        ContainerEnumName::CombinedHotbarAndInventoryContainer => 0..MAIN_SIZE,
        _ => return None,
    };

    slots.contains(&slot).then_some(slot)
}

/// Resolves the item a block hands out when it is picked. Block and item share the identifier.
pub(crate) fn picked_item(blocks: &BlockRegistry, items: &ItemRegistry, block_id: i32) -> Option<ItemStack> {
    let permutation = blocks.get_permutation(block_id)?;
    let id = items.get(permutation.get_identifier())?;

    Some(ItemStack {
        id,
        count: 1,
        meta: 0,
        block_runtime_id: block_id,
    })
}

pub(crate) fn send_content(session: &mut Session, inventory: &mut PlayerInventory, container: ContainerID) {
    let size = container_of(inventory, &container).size();
    let mut slots = Vec::with_capacity(size);

    for slot in 0..size {
        let item = container_of(inventory, &container).get(slot).copied().unwrap_or_else(ItemStack::air);
        let net_id = (!item.is_empty()).then(|| inventory.next_stack_id());

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

fn send_held_item(session: &mut Session, inventory: &PlayerInventory, actor: &ActorId) {
    let held_slot = inventory.held_slot() as i8;

    session.send(BedrockProtocol::MobEquipmentPacket(
        MobEquipmentPacket {
            target_runtime_id: ActorRuntimeID(actor.runtime_id),
            item: inventory.held_item().to_descriptor(None),
            slot: held_slot,
            selected_slot: held_slot,
            container_id: ContainerID::Inventory,
        }
        .into(),
    ));
}

fn container_of<'a>(inventory: &'a PlayerInventory, container: &ContainerID) -> &'a Inventory {
    match container {
        ContainerID::Offhand => inventory.offhand(),
        ContainerID::Armor => inventory.armor(),
        _ => inventory.main(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bedrock::protocol::v662::enums::TextProcessingEventOrigin;
    use bedrock::protocol::v2168::types::ItemStackRequestSlotInfo;

    fn drop_action(container: ContainerEnumName, slot: i8, amount: i8) -> ItemStackRequestActionType<BedrockProtocol> {
        ItemStackRequestActionType::Drop {
            amount,
            source: ItemStackRequestSlotInfo {
                container_name: FullContainerName { container, dynamic_id: None },
                slot,
                raw_id: 0,
            },
            randomly: false,
        }
    }

    fn request(actions: Vec<ItemStackRequestActionType<BedrockProtocol>>) -> RequestsEntry<BedrockProtocol> {
        RequestsEntry {
            client_request_id: -1,
            actions,
            strings_to_filter: vec![],
            strings_to_filter_origin: TextProcessingEventOrigin::Unknown,
        }
    }

    fn stack(count: u16) -> ItemStack {
        ItemStack { id: 1, count, ..ItemStack::default() }
    }

    #[test]
    fn drops_take_from_the_right_slot() {
        let mut inventory = PlayerInventory::new();
        inventory.main_mut().set(2, stack(10));
        inventory.main_mut().set(20, stack(5));

        let dropped = drop_items(
            &mut inventory,
            &request(vec![drop_action(ContainerEnumName::HotbarContainer, 2, 3), drop_action(ContainerEnumName::InventoryContainer, 20, 5)]),
        )
        .expect("both drops fit");

        assert_eq!(dropped, vec![(stack(3), false), (stack(5), false)]);
        assert_eq!(inventory.main().get(2), Some(&stack(7)));
        assert!(inventory.main().get(20).unwrap().is_empty());
    }

    #[test]
    fn a_bad_drop_changes_nothing() {
        let mut inventory = PlayerInventory::new();
        inventory.main_mut().set(0, stack(4));

        let actions = vec![drop_action(ContainerEnumName::HotbarContainer, 0, 2), drop_action(ContainerEnumName::HotbarContainer, 0, 3)];
        assert!(drop_items(&mut inventory, &request(actions)).is_none(), "only two are left for the second drop");
        assert!(
            drop_items(&mut inventory, &request(vec![drop_action(ContainerEnumName::InventoryContainer, 3, 1)])).is_none(),
            "slot 3 is the hotbar"
        );
        assert!(drop_items(&mut inventory, &request(vec![drop_action(ContainerEnumName::CursorContainer, 0, 1)])).is_none());
        assert_eq!(inventory.main().get(0), Some(&stack(4)));
    }
}
