use crate::item::item_stack::ItemStack;
use bedrock::protocol::v662::enums::ContainerID;
use bevy_ecs::prelude::Component;
use std::collections::HashMap;

pub const MAIN_SIZE: usize = 36;
pub const HOTBAR_SIZE: usize = 9;
pub const OFFHAND_SIZE: usize = 1;
pub const ARMOR_SIZE: usize = 4;
pub const MAX_STACK_SIZE: u16 = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InventorySlot {
    Main(usize),
    Armor(usize),
    Offhand(usize),
    Cursor,
    Window(usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotUserData {
    pub id: i16,
    pub meta: u32,
    pub data: Vec<u8>,
}

fn store_user_data(map: &mut HashMap<usize, SlotUserData>, slot: usize, item: Option<&ItemStack>, data: Vec<u8>) {
    match item {
        Some(item) if !item.is_empty() && !data.is_empty() => {
            map.insert(slot, SlotUserData { id: item.id, meta: item.meta, data });
        }
        _ => {
            map.remove(&slot);
        }
    }
}

fn slot_user_data<'a>(map: &'a HashMap<usize, SlotUserData>, slot: usize, item: Option<&ItemStack>) -> Option<&'a [u8]> {
    let item = item.filter(|item| !item.is_empty())?;
    let entry = map.get(&slot)?;
    (entry.id == item.id && entry.meta == item.meta).then_some(entry.data.as_slice())
}

#[derive(Clone)]
pub struct Inventory {
    slots: Vec<ItemStack>,
}

impl Inventory {
    pub fn new(size: usize) -> Self {
        Self { slots: vec![ItemStack::air(); size] }
    }

    pub fn size(&self) -> usize {
        self.slots.len()
    }

    pub fn slots(&self) -> &[ItemStack] {
        &self.slots
    }

    pub fn get(&self, slot: usize) -> Option<&ItemStack> {
        self.slots.get(slot)
    }

    pub fn set(&mut self, slot: usize, item: ItemStack) -> bool {
        match self.slots.get_mut(slot) {
            Some(existing) => {
                *existing = item;
                true
            }
            None => false,
        }
    }

    /// Finds the first slot holding the same item, ignoring how many are stacked in it.
    pub fn first(&self, item: &ItemStack) -> Option<usize> {
        self.slots.iter().position(|slot| slot.is_same(item))
    }

    pub fn first_empty(&self) -> Option<usize> {
        self.slots.iter().position(ItemStack::is_empty)
    }

    pub fn first_with_room(&self, item: &ItemStack, max_stack: u16) -> Option<usize> {
        self.slots
            .iter()
            .position(|slot| slot.is_same(item) && slot.count < max_stack)
            .or_else(|| self.first_empty())
    }

    pub fn add(&mut self, item: ItemStack, max_stack: u16) -> bool {
        let max_stack = max_stack.clamp(1, MAX_STACK_SIZE);
        let free: u32 = self
            .slots
            .iter()
            .map(|slot| match slot {
                slot if slot.is_empty() => max_stack as u32,
                slot if slot.is_same(&item) => max_stack.saturating_sub(slot.count) as u32,
                _ => 0,
            })
            .sum();
        if free < item.count as u32 {
            return false;
        }

        let mut remaining = item.count;
        for slot in self.slots.iter_mut().filter(|slot| !slot.is_empty() && slot.is_same(&item)) {
            let moved = max_stack.saturating_sub(slot.count).min(remaining);
            slot.count += moved;
            remaining -= moved;
        }

        for slot in self.slots.iter_mut().filter(|slot| slot.is_empty()) {
            if remaining == 0 {
                break;
            }

            let moved = remaining.min(max_stack);
            *slot = ItemStack { count: moved, ..item };
            remaining -= moved;
        }

        true
    }

    pub fn swap(&mut self, from: usize, to: usize) -> bool {
        if from >= self.slots.len() || to >= self.slots.len() {
            return false;
        }

        self.slots.swap(from, to);
        true
    }
}

#[derive(Clone)]
struct OpenWindow {
    id: i8,
    slots: Inventory,
    user_data: HashMap<usize, SlotUserData>,
}

#[derive(Component, Clone)]
pub struct PlayerInventory {
    main: Inventory,
    offhand: Inventory,
    armor: Inventory,
    cursor: ItemStack,
    cursor_user_data: Vec<u8>,
    window: Option<OpenWindow>,

    held_slot: u8,
    next_stack_id: i32,
    main_user_data: HashMap<usize, SlotUserData>,
    offhand_user_data: HashMap<usize, SlotUserData>,
    armor_user_data: HashMap<usize, SlotUserData>,
}

impl Default for PlayerInventory {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayerInventory {
    pub fn new() -> Self {
        Self {
            main: Inventory::new(MAIN_SIZE),
            offhand: Inventory::new(OFFHAND_SIZE),
            armor: Inventory::new(ARMOR_SIZE),
            cursor: ItemStack::air(),
            cursor_user_data: Vec::new(),
            window: None,

            held_slot: 0,
            next_stack_id: 0,
            main_user_data: HashMap::new(),
            offhand_user_data: HashMap::new(),
            armor_user_data: HashMap::new(),
        }
    }

    pub fn main(&self) -> &Inventory {
        &self.main
    }

    pub fn main_mut(&mut self) -> &mut Inventory {
        &mut self.main
    }

    pub fn offhand(&self) -> &Inventory {
        &self.offhand
    }

    pub fn offhand_mut(&mut self) -> &mut Inventory {
        &mut self.offhand
    }

    pub fn armor(&self) -> &Inventory {
        &self.armor
    }

    pub fn armor_mut(&mut self) -> &mut Inventory {
        &mut self.armor
    }

    pub fn cursor(&self) -> &ItemStack {
        &self.cursor
    }

    pub fn open_window(&mut self, id: i8, size: usize) {
        self.window = Some(OpenWindow {
            id,
            slots: Inventory::new(size),
            user_data: HashMap::new(),
        });
    }

    pub fn close_window(&mut self) {
        self.window = None;
    }

    pub fn window_id(&self) -> Option<i8> {
        self.window.as_ref().map(|window| window.id)
    }

    pub fn window(&self) -> Option<&Inventory> {
        self.window.as_ref().map(|window| &window.slots)
    }

    pub fn window_user_data(&self, slot: usize) -> Option<&[u8]> {
        let window = self.window.as_ref()?;
        slot_user_data(&window.user_data, slot, window.slots.get(slot))
    }

    pub fn stack_at(&self, slot: InventorySlot) -> Option<ItemStack> {
        match slot {
            InventorySlot::Main(index) => self.main.get(index).copied(),
            InventorySlot::Armor(index) => self.armor.get(index).copied(),
            InventorySlot::Offhand(index) => self.offhand.get(index).copied(),
            InventorySlot::Cursor => Some(self.cursor),
            InventorySlot::Window(index) => self.window.as_ref()?.slots.get(index).copied(),
        }
    }

    pub fn data_at(&self, slot: InventorySlot) -> Vec<u8> {
        match slot {
            InventorySlot::Main(index) => self.main_user_data(index),
            InventorySlot::Armor(index) => self.armor_user_data(index),
            InventorySlot::Offhand(index) => self.offhand_user_data(index),
            InventorySlot::Cursor => (!self.cursor.is_empty()).then_some(self.cursor_user_data.as_slice()),
            InventorySlot::Window(index) => self.window_user_data(index),
        }
        .map(<[u8]>::to_vec)
        .unwrap_or_default()
    }

    pub fn put(&mut self, slot: InventorySlot, stack: ItemStack, data: Vec<u8>) -> bool {
        match slot {
            InventorySlot::Main(index) => {
                if !self.main.set(index, stack) {
                    return false;
                }
                self.set_main_user_data(index, data);
            }
            InventorySlot::Armor(index) => {
                if !self.armor.set(index, stack) {
                    return false;
                }
                self.set_armor_user_data(index, data);
            }
            InventorySlot::Offhand(index) => {
                if !self.offhand.set(index, stack) {
                    return false;
                }
                self.set_offhand_user_data(index, data);
            }
            InventorySlot::Cursor => {
                self.cursor_user_data = if stack.is_empty() { Vec::new() } else { data };
                self.cursor = stack;
            }
            InventorySlot::Window(index) => {
                let Some(window) = self.window.as_mut() else {
                    return false;
                };
                if !window.slots.set(index, stack) {
                    return false;
                }
                let item = window.slots.get(index).copied();
                store_user_data(&mut window.user_data, index, item.as_ref(), data);
            }
        }
        true
    }

    pub fn held_slot(&self) -> u8 {
        self.held_slot
    }

    pub fn set_held_slot(&mut self, slot: u8) -> bool {
        if slot as usize >= HOTBAR_SIZE {
            return false;
        }

        self.held_slot = slot;
        true
    }

    pub fn held_item(&self) -> &ItemStack {
        self.main.get(self.held_slot as usize).unwrap_or(const { &ItemStack::air() })
    }

    /// Equips the item the player picked: an existing stack is moved into the hand, otherwise the
    /// item is put in the first free slot. `allow_new` is false for players with finite resources,
    /// they may only pick what they already carry. Returns whether anything changed.
    pub fn pick_item(&mut self, item: ItemStack, allow_new: bool) -> bool {
        if let Some(slot) = self.main.first(&item) {
            return if slot < HOTBAR_SIZE {
                self.set_held_slot(slot as u8)
            } else {
                self.main.swap(self.held_slot as usize, slot)
            };
        }

        if !allow_new {
            return false;
        }

        match self.main.first_empty() {
            None => self.main.set(self.held_slot as usize, item),
            Some(slot) if slot < HOTBAR_SIZE => self.main.set(slot, item) && self.set_held_slot(slot as u8),
            // the hand is emptied into the free slot so nothing is lost
            Some(slot) => self.main.swap(self.held_slot as usize, slot) && self.main.set(self.held_slot as usize, item),
        }
    }

    /// Stack ids start at 1 - zero is reserved for empty slots.
    pub fn next_stack_id(&mut self) -> i32 {
        self.next_stack_id += 1;
        self.next_stack_id
    }

    pub fn set_main_user_data(&mut self, slot: usize, data: Vec<u8>) {
        let item = self.main.get(slot).copied();
        store_user_data(&mut self.main_user_data, slot, item.as_ref(), data);
    }

    pub fn set_offhand_user_data(&mut self, slot: usize, data: Vec<u8>) {
        let item = self.offhand.get(slot).copied();
        store_user_data(&mut self.offhand_user_data, slot, item.as_ref(), data);
    }

    pub fn set_armor_user_data(&mut self, slot: usize, data: Vec<u8>) {
        let item = self.armor.get(slot).copied();
        store_user_data(&mut self.armor_user_data, slot, item.as_ref(), data);
    }

    pub fn main_user_data(&self, slot: usize) -> Option<&[u8]> {
        slot_user_data(&self.main_user_data, slot, self.main.get(slot))
    }

    pub fn offhand_user_data(&self, slot: usize) -> Option<&[u8]> {
        slot_user_data(&self.offhand_user_data, slot, self.offhand.get(slot))
    }

    pub fn armor_user_data(&self, slot: usize) -> Option<&[u8]> {
        slot_user_data(&self.armor_user_data, slot, self.armor.get(slot))
    }

    pub fn user_data(&self, container: ContainerID, slot: usize) -> Option<&[u8]> {
        match container {
            ContainerID::ARMOR => self.armor_user_data(slot),
            ContainerID::OFFHAND => self.offhand_user_data(slot),
            ContainerID(id) if self.window_id() == Some(id) => self.window_user_data(slot),
            _ => self.main_user_data(slot),
        }
    }

    pub fn clear_user_data(&mut self) {
        self.main_user_data.clear();
        self.offhand_user_data.clear();
        self.armor_user_data.clear();
        self.cursor_user_data.clear();
        if let Some(window) = self.window.as_mut() {
            window.user_data.clear();
        }
    }
}

#[cfg(test)]
mod stacking_tests {
    use super::*;

    fn stack(id: i16, count: u16) -> ItemStack {
        ItemStack { id, count, meta: 0, block_runtime_id: 0 }
    }

    #[test]
    fn single_item_stacks_go_into_separate_slots() {
        let mut inventory = Inventory::new(4);
        assert!(inventory.add(stack(7, 1), 1));
        assert!(inventory.add(stack(7, 1), 1));
        assert_eq!(inventory.get(0).unwrap().count, 1);
        assert_eq!(inventory.get(1).unwrap().count, 1);
    }

    #[test]
    fn a_full_inventory_of_single_items_takes_no_more() {
        let mut inventory = Inventory::new(2);
        assert!(inventory.add(stack(7, 1), 1));
        assert!(inventory.add(stack(7, 1), 1));
        assert!(!inventory.add(stack(7, 1), 1));
    }

    #[test]
    fn sixteen_stacks_split_at_sixteen() {
        let mut inventory = Inventory::new(3);
        assert!(inventory.add(stack(9, 20), 16));
        assert_eq!(inventory.get(0).unwrap().count, 16);
        assert_eq!(inventory.get(1).unwrap().count, 4);
    }

    #[test]
    fn normal_items_still_stack_to_sixty_four() {
        let mut inventory = Inventory::new(2);
        assert!(inventory.add(stack(3, 40), 64));
        assert!(inventory.add(stack(3, 40), 64));
        assert_eq!(inventory.get(0).unwrap().count, 64);
        assert_eq!(inventory.get(1).unwrap().count, 16);
    }

    #[test]
    fn first_with_room_skips_full_stacks() {
        let mut inventory = Inventory::new(3);
        inventory.set(0, stack(7, 1));
        assert_eq!(inventory.first_with_room(&stack(7, 1), 1), Some(1));
        assert_eq!(inventory.first_with_room(&stack(7, 1), 64), Some(0));
    }
}
