use crate::entity::entity::Entity;
use crate::entity::entity_id;
use crate::entity::entity_mob::EntityMob;
use bevy_ecs::prelude::Component;

#[derive(Component)]
pub struct Player {}

impl Player {
    pub fn new() -> (Entity, EntityMob, Player) {
        let entity = Entity::default(entity_id::PLAYER.to_string(), rand::random::<i64>());
        let entity_mob = EntityMob::default();
        let player = Player {};

        (entity, entity_mob, player)
    }
}
