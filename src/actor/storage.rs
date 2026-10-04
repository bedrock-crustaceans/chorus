use crate::actor::viewers::Despawn;
use crate::entity::components::actor_id::ActorId;
use crate::entity::components::transform::Transform;
use crate::level::DimensionId;
use crate::level::Level;
use crate::level::dimension_type::DimensionType;
use bevy_ecs::prelude::{Component, Entity, Resource, Without, World};
use std::collections::{HashMap, HashSet};
use tracing::error;

pub type SaveActor = fn(&World, Entity) -> Option<nbtx::Value>;
pub type LoadActor = fn(&mut World, &nbtx::Value, i32) -> Option<Entity>;

#[derive(Component, Clone, Copy, Debug)]
pub struct ActorKind(pub &'static str);

#[derive(Resource, Default)]
pub struct ActorRegistry {
    kinds: HashMap<&'static str, (SaveActor, LoadActor)>,
}

impl ActorRegistry {
    pub fn register(&mut self, identifier: &'static str, save: SaveActor, load: LoadActor) {
        self.kinds.insert(identifier, (save, load));
    }
}

#[derive(Resource, Default)]
struct StoredActorChunks(HashSet<(i32, i32, i32)>);

fn identifier(nbt: &nbtx::Value) -> Option<String> {
    match nbt {
        nbtx::Value::Compound(tag) => match tag.get("identifier".as_bytes()) {
            Some(nbtx::Value::String(identifier)) => Some(identifier.to_string()),
            _ => None,
        },
        _ => None,
    }
}

pub fn load_chunk_actors(world: &mut World) {
    world.init_resource::<StoredActorChunks>();
    let Some(mut level) = world.get_resource_mut::<Level>() else { return };
    let Some(storage) = level.storage().cloned() else { return };
    let available: Vec<(DimensionType, Vec<(i32, i32)>)> = level.dimensions_mut().map(|dimension| (dimension.dimension_type, dimension.take_available())).collect();

    for (dimension, positions) in available {
        for (x, z) in positions {
            let entities = match storage.load_entities(dimension, x, z) {
                Ok(entities) => entities,
                Err(err) => {
                    error!("failed to load the actors of chunk ({x}, {z}) in {}: {err}", dimension.name());
                    continue;
                }
            };
            if entities.is_empty() {
                continue;
            }
            world.resource_mut::<StoredActorChunks>().0.insert((dimension.id(), x, z));
            for nbt in entities {
                let load = identifier(&nbt).and_then(|identifier| world.resource::<ActorRegistry>().kinds.get(identifier.as_str()).map(|(_, load)| *load));
                if let Some(load) = load {
                    load(world, &nbt, dimension.id());
                }
            }
        }
    }
}

fn save_actors(world: &mut World, only: Option<&HashSet<(i32, i32, i32)>>, despawn: bool) {
    world.init_resource::<StoredActorChunks>();
    let Some(storage) = world.get_resource::<Level>().and_then(|level| level.storage().cloned()) else {
        return;
    };

    let mut query = world.query_filtered::<(Entity, &ActorKind, &ActorId, &Transform, &DimensionId), Without<Despawn>>();
    let actors: Vec<(Entity, &'static str, i64, (i32, i32, i32))> = query
        .iter(world)
        .map(|(entity, kind, actor, transform, dimension)| {
            let chunk = (dimension.0, (transform.position.x.floor() as i32) >> 4, (transform.position.z.floor() as i32) >> 4);
            (entity, kind.0, actor.unique_id, chunk)
        })
        .filter(|(.., chunk)| only.is_none_or(|only| only.contains(chunk)))
        .collect();

    let mut chunks: HashMap<(i32, i32, i32), Vec<(i64, nbtx::Value)>> = HashMap::new();
    for &(entity, kind, unique_id, chunk) in &actors {
        let save = world.resource::<ActorRegistry>().kinds.get(kind).map(|(save, _)| *save);
        if let Some(nbt) = save.and_then(|save| save(world, entity)) {
            chunks.entry(chunk).or_default().push((unique_id, nbt));
        }
    }
    let emptied: Vec<(i32, i32, i32)> = world
        .resource::<StoredActorChunks>()
        .0
        .iter()
        .filter(|chunk| !chunks.contains_key(chunk) && only.is_none_or(|only| only.contains(chunk)))
        .copied()
        .collect();
    for chunk in emptied {
        chunks.entry(chunk).or_default();
    }

    let saved_ids: HashSet<i64> = chunks.values().flatten().map(|(id, _)| *id).collect();
    for ((dimension, x, z), entities) in &chunks {
        let Some(dimension_type) = DimensionType::from_id(*dimension) else { continue };
        if let Err(err) = storage.save_entities(dimension_type, *x, *z, entities, &saved_ids) {
            error!("failed to save the actors of chunk ({x}, {z}) in {}: {err}", dimension_type.name());
            continue;
        }
        let stored = &mut world.resource_mut::<StoredActorChunks>().0;
        if entities.is_empty() {
            stored.remove(&(*dimension, *x, *z));
        } else {
            stored.insert((*dimension, *x, *z));
        }
    }
    storage.schedule_flush();

    if despawn {
        for (entity, ..) in actors {
            world.entity_mut(entity).insert(Despawn);
        }
    }
}

pub fn save_unloaded_chunk_actors(world: &mut World) {
    let Some(mut level) = world.get_resource_mut::<Level>() else { return };
    let unloaded: HashSet<(i32, i32, i32)> = level
        .dimensions_mut()
        .flat_map(|dimension| {
            let id = dimension.id();
            dimension.take_unloaded().into_iter().map(move |(x, z)| (id, x, z))
        })
        .collect();
    if !unloaded.is_empty() {
        save_actors(world, Some(&unloaded), true);
    }
}

pub fn save_all_actors(world: &mut World) {
    save_actors(world, None, false);
}
