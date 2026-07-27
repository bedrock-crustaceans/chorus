use crate::item::item_stack::ItemStack;
use bevy_ecs::prelude::Component;
use vek::Vec3;

pub const PICKUP_DELAY_TICKS: u32 = 10;
pub const PICKUP_RADIUS: f32 = 1.75;

#[derive(Component)]
pub struct ItemEntity {
    unique_id: i64,
    runtime_id: u64,
    position: Vec3<f32>,
    stack: ItemStack,
    pickup_delay: u32,
}

impl ItemEntity {
    pub fn new(unique_id: i64, runtime_id: u64, position: Vec3<f32>, stack: ItemStack) -> Self {
        Self {
            unique_id,
            runtime_id,
            position,
            stack,
            pickup_delay: PICKUP_DELAY_TICKS,
        }
    }

    pub fn unique_id(&self) -> i64 {
        self.unique_id
    }

    pub fn runtime_id(&self) -> u64 {
        self.runtime_id
    }

    pub fn position(&self) -> Vec3<f32> {
        self.position
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

    pub fn is_within_reach(&self, target: Vec3<f32>) -> bool {
        self.position.distance(target) <= PICKUP_RADIUS
    }
}
