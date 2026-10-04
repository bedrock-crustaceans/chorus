use crate::item_stack::ItemStack;
use bevy_ecs::prelude::Component;
use glam::Vec3;

pub const PICKUP_DELAY_TICKS: u32 = 10;
pub const PICKUP_RADIUS: f32 = 1.75;
pub const DESPAWN_TICKS: u32 = 6000;

#[derive(Component)]
pub struct ItemEntity {
    stack: ItemStack,
    pickup_delay: u32,
    age: u32,
    pub on_ground: bool,
}

impl ItemEntity {
    pub fn new(stack: ItemStack) -> Self {
        Self {
            stack,
            pickup_delay: PICKUP_DELAY_TICKS,
            age: 0,
            on_ground: false,
        }
    }

    pub fn absorb(&mut self, other: &mut ItemEntity, max: u16) -> u16 {
        if !self.stack.is_same(&other.stack) {
            return 0;
        }
        let moved = max.saturating_sub(self.stack.count).min(other.stack.count);
        if moved == 0 {
            return 0;
        }
        self.stack.count += moved;
        other.stack.count -= moved;
        self.age = self.age.min(other.age);
        self.pickup_delay = self.pickup_delay.max(other.pickup_delay);
        moved
    }

    pub fn tick_age(&mut self) -> bool {
        self.age += 1;
        self.age >= DESPAWN_TICKS
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

#[cfg(test)]
mod tests {
    use super::*;

    fn stack(id: i16, count: u16) -> ItemStack {
        ItemStack { id, count, ..ItemStack::default() }
    }

    #[test]
    fn absorb_moves_what_fits_and_keeps_the_younger_age() {
        let mut target = ItemEntity::new(stack(1, 40));
        let mut source = ItemEntity::new(stack(1, 40));
        target.age = 100;
        source.age = 20;
        source.pickup_delay = 30;

        assert_eq!(target.absorb(&mut source, 64), 24);
        assert_eq!((target.stack().count, source.stack().count), (64, 16));
        assert_eq!((target.age, target.pickup_delay), (20, 30));
        assert_eq!(target.absorb(&mut source, 64), 0, "a full stack takes nothing more");
    }

    #[test]
    fn absorb_ignores_different_items() {
        let mut target = ItemEntity::new(stack(1, 1));
        let mut source = ItemEntity::new(stack(2, 1));
        assert_eq!(target.absorb(&mut source, 64), 0);
        assert_eq!((target.stack().count, source.stack().count), (1, 1));
    }
}
