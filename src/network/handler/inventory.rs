use crate::entity::components::actor_id::ActorId;
use crate::item::item_stack::{ItemStack, user_data_of};
use crate::level::level::Level;
use crate::network::BedrockProtocol;
use crate::network::handler::PacketReceivedMessage;
use crate::network::session::Session;
use crate::network::session::state::{SessionState, SessionStateChangedMessage};
use crate::player::chunk_view::ChunkView;
use crate::player::gamemode::Gamemode;
use crate::player::inventory::{ARMOR_SIZE, HOTBAR_SIZE, Inventory, InventorySlot, MAIN_SIZE, OFFHAND_SIZE, PlayerInventory};
use crate::player::inventory_moves::{LegacyAction, LegacyEndpoint, LegacyOutcome, MoveRules, apply_legacy_actions, armor_slot_accepts, move_items, swap_slots, take_items};
use crate::registry::block_registry::BlockRegistry;
use crate::registry::item_registry::ItemRegistry;
use bedrock::protocol::v662::enums::ContainerID;
use bedrock::protocol::v662::packets::{ContainerOpenPacket, ItemStackResponsePacket, RequestsEntry};
use bedrock::protocol::v662::types::{ActorRuntimeID, ActorUniqueID, InventoryTransactionData};
use bedrock::protocol::v685::packets::ContainerClosePacket;
use bedrock::protocol::v712::types::ItemStackResponseContainerInfo;
use bedrock::protocol::v729::types::FullContainerName;
use bedrock::protocol::v898::packets::InteractPacketAction;
use bedrock::protocol::v944::types::NetworkBlockPosition;
use bedrock::protocol::v975::packets::MobEquipmentPacket;
use bedrock::protocol::v1001::packets::InventoryContentPacket;
use bedrock::protocol::v1001::types::{InventoryAction, InventorySourceKind};
use bedrock::protocol::v2168::enums::ItemStackNetResult;
use bedrock::protocol::v2168::types::{NetworkItemStackDescriptorV2, RedactableString};
use bedrock::protocol::v2193::types::{ItemStackResponseInfo, ItemStackResponseSlotInfo};
use bedrock::protocol::v2225::enums::{ContainerEnumName, ContainerType, ItemStackRequestActionType};
use bevy_ecs::message::{Message, MessageReader, MessageWriter};
use bevy_ecs::prelude::{Entity, Query, Res};
use tracing::debug;

const PLAYER_WINDOW: ContainerID = ContainerID::FIRST;

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

        for container in [ContainerID::INVENTORY, ContainerID::OFFHAND, ContainerID::ARMOR] {
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
                    container_id: PLAYER_WINDOW.0 as u32,
                });
            }
            // the client expects an ACK for every close, otherwise it refuses to open
            // another window afterwards. interesting
            BedrockProtocol::ContainerClosePacket(packet) => {
                session.send(BedrockProtocol::ContainerClosePacket(
                    ContainerClosePacket {
                        container_id: packet.container_id,
                        container_type: packet.container_type.clone(),
                        server_initiated_close: false,
                    }
                    .into(),
                ));

                close_writer.write(InventoryCloseMessage {
                    entity: ev.entity,
                    container_id: packet.container_id.0 as u32,
                });
            }
            BedrockProtocol::MobEquipmentPacket(packet) => {
                // the offhand uses the same packet, only the main inventory changes the held slot
                if !matches!(packet.container_id, ContainerID::INVENTORY) {
                    continue;
                }

                let slot = packet.selected_slot as u8;
                if !inventory.set_held_slot(slot) {
                    send_held_item(&mut session, &inventory, actor);
                    continue;
                }

                held_writer.write(PlayerItemHeldMessage { entity: ev.entity, slot });

                if *gamemode != Gamemode::Creative {
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
                    send_held_item(&mut session, &inventory, actor);
                    continue;
                }

                inventory.main_mut().set(slot as usize, item);
                inventory.set_main_user_data(slot as usize, Vec::new());
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

                send_content(&mut session, &mut inventory, ContainerID::INVENTORY);
                send_held_item(&mut session, &inventory, actor);
            }
            BedrockProtocol::ItemStackRequestPacket(packet) => {
                let mut responses = Vec::new();
                let rules = StackRules { items: &items };

                for request in &packet.requests {
                    let outcome = if gamemode.allows_interaction() {
                        apply_stack_request(&mut inventory, request, &rules, *gamemode == Gamemode::Creative)
                    } else {
                        RequestOutcome::Rejected
                    };
                    debug!(
                        "stack request {} from {}: {:?} -> {}",
                        request.client_request_id,
                        actor.unique_id,
                        request.actions,
                        match &outcome {
                            RequestOutcome::Unsupported => "left to the client",
                            RequestOutcome::Rejected => "rejected",
                            RequestOutcome::Applied { .. } => "applied",
                        }
                    );
                    let response = match outcome {
                        RequestOutcome::Unsupported => continue,
                        RequestOutcome::Applied { dropped, touched } => {
                            for (stack, randomly) in dropped {
                                drop_writer.write(ItemDropMessage { entity: ev.entity, stack, randomly });
                            }
                            applied_response(&mut inventory, request.client_request_id, &touched)
                        }
                        RequestOutcome::Rejected => {
                            debug!("rejecting stack request {} from {}", request.client_request_id, actor.unique_id);
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
            BedrockProtocol::InventoryTransactionPacket(packet) => match &packet.data {
                InventoryTransactionData::Normal => {
                    let window_id = inventory.window_id();
                    let Some(actions) = packet.actions.iter().map(|action| legacy_action(action, window_id)).collect::<Option<Vec<_>>>() else {
                        continue;
                    };
                    if actions.is_empty() {
                        continue;
                    }
                    let outcome = if gamemode.allows_interaction() {
                        apply_legacy_actions(&mut inventory, &actions, &StackRules { items: &items }, *gamemode == Gamemode::Creative)
                    } else {
                        LegacyOutcome::Rejected
                    };
                    debug!("inventory transaction from {}: {:?} -> {:?}", actor.unique_id, actions, outcome);
                    match outcome {
                        LegacyOutcome::Applied { dropped } => {
                            for stack in dropped {
                                drop_writer.write(ItemDropMessage {
                                    entity: ev.entity,
                                    stack,
                                    randomly: false,
                                });
                            }
                        }
                        LegacyOutcome::Rejected => resync_inventory(&mut session, &mut inventory),
                    }
                }
                InventoryTransactionData::InventoryMismatch => {
                    debug!("{} reported an inventory mismatch", actor.unique_id);
                    resync_inventory(&mut session, &mut inventory);
                }
                _ => {}
            },
            _ => {}
        }
    }
}

fn resync_inventory(session: &mut Session, inventory: &mut PlayerInventory) {
    for container in [ContainerID::INVENTORY, ContainerID::ARMOR, ContainerID::OFFHAND] {
        send_content(session, inventory, container);
    }
    if let Some(id) = inventory.window_id() {
        send_content(session, inventory, ContainerID(id));
    }
}

fn stack_of(descriptor: &NetworkItemStackDescriptorV2) -> ItemStack {
    if descriptor.id == 0 || descriptor.stack_size == 0 {
        return ItemStack::air();
    }
    ItemStack {
        id: descriptor.id,
        count: descriptor.stack_size,
        meta: descriptor.aux_value,
        block_runtime_id: descriptor.block_runtime_id as i32,
    }
}

fn legacy_action(action: &InventoryAction<BedrockProtocol>, window_id: Option<i8>) -> Option<LegacyAction> {
    let slot = usize::try_from(action.slot).ok()?;
    let endpoint = match action.source.source_type {
        InventorySourceKind::Container => match action.source.container_id? {
            id if Some(id) == window_id => LegacyEndpoint::Slot(InventorySlot::Window(slot)),
            id if id == ContainerID::INVENTORY.0 => LegacyEndpoint::Slot(InventorySlot::Main(slot)),
            id if id == ContainerID::ARMOR.0 => LegacyEndpoint::Slot(InventorySlot::Armor(slot)),
            id if id == ContainerID::OFFHAND.0 => LegacyEndpoint::Slot(InventorySlot::Offhand(slot)),
            id if id == ContainerID::PLAYER_ONLY_UI.0 && slot == 0 => LegacyEndpoint::Slot(InventorySlot::Cursor),
            _ => return None,
        },
        InventorySourceKind::WorldInteraction if action.source.flags.is_none_or(|flags| flags == 0) => LegacyEndpoint::Drop,
        InventorySourceKind::Creative => LegacyEndpoint::Creative,
        _ => return None,
    };
    Some(LegacyAction {
        endpoint,
        from: stack_of(&action.from_item_descriptor),
        to: stack_of(&action.to_item_descriptor),
        to_data: user_data_of(&action.to_item_descriptor.user_data_buffer),
    })
}

struct StackRules<'a> {
    items: &'a ItemRegistry,
}

impl MoveRules for StackRules<'_> {
    fn max_stack(&self, item: &ItemStack) -> u16 {
        self.items.max_stack_size(item.id)
    }

    fn accepts(&self, slot: InventorySlot, item: &ItemStack) -> bool {
        match slot {
            InventorySlot::Armor(index) => self.items.identifier(item.id).is_some_and(|identifier| armor_slot_accepts(index, identifier)),
            _ => true,
        }
    }
}

struct TouchedSlot {
    container: FullContainerName<BedrockProtocol>,
    slot: i8,
}

enum RequestOutcome {
    Unsupported,
    Rejected,
    Applied { dropped: Vec<(ItemStack, bool)>, touched: Vec<TouchedSlot> },
}

fn is_tracked(container: &FullContainerName<BedrockProtocol>) -> bool {
    matches!(
        container.container,
        ContainerEnumName::HotbarContainer
            | ContainerEnumName::InventoryContainer
            | ContainerEnumName::CombinedHotbarAndInventoryContainer
            | ContainerEnumName::ArmorContainer
            | ContainerEnumName::OffhandContainer
            | ContainerEnumName::CursorContainer
    )
}

fn resolve_slot(container: &FullContainerName<BedrockProtocol>, slot: i8) -> Option<InventorySlot> {
    let index = usize::try_from(slot).ok()?;
    match container.container {
        ContainerEnumName::ArmorContainer => (index < ARMOR_SIZE).then_some(InventorySlot::Armor(index)),
        ContainerEnumName::OffhandContainer => (index < OFFHAND_SIZE).then_some(InventorySlot::Offhand(index)),
        ContainerEnumName::CursorContainer => (index == 0).then_some(InventorySlot::Cursor),
        _ => main_slot(container, slot).map(InventorySlot::Main),
    }
}

fn apply_stack_request(inventory: &mut PlayerInventory, request: &RequestsEntry<BedrockProtocol>, rules: &dyn MoveRules, may_destroy: bool) -> RequestOutcome {
    let tracked = |action: &ItemStackRequestActionType<BedrockProtocol>| match action {
        ItemStackRequestActionType::Take { source, destination, .. } | ItemStackRequestActionType::Place { source, destination, .. } | ItemStackRequestActionType::Swap { source, destination } => {
            is_tracked(&source.container_name) && is_tracked(&destination.container_name)
        }
        ItemStackRequestActionType::Drop { source, .. } | ItemStackRequestActionType::Destroy { source, .. } => is_tracked(&source.container_name),
        _ => false,
    };
    if request.actions.is_empty() || !request.actions.iter().all(tracked) {
        return RequestOutcome::Unsupported;
    }

    let backup = inventory.clone();
    let mut dropped = Vec::new();
    let mut touched = Vec::new();
    let mut touch = |container: &FullContainerName<BedrockProtocol>, slot: i8| touched.push(TouchedSlot { container: container.clone(), slot });

    for action in &request.actions {
        let done = match action {
            ItemStackRequestActionType::Take { amount, source, destination } | ItemStackRequestActionType::Place { amount, source, destination } => {
                touch(&source.container_name, source.slot);
                touch(&destination.container_name, destination.slot);
                match (
                    resolve_slot(&source.container_name, source.slot),
                    resolve_slot(&destination.container_name, destination.slot),
                    u16::try_from(*amount),
                ) {
                    (Some(from), Some(to), Ok(amount)) => move_items(inventory, from, to, amount, rules),
                    _ => false,
                }
            }
            ItemStackRequestActionType::Swap { source, destination } => {
                touch(&source.container_name, source.slot);
                touch(&destination.container_name, destination.slot);
                match (resolve_slot(&source.container_name, source.slot), resolve_slot(&destination.container_name, destination.slot)) {
                    (Some(first), Some(second)) => swap_slots(inventory, first, second, rules),
                    _ => false,
                }
            }
            ItemStackRequestActionType::Drop { amount, source, randomly } => {
                touch(&source.container_name, source.slot);
                match (resolve_slot(&source.container_name, source.slot), u16::try_from(*amount)) {
                    (Some(from), Ok(amount)) => take_items(inventory, from, amount).map(|(stack, _)| dropped.push((stack, *randomly))).is_some(),
                    _ => false,
                }
            }
            ItemStackRequestActionType::Destroy { amount, source } => {
                touch(&source.container_name, source.slot);
                match (resolve_slot(&source.container_name, source.slot), u16::try_from(*amount)) {
                    (Some(from), Ok(amount)) if may_destroy => take_items(inventory, from, amount).is_some(),
                    _ => false,
                }
            }
            _ => false,
        };
        if !done {
            *inventory = backup;
            return RequestOutcome::Rejected;
        }
    }

    RequestOutcome::Applied { dropped, touched }
}

fn applied_response(inventory: &mut PlayerInventory, client_request_id: i32, touched: &[TouchedSlot]) -> ItemStackResponseInfo<BedrockProtocol> {
    let mut containers: Vec<ItemStackResponseContainerInfo<BedrockProtocol>> = Vec::new();
    let mut seen: Vec<(std::mem::Discriminant<ContainerEnumName>, i8)> = Vec::new();

    for touched in touched {
        let key = (std::mem::discriminant(&touched.container.container), touched.slot);
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        let Some(slot) = resolve_slot(&touched.container, touched.slot) else { continue };
        let stack = inventory.stack_at(slot).unwrap_or_else(ItemStack::air);
        let net_id = (!stack.is_empty()).then(|| inventory.next_stack_id());

        containers.push(ItemStackResponseContainerInfo {
            container_name: touched.container.clone(),
            slots: vec![ItemStackResponseSlotInfo {
                requested_slot: touched.slot,
                slot: touched.slot,
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
        client_request_id,
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
        let user_data = inventory.user_data(container, slot).unwrap_or_default();

        slots.push(item.to_descriptor_with_user_data(net_id, user_data));
    }

    session.send(BedrockProtocol::InventoryContentPacket(
        InventoryContentPacket {
            inventory_id: container.0 as u32,
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
    let user_data = inventory.main_user_data(held_slot as usize).unwrap_or_default();

    session.send(BedrockProtocol::MobEquipmentPacket(
        MobEquipmentPacket {
            target_runtime_id: ActorRuntimeID(actor.runtime_id),
            item: inventory.held_item().to_descriptor_with_user_data(None, user_data),
            slot: held_slot,
            selected_slot: held_slot,
            container_id: ContainerID::INVENTORY,
        }
        .into(),
    ));
}

fn container_of<'a>(inventory: &'a PlayerInventory, container: &ContainerID) -> &'a Inventory {
    match *container {
        ContainerID::OFFHAND => inventory.offhand(),
        ContainerID::ARMOR => inventory.armor(),
        ContainerID(id) if inventory.window_id() == Some(id) => inventory.window().unwrap_or(inventory.main()),
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

    fn slot_info(container: ContainerEnumName, slot: i8) -> ItemStackRequestSlotInfo<BedrockProtocol> {
        ItemStackRequestSlotInfo {
            container_name: FullContainerName { container, dynamic_id: None },
            slot,
            raw_id: 0,
        }
    }

    fn move_action(container_from: ContainerEnumName, from: i8, container_to: ContainerEnumName, to: i8, amount: i8) -> ItemStackRequestActionType<BedrockProtocol> {
        ItemStackRequestActionType::Take {
            amount,
            source: slot_info(container_from, from),
            destination: slot_info(container_to, to),
        }
    }

    fn dropped_by(inventory: &mut PlayerInventory, request: &RequestsEntry<BedrockProtocol>) -> Option<Vec<(ItemStack, bool)>> {
        let items = ItemRegistry::new();
        match apply_stack_request(inventory, request, &StackRules { items: &items }, false) {
            RequestOutcome::Applied { dropped, .. } => Some(dropped),
            _ => None,
        }
    }

    fn outcome(inventory: &mut PlayerInventory, request: &RequestsEntry<BedrockProtocol>) -> &'static str {
        let items = ItemRegistry::new();
        match apply_stack_request(inventory, request, &StackRules { items: &items }, false) {
            RequestOutcome::Applied { .. } => "applied",
            RequestOutcome::Rejected => "rejected",
            RequestOutcome::Unsupported => "unsupported",
        }
    }

    fn descriptor(id: i16, count: u16) -> NetworkItemStackDescriptorV2 {
        stack(count).to_descriptor_with_user_data(None, &[]).tap_id(id)
    }

    trait TapId {
        fn tap_id(self, id: i16) -> Self;
    }

    impl TapId for NetworkItemStackDescriptorV2 {
        fn tap_id(mut self, id: i16) -> Self {
            if self.id != 0 {
                self.id = id;
            }
            self
        }
    }

    fn transaction_action(source_type: InventorySourceKind, container_id: Option<i8>, slot: u32) -> InventoryAction<BedrockProtocol> {
        InventoryAction {
            source: bedrock::protocol::v2193::types::InventorySource {
                source_type,
                container_id,
                flags: None,
            },
            slot,
            from_item_descriptor: descriptor(1, 0),
            to_item_descriptor: descriptor(1, 3),
        }
    }

    #[test]
    fn legacy_actions_map_onto_the_players_containers() {
        let main = legacy_action(&transaction_action(InventorySourceKind::Container, Some(ContainerID::INVENTORY.0), 5), None).unwrap();
        assert_eq!(main.endpoint, LegacyEndpoint::Slot(InventorySlot::Main(5)));
        assert_eq!(main.to.count, 3);
        assert!(main.from.is_empty());
        let armor = legacy_action(&transaction_action(InventorySourceKind::Container, Some(ContainerID::ARMOR.0), 2), None).unwrap();
        assert_eq!(armor.endpoint, LegacyEndpoint::Slot(InventorySlot::Armor(2)));
        let offhand = legacy_action(&transaction_action(InventorySourceKind::Container, Some(ContainerID::OFFHAND.0), 0), None).unwrap();
        assert_eq!(offhand.endpoint, LegacyEndpoint::Slot(InventorySlot::Offhand(0)));
        let cursor = legacy_action(&transaction_action(InventorySourceKind::Container, Some(ContainerID::PLAYER_ONLY_UI.0), 0), None).unwrap();
        assert_eq!(cursor.endpoint, LegacyEndpoint::Slot(InventorySlot::Cursor));
    }

    #[test]
    fn the_open_window_id_maps_onto_the_window_slots() {
        let chest = legacy_action(&transaction_action(InventorySourceKind::Container, Some(7), 12), Some(7)).unwrap();
        assert_eq!(chest.endpoint, LegacyEndpoint::Slot(InventorySlot::Window(12)));
        assert!(legacy_action(&transaction_action(InventorySourceKind::Container, Some(7), 12), None).is_none());
        assert!(legacy_action(&transaction_action(InventorySourceKind::Container, Some(8), 12), Some(7)).is_none());
    }

    #[test]
    fn legacy_actions_for_things_the_server_does_not_track_are_left_alone() {
        assert!(legacy_action(&transaction_action(InventorySourceKind::Container, Some(ContainerID::PLAYER_ONLY_UI.0), 1), None).is_none());
        assert!(legacy_action(&transaction_action(InventorySourceKind::Container, Some(50), 0), None).is_none());
        assert!(legacy_action(&transaction_action(InventorySourceKind::Container, None, 0), None).is_none());
        assert!(legacy_action(&transaction_action(InventorySourceKind::UntrackedInteractionUI, None, 0), None).is_none());
    }

    #[test]
    fn dropping_and_creative_actions_are_recognised() {
        let drop = legacy_action(&transaction_action(InventorySourceKind::WorldInteraction, None, 0), None).unwrap();
        assert_eq!(drop.endpoint, LegacyEndpoint::Drop);
        let creative = legacy_action(&transaction_action(InventorySourceKind::Creative, None, 0), None).unwrap();
        assert_eq!(creative.endpoint, LegacyEndpoint::Creative);
    }

    #[test]
    fn dragging_a_stack_to_another_slot_through_the_cursor_is_applied() {
        let mut inventory = PlayerInventory::new();
        inventory.main_mut().set(0, stack(8));
        let take = request(vec![move_action(ContainerEnumName::HotbarContainer, 0, ContainerEnumName::CursorContainer, 0, 8)]);
        assert_eq!(outcome(&mut inventory, &take), "applied");
        assert_eq!(inventory.cursor(), &stack(8));
        let place = request(vec![move_action(ContainerEnumName::CursorContainer, 0, ContainerEnumName::InventoryContainer, 12, 8)]);
        assert_eq!(outcome(&mut inventory, &place), "applied");
        assert_eq!(inventory.main().get(12), Some(&stack(8)));
        assert!(inventory.main().get(0).unwrap().is_empty());
        assert!(inventory.cursor().is_empty());
    }

    #[test]
    fn a_refused_move_puts_everything_back() {
        let mut inventory = PlayerInventory::new();
        inventory.main_mut().set(0, stack(2));
        let actions = vec![
            move_action(ContainerEnumName::HotbarContainer, 0, ContainerEnumName::HotbarContainer, 1, 1),
            move_action(ContainerEnumName::HotbarContainer, 0, ContainerEnumName::HotbarContainer, 2, 5),
        ];
        assert_eq!(outcome(&mut inventory, &request(actions)), "rejected");
        assert_eq!(inventory.main().get(0), Some(&stack(2)));
        assert!(inventory.main().get(1).unwrap().is_empty());
    }

    #[test]
    fn requests_about_crafting_are_left_to_the_client() {
        let mut inventory = PlayerInventory::new();
        let craft = request(vec![move_action(ContainerEnumName::CraftingOutputPreviewContainer, 0, ContainerEnumName::CursorContainer, 0, 1)]);
        assert_eq!(outcome(&mut inventory, &craft), "unsupported");
        assert_eq!(outcome(&mut inventory, &request(vec![])), "unsupported");
    }

    #[test]
    fn armour_slots_refuse_items_the_server_does_not_know_as_armour() {
        let mut inventory = PlayerInventory::new();
        inventory.main_mut().set(0, stack(1));
        let equip = request(vec![move_action(ContainerEnumName::HotbarContainer, 0, ContainerEnumName::ArmorContainer, 0, 1)]);
        assert_eq!(outcome(&mut inventory, &equip), "rejected");
        assert!(inventory.armor().get(0).unwrap().is_empty());
    }

    #[test]
    fn drops_take_from_the_right_slot() {
        let mut inventory = PlayerInventory::new();
        inventory.main_mut().set(2, stack(10));
        inventory.main_mut().set(20, stack(5));

        let dropped = dropped_by(
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
        assert!(dropped_by(&mut inventory, &request(actions)).is_none(), "only two are left for the second drop");
        assert!(
            dropped_by(&mut inventory, &request(vec![drop_action(ContainerEnumName::InventoryContainer, 3, 1)])).is_none(),
            "slot 3 is the hotbar"
        );
        assert!(
            dropped_by(&mut inventory, &request(vec![drop_action(ContainerEnumName::CursorContainer, 0, 1)])).is_none(),
            "the cursor is empty"
        );
        assert_eq!(inventory.main().get(0), Some(&stack(4)));
    }
}
