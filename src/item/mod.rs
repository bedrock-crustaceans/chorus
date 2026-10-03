pub mod item_entity;
pub mod item_stack;

use bevy_ecs::prelude::{Entity, Message};

#[derive(Message, Clone, Debug)]
pub struct ItemTakenMessage {
    pub item_entity: Entity,
    pub dimension_id: i32,
    pub item_unique_id: i64,
    pub item_runtime_id: u64,
    pub player_runtime_id: u64,
}
