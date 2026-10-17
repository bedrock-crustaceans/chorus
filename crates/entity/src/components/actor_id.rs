use bevy_ecs::entity::Entity;
use bevy_ecs::lifecycle::{Add, Remove};
use bevy_ecs::observer::On;
use bevy_ecs::prelude::{Component, Query, ResMut, Resource};
use std::collections::HashMap;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActorId {
    pub unique_id: i64,
    pub runtime_id: u64,
}

#[derive(Resource, Default, Debug)]
pub struct ActorIndex {
    by_runtime_id: HashMap<u64, Entity>,
    by_unique_id: HashMap<i64, Entity>,
}

impl ActorIndex {
    pub fn by_runtime_id(&self, runtime_id: u64) -> Option<Entity> {
        self.by_runtime_id.get(&runtime_id).copied()
    }

    pub fn by_unique_id(&self, unique_id: i64) -> Option<Entity> {
        self.by_unique_id.get(&unique_id).copied()
    }

    pub fn len(&self) -> usize {
        self.by_runtime_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_runtime_id.is_empty()
    }

    pub fn track_actor_added(event: On<Add<ActorId>>, actors: Query<&ActorId>, mut index: ResMut<ActorIndex>) {
        if let Ok(actor) = actors.get(event.entity) {
            index.by_runtime_id.insert(actor.runtime_id, event.entity);
            index.by_unique_id.insert(actor.unique_id, event.entity);
        }
    }

    pub fn track_actor_removed(event: On<Remove<ActorId>>, actors: Query<&ActorId>, mut index: ResMut<ActorIndex>) {
        if let Ok(actor) = actors.get(event.entity) {
            if index.by_runtime_id.get(&actor.runtime_id) == Some(&event.entity) {
                index.by_runtime_id.remove(&actor.runtime_id);
            }
            if index.by_unique_id.get(&actor.unique_id) == Some(&event.entity) {
                index.by_unique_id.remove(&actor.unique_id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::world::World;

    fn world() -> World {
        let mut world = World::new();
        world.init_resource::<ActorIndex>();
        world.add_observer(ActorIndex::track_actor_added);
        world.add_observer(ActorIndex::track_actor_removed);
        world
    }

    #[test]
    fn indexes_spawned_actor_and_forgets_it_on_despawn() {
        let mut world = world();
        let entity = world.spawn(ActorId { unique_id: -5, runtime_id: 9 }).id();

        let index = world.resource::<ActorIndex>();
        assert_eq!(index.by_runtime_id(9), Some(entity));
        assert_eq!(index.by_unique_id(-5), Some(entity));

        world.despawn(entity);

        let index = world.resource::<ActorIndex>();
        assert_eq!(index.by_runtime_id(9), None);
        assert_eq!(index.by_unique_id(-5), None);
        assert!(index.is_empty());
    }

    #[test]
    fn forgets_actor_when_component_removed() {
        let mut world = world();
        let entity = world.spawn(ActorId { unique_id: 1, runtime_id: 2 }).id();
        world.entity_mut(entity).remove::<ActorId>();
        assert_eq!(world.resource::<ActorIndex>().by_runtime_id(2), None);
    }
}
