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

    /// Stack ids start at 1 - zero is reserved for empty slots.
    pub fn next_stack_id(&mut self) -> i32 {
        self.next_stack_id += 1;
        self.next_stack_id
    }
}
