use crate::components::ageable::Ageable;
use crate::components::breedable::Breedable;
use crate::entity::Entity;
use crate::entity_id;
use crate::entity_mob::EntityMob;
use bevy_ecs::prelude::Component;

#[derive(Component)]
pub struct Pig;

impl Pig {
    pub fn new() -> (Entity, EntityMob, Pig, Ageable, Breedable) {
        let entity = Entity::default(entity_id::PIG.to_string());
        let entity_mob = EntityMob::default();
        let pig = Pig {};
        let ageable = Ageable::default();
        let breedable = Breedable::default();

        (entity, entity_mob, pig, ageable, breedable)
    }
}
