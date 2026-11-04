use crate::item::item_stack::ItemStack;
use crate::player::inventory::{InventorySlot, PlayerInventory};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LegacyEndpoint {
    Slot(InventorySlot),
    Drop,
    Creative,
}

#[derive(Clone, Debug)]
pub struct LegacyAction {
    pub endpoint: LegacyEndpoint,
    pub from: ItemStack,
    pub to: ItemStack,
    pub to_data: Vec<u8>,
}

#[derive(Debug, PartialEq)]
pub enum LegacyOutcome {
    Applied { dropped: Vec<ItemStack> },
    Rejected,
}

fn same_stack(a: &ItemStack, b: &ItemStack) -> bool {
    (a.is_empty() && b.is_empty()) || (a.id == b.id && a.meta == b.meta && a.count == b.count)
}

pub fn apply_legacy_actions(inventory: &mut PlayerInventory, actions: &[LegacyAction], rules: &dyn MoveRules, may_create: bool) -> LegacyOutcome {
    let mut balance: HashMap<(i16, u32), i64> = HashMap::new();
    let mut creative = false;

    for action in actions {
        match action.endpoint {
            LegacyEndpoint::Slot(slot) => {
                let Some(current) = inventory.stack_at(slot) else {
                    return LegacyOutcome::Rejected;
                };
                if !same_stack(&current, &action.from) {
                    return LegacyOutcome::Rejected;
                }
                if !action.to.is_empty() && (!rules.accepts(slot, &action.to) || action.to.count > rules.max_stack(&action.to)) {
                    return LegacyOutcome::Rejected;
                }
            }
            LegacyEndpoint::Drop => {}
            LegacyEndpoint::Creative => {
                if !may_create {
                    return LegacyOutcome::Rejected;
                }
                creative = true;
            }
        }
        if !action.from.is_empty() {
            *balance.entry((action.from.id, action.from.meta)).or_default() += i64::from(action.from.count);
        }
        if !action.to.is_empty() {
            *balance.entry((action.to.id, action.to.meta)).or_default() -= i64::from(action.to.count);
        }
    }
    if !creative && balance.values().any(|difference| *difference != 0) {
        return LegacyOutcome::Rejected;
    }

    let mut dropped = Vec::new();
    for action in actions {
        match action.endpoint {
            LegacyEndpoint::Slot(slot) => {
                let data = if action.to.is_empty() { Vec::new() } else { action.to_data.clone() };
                inventory.put(slot, action.to, data);
            }
            LegacyEndpoint::Drop if !action.to.is_empty() => dropped.push(action.to),
            _ => {}
        }
    }
    LegacyOutcome::Applied { dropped }
}

pub fn armor_slot_accepts(slot: usize, identifier: &str) -> bool {
    let name = identifier.strip_prefix("minecraft:").unwrap_or(identifier);
    match slot {
        0 => name.ends_with("_helmet") || name == "carved_pumpkin" || name.ends_with("_head") || name.ends_with("_skull"),
        1 => name.ends_with("_chestplate") || name == "elytra",
        2 => name.ends_with("_leggings"),
        3 => name.ends_with("_boots"),
        _ => false,
    }
}

pub trait MoveRules {
    fn max_stack(&self, item: &ItemStack) -> u16;
    fn accepts(&self, slot: InventorySlot, item: &ItemStack) -> bool;
}

fn merged_count(inventory: &PlayerInventory, destination: InventorySlot, moving: &ItemStack, data: &[u8], amount: u16, rules: &dyn MoveRules) -> Option<u16> {
    let present = inventory.stack_at(destination)?;
    if present.is_empty() {
        return Some(amount);
    }
    let fits = present.is_same(moving) && inventory.data_at(destination) == data && present.count + amount <= rules.max_stack(moving);
    fits.then_some(present.count + amount)
}

pub fn move_items(inventory: &mut PlayerInventory, source: InventorySlot, destination: InventorySlot, amount: u16, rules: &dyn MoveRules) -> bool {
    let Some(from) = inventory.stack_at(source) else { return false };
    if source == destination || amount == 0 || from.is_empty() || from.count < amount || !rules.accepts(destination, &from) {
        return false;
    }
    let data = inventory.data_at(source);
    let Some(new_count) = merged_count(inventory, destination, &from, &data, amount, rules) else {
        return false;
    };

    let remaining = from.count - amount;
    let left = if remaining == 0 { ItemStack::air() } else { ItemStack { count: remaining, ..from } };
    let left_data = if remaining == 0 { Vec::new() } else { data.clone() };

    let moved = ItemStack { count: new_count, ..from };
    inventory.put(destination, moved, data) && inventory.put(source, left, left_data)
}

pub fn swap_slots(inventory: &mut PlayerInventory, first: InventorySlot, second: InventorySlot, rules: &dyn MoveRules) -> bool {
    let (Some(a), Some(b)) = (inventory.stack_at(first), inventory.stack_at(second)) else {
        return false;
    };
    if first == second || (!a.is_empty() && !rules.accepts(second, &a)) || (!b.is_empty() && !rules.accepts(first, &b)) {
        return false;
    }
    let (a_data, b_data) = (inventory.data_at(first), inventory.data_at(second));
    inventory.put(first, b, b_data) && inventory.put(second, a, a_data)
}

pub fn take_items(inventory: &mut PlayerInventory, source: InventorySlot, amount: u16) -> Option<(ItemStack, Vec<u8>)> {
    let from = inventory.stack_at(source)?;
    if amount == 0 || from.is_empty() || from.count < amount {
        return None;
    }
    let data = inventory.data_at(source);
    let remaining = from.count - amount;
    let left = if remaining == 0 { ItemStack::air() } else { ItemStack { count: remaining, ..from } };
    let left_data = if remaining == 0 { Vec::new() } else { data.clone() };
    inventory.put(source, left, left_data).then_some((ItemStack { count: amount, ..from }, data))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Rules;

    impl MoveRules for Rules {
        fn max_stack(&self, item: &ItemStack) -> u16 {
            if item.id == 1 { 1 } else { 64 }
        }

        fn accepts(&self, slot: InventorySlot, item: &ItemStack) -> bool {
            match slot {
                InventorySlot::Armor(0) => item.id == 7,
                InventorySlot::Armor(_) => false,
                _ => true,
            }
        }
    }

    fn stack(id: i16, count: u16) -> ItemStack {
        ItemStack { id, count, meta: 0, block_runtime_id: 0 }
    }

    fn inventory() -> PlayerInventory {
        PlayerInventory::new()
    }

    fn legacy(endpoint: LegacyEndpoint, from: ItemStack, to: ItemStack) -> LegacyAction {
        LegacyAction { endpoint, from, to, to_data: Vec::new() }
    }

    fn slot(index: usize) -> LegacyEndpoint {
        LegacyEndpoint::Slot(InventorySlot::Main(index))
    }

    #[test]
    fn picking_a_stack_up_onto_the_cursor_is_applied() {
        let mut inv = inventory();
        inv.put(InventorySlot::Main(0), stack(3, 10), vec![]);
        let actions = [
            legacy(slot(0), stack(3, 10), ItemStack::air()),
            legacy(LegacyEndpoint::Slot(InventorySlot::Cursor), ItemStack::air(), stack(3, 10)),
        ];
        assert_eq!(apply_legacy_actions(&mut inv, &actions, &Rules, false), LegacyOutcome::Applied { dropped: vec![] });
        assert!(inv.stack_at(InventorySlot::Main(0)).unwrap().is_empty());
        assert_eq!(inv.cursor().count, 10);
    }

    #[test]
    fn a_swap_of_two_slots_is_applied_with_the_reported_data() {
        let mut inv = inventory();
        inv.put(InventorySlot::Main(0), stack(3, 1), vec![]);
        inv.put(InventorySlot::Main(1), stack(4, 1), vec![]);
        let mut first = legacy(slot(0), stack(3, 1), stack(4, 1));
        first.to_data = vec![7];
        let second = legacy(slot(1), stack(4, 1), stack(3, 1));
        assert!(matches!(apply_legacy_actions(&mut inv, &[first, second], &Rules, false), LegacyOutcome::Applied { .. }));
        assert_eq!(inv.stack_at(InventorySlot::Main(0)).unwrap().id, 4);
        assert_eq!(inv.data_at(InventorySlot::Main(0)), vec![7]);
        assert_eq!(inv.stack_at(InventorySlot::Main(1)).unwrap().id, 3);
    }

    #[test]
    fn a_drop_takes_the_items_out_and_reports_them() {
        let mut inv = inventory();
        inv.put(InventorySlot::Main(2), stack(3, 6), vec![]);
        let actions = [legacy(slot(2), stack(3, 6), stack(3, 4)), legacy(LegacyEndpoint::Drop, ItemStack::air(), stack(3, 2))];
        assert_eq!(apply_legacy_actions(&mut inv, &actions, &Rules, false), LegacyOutcome::Applied { dropped: vec![stack(3, 2)] });
        assert_eq!(inv.stack_at(InventorySlot::Main(2)).unwrap().count, 4);
    }

    #[test]
    fn a_transaction_that_does_not_match_the_server_changes_nothing() {
        let mut inv = inventory();
        inv.put(InventorySlot::Main(0), stack(3, 5), vec![]);
        let actions = [legacy(slot(0), stack(3, 9), ItemStack::air()), legacy(slot(1), ItemStack::air(), stack(3, 9))];
        assert_eq!(apply_legacy_actions(&mut inv, &actions, &Rules, false), LegacyOutcome::Rejected);
        assert_eq!(inv.stack_at(InventorySlot::Main(0)).unwrap().count, 5);
        assert!(inv.stack_at(InventorySlot::Main(1)).unwrap().is_empty());
    }

    #[test]
    fn items_cannot_be_created_or_lost() {
        let mut inv = inventory();
        inv.put(InventorySlot::Main(0), stack(3, 5), vec![]);
        let duplicated = [legacy(slot(0), stack(3, 5), stack(3, 5)), legacy(slot(1), ItemStack::air(), stack(3, 5))];
        assert_eq!(apply_legacy_actions(&mut inv, &duplicated, &Rules, false), LegacyOutcome::Rejected);
        let lost = [legacy(slot(0), stack(3, 5), ItemStack::air())];
        assert_eq!(apply_legacy_actions(&mut inv, &lost, &Rules, false), LegacyOutcome::Rejected);
        assert_eq!(inv.stack_at(InventorySlot::Main(0)).unwrap().count, 5);
    }

    #[test]
    fn only_creative_players_may_take_items_from_the_creative_list() {
        let mut inv = inventory();
        let actions = [legacy(LegacyEndpoint::Creative, stack(3, 1), stack(3, 1)), legacy(slot(0), ItemStack::air(), stack(3, 1))];
        assert_eq!(apply_legacy_actions(&mut inv, &actions, &Rules, false), LegacyOutcome::Rejected);
        assert!(matches!(apply_legacy_actions(&mut inv, &actions, &Rules, true), LegacyOutcome::Applied { .. }));
        assert_eq!(inv.stack_at(InventorySlot::Main(0)).unwrap().id, 3);
    }

    #[test]
    fn equipping_armour_through_a_legacy_transaction_checks_the_slot() {
        let mut inv = inventory();
        inv.put(InventorySlot::Main(0), stack(7, 1), vec![]);
        inv.put(InventorySlot::Main(1), stack(3, 1), vec![]);
        let good = [legacy(slot(0), stack(7, 1), ItemStack::air()), legacy(LegacyEndpoint::Slot(InventorySlot::Armor(0)), ItemStack::air(), stack(7, 1))];
        assert!(matches!(apply_legacy_actions(&mut inv, &good, &Rules, false), LegacyOutcome::Applied { .. }));
        let bad = [legacy(slot(1), stack(3, 1), ItemStack::air()), legacy(LegacyEndpoint::Slot(InventorySlot::Armor(0)), stack(7, 1), stack(3, 1))];
        assert_eq!(apply_legacy_actions(&mut inv, &bad, &Rules, false), LegacyOutcome::Rejected);
    }

    #[test]
    fn items_move_between_the_inventory_and_an_open_window() {
        let mut inv = inventory();
        inv.open_window(5, 27);
        inv.put(InventorySlot::Window(3), stack(3, 4), vec![8]);
        assert!(move_items(&mut inv, InventorySlot::Window(3), InventorySlot::Main(0), 4, &Rules));
        assert_eq!(inv.stack_at(InventorySlot::Main(0)).unwrap().count, 4);
        assert_eq!(inv.data_at(InventorySlot::Main(0)), vec![8]);
        assert!(inv.stack_at(InventorySlot::Window(3)).unwrap().is_empty());
        assert!(inv.stack_at(InventorySlot::Window(40)).is_none());
    }

    #[test]
    fn without_an_open_window_there_are_no_window_slots() {
        let mut inv = inventory();
        assert!(inv.stack_at(InventorySlot::Window(0)).is_none());
        assert!(!inv.put(InventorySlot::Window(0), stack(3, 1), vec![]));
        inv.open_window(2, 27);
        assert!(inv.put(InventorySlot::Window(0), stack(3, 1), vec![]));
        inv.close_window();
        assert!(inv.stack_at(InventorySlot::Window(0)).is_none());
        assert_eq!(inv.window_id(), None);
    }

    #[test]
    fn armour_belongs_in_the_matching_slot() {
        assert!(armor_slot_accepts(0, "minecraft:iron_helmet"));
        assert!(armor_slot_accepts(0, "minecraft:carved_pumpkin"));
        assert!(armor_slot_accepts(1, "minecraft:elytra"));
        assert!(armor_slot_accepts(2, "minecraft:diamond_leggings"));
        assert!(armor_slot_accepts(3, "minecraft:netherite_boots"));
        assert!(!armor_slot_accepts(0, "minecraft:iron_boots"));
        assert!(!armor_slot_accepts(1, "minecraft:iron_sword"));
        assert!(!armor_slot_accepts(4, "minecraft:iron_helmet"));
    }

    #[test]
    fn a_whole_stack_moves_to_an_empty_slot() {
        let mut inv = inventory();
        inv.put(InventorySlot::Main(0), stack(3, 10), vec![]);
        assert!(move_items(&mut inv, InventorySlot::Main(0), InventorySlot::Main(5), 10, &Rules));
        assert!(inv.stack_at(InventorySlot::Main(0)).unwrap().is_empty());
        assert_eq!(inv.stack_at(InventorySlot::Main(5)).unwrap().count, 10);
    }

    #[test]
    fn half_a_stack_goes_to_the_cursor_and_back() {
        let mut inv = inventory();
        inv.put(InventorySlot::Main(0), stack(3, 10), vec![]);
        assert!(move_items(&mut inv, InventorySlot::Main(0), InventorySlot::Cursor, 5, &Rules));
        assert_eq!(inv.stack_at(InventorySlot::Main(0)).unwrap().count, 5);
        assert_eq!(inv.cursor().count, 5);
        assert!(move_items(&mut inv, InventorySlot::Cursor, InventorySlot::Main(0), 5, &Rules));
        assert_eq!(inv.stack_at(InventorySlot::Main(0)).unwrap().count, 10);
        assert!(inv.cursor().is_empty());
    }

    #[test]
    fn merging_never_passes_the_stack_limit() {
        let mut inv = inventory();
        inv.put(InventorySlot::Main(0), stack(3, 60), vec![]);
        inv.put(InventorySlot::Main(1), stack(3, 10), vec![]);
        assert!(!move_items(&mut inv, InventorySlot::Main(1), InventorySlot::Main(0), 10, &Rules));
        assert_eq!(inv.stack_at(InventorySlot::Main(0)).unwrap().count, 60);
        assert_eq!(inv.stack_at(InventorySlot::Main(1)).unwrap().count, 10);
    }

    #[test]
    fn single_items_do_not_merge() {
        let mut inv = inventory();
        inv.put(InventorySlot::Main(0), stack(1, 1), vec![]);
        inv.put(InventorySlot::Main(1), stack(1, 1), vec![]);
        assert!(!move_items(&mut inv, InventorySlot::Main(1), InventorySlot::Main(0), 1, &Rules));
    }

    #[test]
    fn a_different_item_is_not_moved_over_another() {
        let mut inv = inventory();
        inv.put(InventorySlot::Main(0), stack(3, 2), vec![]);
        inv.put(InventorySlot::Main(1), stack(4, 2), vec![]);
        assert!(!move_items(&mut inv, InventorySlot::Main(1), InventorySlot::Main(0), 2, &Rules));
    }

    #[test]
    fn armour_only_goes_in_its_own_slot() {
        let mut inv = inventory();
        inv.put(InventorySlot::Main(0), stack(7, 1), vec![]);
        inv.put(InventorySlot::Main(1), stack(3, 1), vec![]);
        assert!(!move_items(&mut inv, InventorySlot::Main(1), InventorySlot::Armor(0), 1, &Rules));
        assert!(!move_items(&mut inv, InventorySlot::Main(0), InventorySlot::Armor(1), 1, &Rules));
        assert!(move_items(&mut inv, InventorySlot::Main(0), InventorySlot::Armor(0), 1, &Rules));
        assert_eq!(inv.stack_at(InventorySlot::Armor(0)).unwrap().id, 7);
    }

    #[test]
    fn swapping_exchanges_stacks_and_their_data() {
        let mut inv = inventory();
        inv.put(InventorySlot::Main(0), stack(3, 2), vec![1, 2, 3]);
        inv.put(InventorySlot::Main(1), stack(4, 5), vec![]);
        assert!(swap_slots(&mut inv, InventorySlot::Main(0), InventorySlot::Main(1), &Rules));
        assert_eq!(inv.stack_at(InventorySlot::Main(0)).unwrap().id, 4);
        assert_eq!(inv.stack_at(InventorySlot::Main(1)).unwrap().id, 3);
        assert_eq!(inv.data_at(InventorySlot::Main(1)), vec![1, 2, 3]);
        assert!(inv.data_at(InventorySlot::Main(0)).is_empty());
    }

    #[test]
    fn a_swap_into_a_slot_that_refuses_the_item_changes_nothing() {
        let mut inv = inventory();
        inv.put(InventorySlot::Main(0), stack(3, 1), vec![]);
        inv.put(InventorySlot::Armor(0), stack(7, 1), vec![]);
        assert!(!swap_slots(&mut inv, InventorySlot::Main(0), InventorySlot::Armor(0), &Rules));
        assert_eq!(inv.stack_at(InventorySlot::Armor(0)).unwrap().id, 7);
    }

    #[test]
    fn user_data_follows_the_item_when_it_moves() {
        let mut inv = inventory();
        inv.put(InventorySlot::Main(0), stack(3, 1), vec![9, 9]);
        assert!(move_items(&mut inv, InventorySlot::Main(0), InventorySlot::Cursor, 1, &Rules));
        assert_eq!(inv.data_at(InventorySlot::Cursor), vec![9, 9]);
        assert!(move_items(&mut inv, InventorySlot::Cursor, InventorySlot::Main(7), 1, &Rules));
        assert_eq!(inv.data_at(InventorySlot::Main(7)), vec![9, 9]);
        assert!(inv.data_at(InventorySlot::Main(0)).is_empty());
    }

    #[test]
    fn taking_more_than_there_is_fails() {
        let mut inv = inventory();
        inv.put(InventorySlot::Main(0), stack(3, 2), vec![]);
        assert!(take_items(&mut inv, InventorySlot::Main(0), 3).is_none());
        let (taken, _) = take_items(&mut inv, InventorySlot::Main(0), 2).unwrap();
        assert_eq!(taken.count, 2);
        assert!(inv.stack_at(InventorySlot::Main(0)).unwrap().is_empty());
    }
}
