use crate::item::item_stack::ItemStack;
use bevy_ecs::prelude::Component;
use glam::Vec3;

pub const PICKUP_DELAY_TICKS: u32 = 10;
pub const PICKUP_RADIUS: f32 = 1.75;

#[derive(Component)]
pub struct ItemEntity {
    stack: ItemStack,
    pickup_delay: u32,
}

impl ItemEntity {
    pub fn new(stack: ItemStack) -> Self {
        Self {
            stack,
            pickup_delay: PICKUP_DELAY_TICKS,
        }
    }

    pub fn stack(&self) -> ItemStack {
        self.stack
    }

    pub fn tick_pickup_delay(&mut self) {
        self.pickup_delay = self.pickup_delay.saturating_sub(1);
    }

    pub fn can_be_picked_up(&self) -> bool {
        self.pickup_delay == 0
    }
}

pub fn within_pickup_reach(item: Vec3, player: Vec3) -> bool {
    item.distance_squared(player) <= PICKUP_RADIUS * PICKUP_RADIUS
}
