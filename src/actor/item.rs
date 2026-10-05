use crate::actor::physics::Physics;
use crate::actor::storage::ActorKind;
use crate::actor::viewers::{ActorShown, Despawn, NetworkOffset, Viewers, send_to_viewers};
use crate::block::component::loot_component::LootComponent;
use crate::entity::components::actor_id::ActorId;
use crate::entity::components::transform::Transform;
use crate::item::ItemTakenMessage;
use crate::item::item_entity::{ItemEntity, within_pickup_reach};
use crate::item::item_stack::ItemStack;
use crate::level::DimensionId;
use crate::network::BedrockProtocol;
use crate::network::handler::inventory::{ItemDropMessage, picked_item, send_content};
use crate::network::session::Session;
use crate::network::session::state::SessionState;
use crate::player::PLAYER_EYE_HEIGHT;
use crate::player::chunk_view::ChunkView;
use crate::player::gamemode::Gamemode;
use crate::player::inventory::{MAX_STACK_SIZE, PlayerInventory};
use crate::registry::block_registry::BlockRegistry;
use crate::registry::item_registry::ItemRegistry;
use crate::server::ServerState;
use crate::utils::hash_utils::HashUtils;
use crate::world::block::BlockBreakMessage;
use bedrock::protocol::ProtoVersionPackets;
use bedrock::protocol::v662::enums::ContainerID;
use bedrock::protocol::v662::packets::{AddItemActorPacket, TakeItemActorPacket};
use bedrock::protocol::v662::types::{ActorRuntimeID, ActorUniqueID};
use bedrock::protocol::v975::enums::ActorEvent;
use bevy_ecs::message::{MessageReader, MessageWriter};
use bevy_ecs::prelude::{Bundle, Commands, Entity, Local, Query, Res, ResMut, Without, World};
use glam::{Vec2, Vec3};
use std::collections::HashSet;
use std::f32::consts::TAU;

type ActorEventPacket = <BedrockProtocol as ProtoVersionPackets>::ActorEventPacket;

const MERGE_INTERVAL_TICKS: u32 = 10;
const MERGE_REACH: f32 = 0.5;
const THROW_BELOW_EYES: f32 = 0.3;
const THROW_SPEED: f32 = 0.3;

pub const ITEM_IDENTIFIER: &str = "minecraft:item";

fn item_bundle(item: ItemEntity, actor: ActorId, position: Vec3, velocity: Vec3, dimension: i32) -> impl Bundle {
    let physics = Physics::item();
    (
        item,
        actor,
        Transform {
            position,
            velocity,
            ..Transform::default()
        },
        DimensionId(dimension),
        physics,
        NetworkOffset(physics.height / 2.0),
        Viewers::default(),
        ActorKind(ITEM_IDENTIFIER),
    )
}

pub fn spawn_item(commands: &mut Commands, server_state: &mut ServerState, item: ItemEntity, center: Vec3, dimension: i32) -> Entity {
    let jitter = || rand::random::<f32>() * 0.5 - 0.25;
    let position = center + Vec3::new(jitter(), jitter() - Physics::item().height / 2.0, jitter());
    let velocity = Vec3::new(rand::random::<f32>() * 0.2 - 0.1, 0.2, rand::random::<f32>() * 0.2 - 0.1);
    spawn_item_at(commands, server_state, item, position, velocity, dimension)
}

/// Like `spawn_item`, but the item starts exactly where and how fast you say.
pub fn spawn_item_at(commands: &mut Commands, server_state: &mut ServerState, item: ItemEntity, position: Vec3, velocity: Vec3, dimension: i32) -> Entity {
    let actor = ActorId {
        unique_id: server_state.get_unique_id(),
        runtime_id: server_state.get_runtime_id(),
    };
    commands.spawn(item_bundle(item, actor, position, velocity, dimension)).id()
}

fn floats(values: &[f32]) -> nbtx::Value {
    nbtx::Value::List(nbtx::ValueList::Float(values.to_vec()))
}

fn read_floats(tag: &nbtx::Compound, key: &str) -> Option<Vec3> {
    match tag.get(key.as_bytes()) {
        Some(nbtx::Value::List(nbtx::ValueList::Float(values))) if values.len() >= 3 => Some(Vec3::new(values[0], values[1], values[2])),
        _ => None,
    }
}

pub fn save_item(world: &World, entity: Entity) -> Option<nbtx::Value> {
    let entity = world.get_entity(entity).ok()?;
    let (item, actor, transform) = (entity.get::<ItemEntity>()?, entity.get::<ActorId>()?, entity.get::<Transform>()?);
    let stack = item.stack();
    let name = world.resource::<ItemRegistry>().items().iter().find(|definition| definition.runtime_id == stack.id)?.identifier.clone();

    let mut item_tag = nbtx::Compound::new();
    item_tag.insert("Name".into(), nbtx::Value::String(name.as_str().into()));
    item_tag.insert("Count".into(), nbtx::Value::Byte(stack.count.min(i8::MAX as u16) as i8));
    item_tag.insert("Damage".into(), nbtx::Value::Short(stack.meta as i16));
    item_tag.insert("WasPickedUp".into(), nbtx::Value::Byte(0));
    if stack.block_runtime_id != 0
        && let Some(permutation) = world.resource::<BlockRegistry>().get_permutation(stack.block_runtime_id)
    {
        item_tag.insert("Block".into(), permutation.to_nbt());
    }

    let offset = Physics::item().height / 2.0;
    let (position, velocity) = (transform.position, transform.velocity);
    let mut tag = nbtx::Compound::new();
    tag.insert("identifier".into(), nbtx::Value::String(ITEM_IDENTIFIER.into()));
    tag.insert("UniqueID".into(), nbtx::Value::Long(actor.unique_id));
    tag.insert("Pos".into(), floats(&[position.x, position.y + offset, position.z]));
    tag.insert("Motion".into(), floats(&[velocity.x, velocity.y, velocity.z]));
    tag.insert("Rotation".into(), floats(&[0.0, 0.0]));
    tag.insert("OnGround".into(), nbtx::Value::Byte(entity.get::<Physics>().is_some_and(|physics| physics.on_ground) as i8));
    tag.insert("Age".into(), nbtx::Value::Short(item.age().min(i16::MAX as u32) as i16));
    tag.insert("PickupDelay".into(), nbtx::Value::Short(item.pickup_delay().min(i16::MAX as u32) as i16));
    tag.insert("Item".into(), nbtx::Value::Compound(item_tag));
    Some(nbtx::Value::Compound(tag))
}

pub fn load_item(world: &mut World, nbt: &nbtx::Value, dimension: i32) -> Option<Entity> {
    let nbtx::Value::Compound(tag) = nbt else { return None };
    let nbtx::Value::Compound(item_tag) = tag.get("Item".as_bytes())? else { return None };
    let nbtx::Value::String(name) = item_tag.get("Name".as_bytes())? else { return None };
    let id = world.resource::<ItemRegistry>().get(&name.to_string())?;
    let count = match item_tag.get("Count".as_bytes()) {
        Some(nbtx::Value::Byte(count)) => (*count).max(1) as u16,
        _ => 1,
    };
    let meta = match item_tag.get("Damage".as_bytes()) {
        Some(nbtx::Value::Short(damage)) => *damage as u32,
        _ => 0,
    };
    let block_runtime_id = match item_tag.get("Block".as_bytes()) {
        Some(nbtx::Value::Compound(block)) => match (block.get("name".as_bytes()), block.get("states".as_bytes())) {
            (Some(nbtx::Value::String(block_name)), Some(nbtx::Value::Compound(states))) => HashUtils::hash_block_nbt(&block_name.to_string(), states.clone()),
            (Some(nbtx::Value::String(block_name)), _) => HashUtils::hash_block_nbt(&block_name.to_string(), nbtx::Compound::new()),
            _ => 0,
        },
        _ => 0,
    };
    let short = |key: &str| match tag.get(key.as_bytes()) {
        Some(nbtx::Value::Short(value)) => (*value).max(0) as u32,
        _ => 0,
    };
    let stack = ItemStack { id, count, meta, block_runtime_id };
    let position = read_floats(tag, "Pos")? - Vec3::new(0.0, Physics::item().height / 2.0, 0.0);
    let velocity = read_floats(tag, "Motion").unwrap_or_default();
    let mut server_state = world.resource_mut::<ServerState>();
    let unique_id = match tag.get("UniqueID".as_bytes()) {
        Some(nbtx::Value::Long(unique_id)) => *unique_id,
        _ => server_state.get_unique_id(),
    };
    let actor = ActorId {
        unique_id,
        runtime_id: server_state.get_runtime_id(),
    };
    Some(
        world
            .spawn(item_bundle(ItemEntity::restore(stack, short("Age"), short("PickupDelay")), actor, position, velocity, dimension))
            .id(),
    )
}

pub fn spawn_block_drops(
    mut reader: MessageReader<BlockBreakMessage>,
    players: Query<(&Gamemode, &ChunkView)>,
    blocks: Res<BlockRegistry>,
    items: Res<ItemRegistry>,
    mut server_state: ResMut<ServerState>,
    mut commands: Commands,
) {
    for msg in reader.read() {
        let Ok((gamemode, view)) = players.get(msg.entity) else { continue };
        if *gamemode == Gamemode::Creative {
            continue;
        }
        if let Some(stack) = block_drop(&blocks, &items, msg.block_id) {
            spawn_item(&mut commands, &mut server_state, ItemEntity::new(stack), msg.position.as_vec3() + Vec3::splat(0.5), view.dimension);
        }
    }
}

pub fn spawn_player_drops(mut reader: MessageReader<ItemDropMessage>, players: Query<(&Transform, &ChunkView)>, mut server_state: ResMut<ServerState>, mut commands: Commands) {
    for msg in reader.read() {
        let Ok((transform, view)) = players.get(msg.entity) else { continue };
        let position = transform.position - Vec3::new(0.0, THROW_BELOW_EYES, 0.0);
        let velocity = if msg.randomly { scatter_velocity() } else { throw_velocity(transform.rotation) };
        spawn_item_at(&mut commands, &mut server_state, ItemEntity::thrown(msg.stack), position, velocity, view.dimension);
    }
}

/// Sends the item off where the player is looking, a bit upwards and with a little wobble so a
/// pile of drops doesn't stack up on one spot.
fn throw_velocity(rotation: Vec2) -> Vec3 {
    let (pitch, yaw) = (rotation.x.to_radians(), rotation.y.to_radians());
    let (angle, wobble) = (rand::random::<f32>() * TAU, rand::random::<f32>() * 0.02);

    Vec3::new(
        -yaw.sin() * pitch.cos() * THROW_SPEED + angle.cos() * wobble,
        -pitch.sin() * THROW_SPEED + 0.1 + (rand::random::<f32>() - rand::random::<f32>()) * 0.1,
        yaw.cos() * pitch.cos() * THROW_SPEED + angle.sin() * wobble,
    )
}

fn scatter_velocity() -> Vec3 {
    let (angle, speed) = (rand::random::<f32>() * TAU, rand::random::<f32>() * 0.5);
    Vec3::new(-angle.sin() * speed, 0.2, angle.cos() * speed)
}

/// The stack a broken block leaves behind, following its `LootComponent` when it has one.
fn block_drop(blocks: &BlockRegistry, items: &ItemRegistry, block_id: i32) -> Option<ItemStack> {
    let Some(loot) = blocks.get_components(block_id).and_then(|components| components.get::<LootComponent>()) else {
        return picked_item(blocks, items, block_id);
    };
    let identifier = loot.item?;

    Some(ItemStack {
        id: items.get(identifier)?,
        count: 1,
        meta: 0,
        block_runtime_id: blocks.get_block_id(identifier).unwrap_or(0),
    })
}

pub fn send_item_spawns(mut reader: MessageReader<ActorShown>, items: Query<(&ItemEntity, &ActorId, &Transform, &NetworkOffset)>, mut sessions: Query<&mut Session>) {
    for shown in reader.read() {
        let (Ok((item, actor, transform, offset)), Ok(mut session)) = (items.get(shown.actor), sessions.get_mut(shown.viewer)) else {
            continue;
        };
        let position = transform.position;
        session.send(BedrockProtocol::AddItemActorPacket(
            AddItemActorPacket {
                target_actor_id: ActorUniqueID(actor.unique_id),
                target_runtime_id: ActorRuntimeID(actor.runtime_id),
                item: item.stack().to_actor_descriptor(),
                position: (position.x, position.y + offset.0, position.z),
                velocity: (transform.velocity.x, transform.velocity.y, transform.velocity.z),
                entity_data: vec![],
                from_fishing: false,
            }
            .into(),
        ));
    }
}

pub fn tick_item_entities(mut items: Query<(Entity, &mut ItemEntity), Without<Despawn>>, mut commands: Commands) {
    for (entity, mut item) in &mut items {
        item.tick_pickup_delay();
        if item.tick_age() {
            commands.entity(entity).insert(Despawn);
        }
    }
}

type MergeItem<'a> = (Entity, &'a mut ItemEntity, &'a ActorId, &'a Transform, &'a DimensionId, &'a Viewers);

pub fn merge_item_entities(mut items: Query<MergeItem, Without<Despawn>>, mut sessions: Query<&mut Session>, mut commands: Commands, mut ticks: Local<u32>) {
    *ticks += 1;
    if *ticks < MERGE_INTERVAL_TICKS {
        return;
    }
    *ticks = 0;

    let (size, reach) = (Physics::item().height, Physics::item().width + MERGE_REACH);
    let mut removed = HashSet::new();
    let mut pairs = items.iter_combinations_mut();
    while let Some([first, second]) = pairs.fetch_next() {
        let (
            (first_entity, mut first_item, first_actor, first_transform, first_dimension, first_viewers),
            (second_entity, mut second_item, second_actor, second_transform, second_dimension, second_viewers),
        ) = (first, second);
        if first_dimension.0 != second_dimension.0 || removed.contains(&first_entity) || removed.contains(&second_entity) {
            continue;
        }
        let (a, b) = (first_transform.position, second_transform.position);
        if (a.x - b.x).abs() > reach || (a.z - b.z).abs() > reach || (a.y - b.y).abs() >= size || !first_item.stack().is_same(&second_item.stack()) {
            continue;
        }

        let first_is_target = first_item.stack().count >= second_item.stack().count;
        let (target, source, target_actor, source_actor, source_entity, target_viewers, source_viewers) = if first_is_target {
            (&mut *first_item, &mut *second_item, first_actor, second_actor, second_entity, first_viewers, second_viewers)
        } else {
            (&mut *second_item, &mut *first_item, second_actor, first_actor, first_entity, second_viewers, first_viewers)
        };
        if target.absorb(source, MAX_STACK_SIZE) == 0 {
            continue;
        }
        send_to_viewers(target_viewers, &mut sessions, &stack_size_packet(target_actor, target.stack().count));
        if source.stack().count == 0 {
            removed.insert(source_entity);
            commands.entity(source_entity).insert(Despawn);
        } else {
            send_to_viewers(source_viewers, &mut sessions, &stack_size_packet(source_actor, source.stack().count));
        }
    }
}

fn stack_size_packet(actor: &ActorId, count: u16) -> BedrockProtocol {
    BedrockProtocol::ActorEventPacket(
        ActorEventPacket {
            target_runtime_id: ActorRuntimeID(actor.runtime_id),
            event_id: ActorEvent::UpdateStackSize,
            data: count as i32,
            fire_at_position: None,
        }
        .into(),
    )
}

pub fn handle_item_pickup(
    mut players: Query<(Entity, &mut PlayerInventory, &ActorId, &Transform, &ChunkView)>,
    items: Query<(Entity, &ItemEntity, &ActorId, &Transform, &DimensionId, &Viewers), Without<Despawn>>,
    mut sessions: Query<&mut Session>,
    mut writer: MessageWriter<ItemTakenMessage>,
    mut commands: Commands,
) {
    let mut claimed = HashSet::new();

    for (player, mut inventory, player_actor, player_transform, view) in &mut players {
        if sessions.get(player).is_ok_and(|session| session.get_state() != SessionState::Play) {
            continue;
        }

        let feet = player_transform.position - Vec3::new(0.0, PLAYER_EYE_HEIGHT, 0.0);
        let mut changed = false;

        for (entity, item, item_actor, item_transform, dimension, viewers) in &items {
            if claimed.contains(&entity) || dimension.0 != view.dimension || !item.can_be_picked_up() || !within_pickup_reach(item_transform.position, feet) {
                continue;
            }
            if !inventory.main_mut().add(item.stack()) {
                continue;
            }

            claimed.insert(entity);
            changed = true;
            let take = BedrockProtocol::TakeItemActorPacket(
                TakeItemActorPacket {
                    item_runtime_id: ActorRuntimeID(item_actor.runtime_id),
                    actor_runtime_id: ActorRuntimeID(player_actor.runtime_id),
                }
                .into(),
            );
            send_to_viewers(viewers, &mut sessions, &take);
            commands.entity(entity).insert(Despawn);
            writer.write(ItemTakenMessage {
                item_entity: entity,
                dimension_id: dimension.0,
                item_unique_id: item_actor.unique_id,
                item_runtime_id: item_actor.runtime_id,
                player_runtime_id: player_actor.runtime_id,
            });
        }

        if changed && let Ok(mut session) = sessions.get_mut(player) {
            send_content(&mut session, &mut inventory, ContainerID::Inventory);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::storage::{ActorRegistry, save_all_actors};
    use crate::level::Level;
    use crate::level::dimension_type::DimensionType;
    use bevy_ecs::system::RunSystemOnce;

    #[test]
    fn items_round_trip_through_storage() {
        let directory = std::env::temp_dir().join(format!("chorus_actor_storage_{}", std::process::id()));
        let mut world = World::new();
        world.run_system_once(ItemRegistry::init).unwrap();
        world.run_system_once(BlockRegistry::init).unwrap();
        world.insert_resource(ServerState::new());
        let mut registry = ActorRegistry::default();
        registry.register(ITEM_IDENTIFIER, save_item, load_item);
        world.insert_resource(registry);
        let level = Level::open(&directory, "test", 0, world.resource::<BlockRegistry>(), 1);
        let storage = level.storage().cloned().expect("storage opens");
        world.insert_resource(level);

        let stone = world.resource::<BlockRegistry>().get_block_id("minecraft:stone").unwrap();
        let stack = ItemStack {
            id: world.resource::<ItemRegistry>().get("minecraft:stone").unwrap(),
            count: 12,
            meta: 0,
            block_runtime_id: stone,
        };
        let actor = ActorId { unique_id: 42, runtime_id: 7 };
        let position = Vec3::new(20.5, 64.0, -3.25);
        let entity = world.spawn(item_bundle(ItemEntity::restore(stack, 100, 0), actor, position, Vec3::ZERO, 0)).id();

        save_all_actors(&mut world);
        world.despawn(entity);
        let saved = storage.load_entities(DimensionType::Overworld, 1, -1).unwrap();
        assert_eq!(saved.len(), 1);

        let loaded = load_item(&mut world, &saved[0], 0).expect("item loads");
        let loaded = world.entity(loaded);
        let item = loaded.get::<ItemEntity>().unwrap();
        assert_eq!((item.stack(), item.age()), (stack, 100));
        assert_eq!(loaded.get::<ActorId>().unwrap().unique_id, 42);
        assert!((loaded.get::<Transform>().unwrap().position - position).length() < 1e-5);

        let moved = loaded.id();
        world.get_mut::<Transform>(moved).unwrap().position = Vec3::new(40.0, 64.0, 40.0);
        save_all_actors(&mut world);
        assert!(storage.load_entities(DimensionType::Overworld, 1, -1).unwrap().is_empty(), "the old chunk no longer lists it");
        assert_eq!(storage.load_entities(DimensionType::Overworld, 2, 2).unwrap().len(), 1);

        drop(world);
        drop(storage);
        let _ = std::fs::remove_dir_all(directory);
    }
}
