use crate::item::item_stack::{ItemStack, MAX_STACK_SIZE};
use bevy_ecs::prelude::Component;

pub const INVENTORY_SIZE: usize = 36;

#[derive(Component)]
pub struct Inventory {
    slots: Vec<Option<ItemStack>>,
}

impl Default for Inventory {
    fn default() -> Self {
        Self::new(INVENTORY_SIZE)
    }
}

impl Inventory {
    pub fn new(size: usize) -> Self {
        Self { slots: vec![None; size] }
    }

    pub fn size(&self) -> usize {
        self.slots.len()
    }

    pub fn slots(&self) -> &[Option<ItemStack>] {
        &self.slots
    }

    pub fn add(&mut self, stack: ItemStack) -> bool {
        let mut remaining = stack.count();

        for slot in self.slots.iter_mut() {
            if remaining == 0 {
                break;
            }
            if let Some(existing) = slot
                && existing.is_same_kind(&stack)
                && existing.count() < MAX_STACK_SIZE
            {
                let moved = (MAX_STACK_SIZE - existing.count()).min(remaining);
                existing.set_count(existing.count() + moved);
                remaining -= moved;
            }
        }

        for slot in self.slots.iter_mut() {
            if remaining == 0 {
                break;
            }
            if slot.is_none() {
                let moved = remaining.min(MAX_STACK_SIZE);
                *slot = Some(ItemStack::new(stack.block_id(), moved));
                remaining -= moved;
            }
        }

        remaining == 0
    }
}
