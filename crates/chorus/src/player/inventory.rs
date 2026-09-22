use crate::item::item_stack::ItemStack;

pub const MAIN_SIZE: usize = 36;
pub const HOTBAR_SIZE: usize = 9;
pub const OFFHAND_SIZE: usize = 1;
pub const ARMOR_SIZE: usize = 4;

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

    pub fn swap(&mut self, from: usize, to: usize) -> bool {
        if from >= self.slots.len() || to >= self.slots.len() {
            return false;
        }

        self.slots.swap(from, to);
        true
    }
}

pub struct PlayerInventory {
    main: Inventory,
    offhand: Inventory,
    armor: Inventory,

    held_slot: u8,
    next_stack_id: i32,
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

            held_slot: 0,
            next_stack_id: 0,
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

    pub fn armor(&self) -> &Inventory {
        &self.armor
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
}
