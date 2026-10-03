use bevy_ecs::prelude::Component;
use std::collections::HashMap;

#[derive(Component)]
pub struct EntityHumanoidMonster {
    // NBT fields
    item_in_hand: Option<HashMap<String, nbtx::Value>>,
}
