use crate::entity::components::ageable::Ageable;
use crate::entity::components::breedable::Breedable;
use crate::entity::entity::Entity;
use crate::entity::entity_id;
use crate::entity::entity_mob::EntityMob;
use bevy_ecs::prelude::Component;

#[derive(Component)]
pub struct Pig;

impl Pig {
    pub fn new() -> (Entity, EntityMob, Pig, Ageable, Breedable) {
        let entity = Entity::default(entity_id::PIG.to_string(), rand::random::<i64>());
        let entity_mob = EntityMob::default();
        let pig = Pig {};
        let ageable = Ageable::default();
        let breedable = Breedable::default();

        (entity, entity_mob, pig, ageable, breedable)
    }
}
