use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::friction_component::FrictionComponent;
use crate::entity::components::actor_id::ActorId;
use crate::entity::components::transform::Transform;
use crate::item::ItemTakenMessage;
use crate::item::item_entity::{ItemEntity, within_pickup_reach};
use crate::level::DimensionId;
use crate::level::Level;
use crate::network::BedrockProtocol;
use crate::network::handler::block::BlockBreakMessage;
use crate::network::handler::inventory::{picked_item, send_content};
use crate::network::session::Session;
use crate::network::session::state::SessionState;
use crate::player::PLAYER_EYE_HEIGHT;
use crate::player::chunk_view::ChunkView;
use crate::player::gamemode::Gamemode;
use crate::player::inventory::MAX_STACK_SIZE;
use crate::player::inventory::PlayerInventory;
use crate::registry::block_registry::BlockRegistry;
use crate::registry::item_registry::ItemRegistry;
use crate::server::ServerState;
use bedrock::protocol::ProtoVersionPackets;
use bedrock::protocol::v662::enums::ContainerID;
use bedrock::protocol::v662::packets::{AddItemActorPacket, MoveActorAbsolutePacket, RemoveActorPacket, TakeItemActorPacket};
use bedrock::protocol::v662::types::{ActorRuntimeID, ActorUniqueID, MoveActorAbsoluteData};
use bedrock::protocol::v975::enums::ActorEvent;
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::message::{MessageReader, MessageWriter};
use bevy_ecs::prelude::{Added, Commands, Entity, Local, Query, Ref, Res, ResMut};
use glam::{IVec3, Vec3};
use std::collections::{HashMap, HashSet};

type ActorEventPacket = <BedrockProtocol as ProtoVersionPackets>::ActorEventPacket;

pub fn spawn_block_drops(
    mut reader: MessageReader<BlockBreakMessage>,
    players: Query<(&Gamemode, &ChunkView)>,
    blocks: Res<BlockRegistry>,
    items: Res<ItemRegistry>,
    mut server_state: ResMut<ServerState>,
    mut commands: Commands,
) {
    for msg in reader.read() {
        let Ok((gamemode, view)) = players.get(msg.entity) else {
            continue;
        };
        if *gamemode == Gamemode::Creative {
            continue;
        }

        let Some(stack) = picked_item(&blocks, &items, msg.block_id) else {
            continue;
        };

        let runtime_id = server_state.get_runtime_id();
        let jitter = || rand::random::<f32>() * 0.5 - 0.25;
        let position = msg.position.as_vec3() + Vec3::new(0.5 + jitter(), 0.5 + jitter() - ITEM_SIZE / 2.0, 0.5 + jitter());
        let velocity = Vec3::new(rand::random::<f32>() * 0.2 - 0.1, 0.2, rand::random::<f32>() * 0.2 - 0.1);
        commands.spawn((
            ItemEntity::new(stack),
            ActorId {
                unique_id: runtime_id as i64,
                runtime_id,
            },
            Transform {
                position,
                velocity,
                ..Transform::default()
            },
            DimensionId(view.dimension),
        ));
    }
}

fn add_item_packet(item: &ItemEntity, actor: &ActorId, transform: &Transform) -> BedrockProtocol {
    let position = transform.position;
    BedrockProtocol::AddItemActorPacket(
        AddItemActorPacket {
            target_actor_id: ActorUniqueID(actor.unique_id),
            target_runtime_id: ActorRuntimeID(actor.runtime_id),
            item: item.stack().to_actor_descriptor(),
            position: (position.x, position.y + ITEM_SIZE / 2.0, position.z),
            velocity: (transform.velocity.x, transform.velocity.y, transform.velocity.z),
            entity_data: vec![],
            from_fishing: false,
        }
        .into(),
    )
}

pub fn broadcast_spawned_items(new_items: Query<(&ItemEntity, &ActorId, &Transform, &DimensionId), Added<ItemEntity>>, mut sessions: Query<(&mut Session, &ChunkView)>) {
    for (item, actor, transform, dimension) in &new_items {
        for (mut session, view) in &mut sessions {
            if session.get_state() == SessionState::Play && view.dimension == dimension.0 {
                session.send(add_item_packet(item, actor, transform));
            }
        }
    }
}

/// Sends every item in a dimension to players who just joined it or arrived from another dimension,
/// since items are otherwise only announced when they spawn.
pub fn show_items_to_new_viewers(items: Query<(Ref<ItemEntity>, &ActorId, &Transform, &DimensionId)>, mut sessions: Query<(Entity, &mut Session, &ChunkView)>, mut shown: Local<HashMap<Entity, i32>>) {
    shown.retain(|entity, _| sessions.contains(*entity));
    for (entity, mut session, view) in &mut sessions {
        if session.get_state() != SessionState::Play || shown.get(&entity) == Some(&view.dimension) {
            continue;
        }
        shown.insert(entity, view.dimension);
        for (item, actor, transform, dimension) in &items {
            if dimension.0 == view.dimension && !item.is_added() {
                session.send(add_item_packet(&item, actor, transform));
            }
        }
    }
}

const MERGE_INTERVAL_TICKS: u32 = 10;
/// How far apart two item boxes may be horizontally and still merge, like vanilla.
const MERGE_REACH: f32 = 0.5;

/// Merges nearby stacks of the same item into the larger one.
pub fn merge_item_entities(
    mut items: Query<(Entity, &mut ItemEntity, &ActorId, &Transform, &DimensionId)>,
    mut sessions: Query<(&mut Session, &ChunkView)>,
    mut commands: Commands,
    mut ticks: Local<u32>,
) {
    *ticks += 1;
    if *ticks < MERGE_INTERVAL_TICKS {
        return;
    }
    *ticks = 0;

    let mut removed = HashSet::new();
    let mut updates: Vec<(i32, u64, u16)> = Vec::new();
    let mut gone: Vec<(i32, i64)> = Vec::new();
    let mut pairs = items.iter_combinations_mut();
    while let Some(
        [
            (first_entity, mut first, first_actor, first_transform, first_dimension),
            (second_entity, mut second, second_actor, second_transform, second_dimension),
        ],
    ) = pairs.fetch_next()
    {
        if first_dimension.0 != second_dimension.0 || removed.contains(&first_entity) || removed.contains(&second_entity) {
            continue;
        }
        let (a, b) = (first_transform.position, second_transform.position);
        let reach = ITEM_SIZE + MERGE_REACH;
        if (a.x - b.x).abs() > reach || (a.z - b.z).abs() > reach || (a.y - b.y).abs() >= ITEM_SIZE || !first.stack().is_same(&second.stack()) {
            continue;
        }

        let first_is_target = first.stack().count >= second.stack().count;
        let (target, source, target_actor, source_actor, source_entity) = if first_is_target {
            (&mut *first, &mut *second, first_actor, second_actor, second_entity)
        } else {
            (&mut *second, &mut *first, second_actor, first_actor, first_entity)
        };
        if target.absorb(source, MAX_STACK_SIZE) == 0 {
            continue;
        }
        updates.push((first_dimension.0, target_actor.runtime_id, target.stack().count));
        if source.stack().count == 0 {
            removed.insert(source_entity);
            gone.push((first_dimension.0, source_actor.unique_id));
            commands.entity(source_entity).despawn();
        } else {
            updates.push((first_dimension.0, source_actor.runtime_id, source.stack().count));
        }
    }

    for (mut session, view) in &mut sessions {
        if session.get_state() != SessionState::Play {
            continue;
        }
        for &(_, runtime_id, count) in updates.iter().filter(|(dimension, ..)| *dimension == view.dimension) {
            session.send(BedrockProtocol::ActorEventPacket(
                ActorEventPacket {
                    target_runtime_id: ActorRuntimeID(runtime_id),
                    event_id: ActorEvent::UpdateStackSize,
                    data: count as i32,
                    fire_at_position: None,
                }
                .into(),
            ));
        }
        for &(_, unique_id) in gone.iter().filter(|(dimension, _)| *dimension == view.dimension) {
            session.send(BedrockProtocol::RemoveActorPacket(
                RemoveActorPacket {
                    target_actor_id: ActorUniqueID(unique_id),
                }
                .into(),
            ));
        }
    }
}

/// Width and height of an item's collision box; its position is the bottom centre.
const ITEM_SIZE: f32 = 0.25;
const GRAVITY: f32 = 0.04;
const DRAG: f32 = 0.98;
const DEFAULT_FRICTION: f32 = 0.6;

/// Collision boxes of the blocks an item can't pass through.
struct BlockCollision<'a> {
    level: &'a Level,
    blocks: &'a BlockRegistry,
    dimension: i32,
    air: Option<i32>,
}

impl BlockCollision<'_> {
    /// The block's collision box in world space; unloaded chunks count as solid so items never fall out of the world.
    fn collision_box(&self, block: IVec3) -> Option<(Vec3, Vec3)> {
        let origin = block.as_vec3();
        let Some(id) = self.level.get_block(self.dimension, block.x, block.y, block.z, 0) else {
            return Some((origin, origin + Vec3::ONE));
        };
        if Some(id) == self.air {
            return None;
        }
        match self.blocks.get_components(id).and_then(|components| components.get::<CollisionBoxComponent>()) {
            Some(collision) if !collision.enabled => None,
            Some(collision) => Some((origin + collision.origin, origin + collision.origin + collision.size)),
            None => Some((origin, origin + Vec3::ONE)),
        }
    }

    fn friction(&self, block: IVec3) -> f32 {
        self.level
            .get_block(self.dimension, block.x, block.y, block.z, 0)
            .and_then(|id| self.blocks.get_components(id))
            .and_then(|components| components.get::<FrictionComponent>())
            .map_or(DEFAULT_FRICTION, |friction| friction.friction)
    }

    /// Moves a box along one axis as far as it can go and returns the distance travelled.
    fn sweep(&self, min: Vec3, max: Vec3, axis: usize, delta: f32) -> f32 {
        if delta == 0.0 {
            return 0.0;
        }
        let (mut swept_min, mut swept_max) = (min, max);
        if delta < 0.0 {
            swept_min[axis] += delta;
        } else {
            swept_max[axis] += delta;
        }
        let from = (swept_min - Vec3::splat(0.5)).floor().as_ivec3();
        let to = (swept_max + Vec3::splat(0.5)).floor().as_ivec3();
        let mut allowed = delta;
        for x in from.x..=to.x {
            for y in from.y..=to.y {
                for z in from.z..=to.z {
                    let Some((block_min, block_max)) = self.collision_box(IVec3::new(x, y, z)) else { continue };
                    let overlaps = (0..3).filter(|&other| other != axis).all(|other| min[other] < block_max[other] && max[other] > block_min[other]);
                    if !overlaps {
                        continue;
                    }
                    if delta > 0.0 && block_min[axis] >= max[axis] {
                        allowed = allowed.min(block_min[axis] - max[axis]);
                    } else if delta < 0.0 && block_max[axis] <= min[axis] {
                        allowed = allowed.max(block_max[axis] - min[axis]);
                    }
                }
            }
        }
        allowed
    }
    /// Advances an item by one tick of gravity, movement and drag, and returns whether it is on the ground.
    fn step(&self, position: &mut Vec3, velocity: &mut Vec3) -> bool {
        velocity.y -= GRAVITY;

        let half = Vec3::new(ITEM_SIZE / 2.0, 0.0, ITEM_SIZE / 2.0);
        let mut on_ground = false;
        for axis in [1, 0, 2] {
            let (min, max) = (*position - half, *position + half + Vec3::new(0.0, ITEM_SIZE, 0.0));
            let moved = self.sweep(min, max, axis, velocity[axis]);
            if moved != velocity[axis] {
                on_ground |= axis == 1 && velocity[axis] < 0.0;
                velocity[axis] = if axis == 1 && on_ground { velocity[axis] * -0.5 } else { 0.0 };
                if axis == 1 && velocity[axis].abs() < 0.05 {
                    velocity[axis] = 0.0;
                }
            }
            position[axis] += moved;
        }

        let friction = if on_ground {
            self.friction((*position - Vec3::new(0.0, 0.01, 0.0)).floor().as_ivec3()) * DRAG
        } else {
            DRAG
        };
        *velocity *= Vec3::new(friction, DRAG, friction);
        if velocity.x.abs() < 1e-3 {
            velocity.x = 0.0;
        }
        if velocity.z.abs() < 1e-3 {
            velocity.z = 0.0;
        }
        on_ground
    }
}

pub fn tick_item_entities(
    mut items: Query<(Entity, &mut ItemEntity, &mut Transform, &ActorId, &DimensionId)>,
    mut sessions: Query<(&mut Session, &ChunkView)>,
    level: Option<Res<Level>>,
    blocks: Res<BlockRegistry>,
    mut commands: Commands,
) {
    let Some(level) = level else { return };
    let air = blocks.get_block_id("minecraft:air");

    for (entity, mut item, mut transform, actor, dimension) in &mut items {
        item.tick_pickup_delay();
        if item.tick_age() {
            for (mut session, view) in &mut sessions {
                if session.get_state() == SessionState::Play && view.dimension == dimension.0 {
                    session.send(BedrockProtocol::RemoveActorPacket(
                        RemoveActorPacket {
                            target_actor_id: ActorUniqueID(actor.unique_id),
                        }
                        .into(),
                    ));
                }
            }
            commands.entity(entity).despawn();
            continue;
        }

        let collision = BlockCollision {
            level: &level,
            blocks: &blocks,
            dimension: dimension.0,
            air,
        };
        let below = (transform.position - Vec3::new(0.0, 0.01, 0.0)).floor().as_ivec3();
        if item.on_ground && transform.velocity.length_squared() < 1e-6 && collision.collision_box(below).is_some() {
            continue;
        }

        let (mut position, mut velocity) = (transform.position, transform.velocity);
        let on_ground = collision.step(&mut position, &mut velocity);

        let moved = position != transform.position;
        transform.position = position;
        transform.velocity = velocity;
        item.on_ground = on_ground;
        if !moved {
            continue;
        }

        for (mut session, view) in &mut sessions {
            if session.get_state() != SessionState::Play || view.dimension != dimension.0 {
                continue;
            }
            session.send(BedrockProtocol::MoveActorAbsolutePacket(
                MoveActorAbsolutePacket {
                    move_data: MoveActorAbsoluteData {
                        actor_runtime_id: ActorRuntimeID(actor.runtime_id),
                        header: on_ground as i8,
                        position: (position.x, position.y + ITEM_SIZE / 2.0, position.z),
                        rotation_x: 0,
                        rotation_y: 0,
                        rotation_y_head: 0,
                    },
                }
                .into(),
            ));
        }
    }
}

pub fn handle_item_pickup(
    mut players: Query<(&mut Session, &mut PlayerInventory, &ActorId, &Transform, &ChunkView)>,
    items: Query<(Entity, &ItemEntity, &ActorId, &Transform, &DimensionId)>,
    mut writer: MessageWriter<ItemTakenMessage>,
) {
    let mut claimed = HashSet::new();

    for (mut session, mut inventory, player_actor, player_transform, view) in &mut players {
        if session.get_state() != SessionState::Play {
            continue;
        }

        let feet = player_transform.position - Vec3::new(0.0, PLAYER_EYE_HEIGHT, 0.0);
        let mut changed = false;

        for (entity, item, item_actor, item_transform, dimension) in &items {
            if claimed.contains(&entity) || dimension.0 != view.dimension || !item.can_be_picked_up() || !within_pickup_reach(item_transform.position, feet) {
                continue;
            }
            if !inventory.main_mut().add(item.stack()) {
                continue;
            }

            claimed.insert(entity);
            changed = true;

            writer.write(ItemTakenMessage {
                item_entity: entity,
                dimension_id: dimension.0,
                item_unique_id: item_actor.unique_id,
                item_runtime_id: item_actor.runtime_id,
                player_runtime_id: player_actor.runtime_id,
            });
        }

        if changed {
            send_content(&mut session, &mut inventory, ContainerID::Inventory);
        }
    }
}

pub fn broadcast_taken_items(mut reader: MessageReader<ItemTakenMessage>, mut sessions: Query<(&mut Session, &ChunkView)>, mut commands: Commands) {
    for msg in reader.read() {
        for (mut session, view) in &mut sessions {
            if session.get_state() != SessionState::Play || view.dimension != msg.dimension_id {
                continue;
            }

            session.send(BedrockProtocol::TakeItemActorPacket(
                TakeItemActorPacket {
                    item_runtime_id: ActorRuntimeID(msg.item_runtime_id),
                    actor_runtime_id: ActorRuntimeID(msg.player_runtime_id),
                }
                .into(),
            ));
            session.send(BedrockProtocol::RemoveActorPacket(
                RemoveActorPacket {
                    target_actor_id: ActorUniqueID(msg.item_unique_id),
                }
                .into(),
            ));
        }

        commands.entity(msg.item_entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::dimension_type::DimensionType;
    use crate::level::generator::r#impl::flat::{FlatGenerator, FlatLayer};

    fn flat_level(blocks: &BlockRegistry, top: &str) -> Level {
        bevy_tasks::AsyncComputeTaskPool::get_or_init(bevy_tasks::TaskPool::default);
        let id = |name: &str| blocks.get_block_id(name).expect(name);
        let mut level = Level::in_memory("test", 0);
        let dimension = level.insert_dimension(
            DimensionType::Overworld,
            FlatGenerator {
                layers: vec![
                    FlatLayer {
                        block_id: id("minecraft:stone"),
                        height: 3,
                    },
                    FlatLayer { block_id: id(top), height: 1 },
                ],
                biome: 1,
                air_id: id("minecraft:air"),
                min_sub_chunk_y: DimensionType::Overworld.min_sub_chunk_y(),
                sub_chunk_count: DimensionType::Overworld.sub_chunk_count(),
            },
        );
        dimension.request_chunk(0, 0);
        while dimension.tick().is_empty() {
            std::thread::yield_now();
        }
        level
    }

    fn drop_item(top: &str, start: Vec3, velocity: Vec3) -> (Vec3, Vec3, bool, usize) {
        let mut blocks = BlockRegistry::new();
        blocks.register_all(crate::block::r#impl::DEFINITIONS.iter().copied());
        let level = flat_level(&blocks, top);
        let collision = BlockCollision {
            level: &level,
            blocks: &blocks,
            dimension: 0,
            air: blocks.get_block_id("minecraft:air"),
        };
        let (mut position, mut velocity, mut on_ground) = (start, velocity, false);
        let mut ticks = 0;
        while ticks < 400 && !(on_ground && velocity == Vec3::ZERO) {
            on_ground = collision.step(&mut position, &mut velocity);
            ticks += 1;
        }
        (position, velocity, on_ground, ticks)
    }

    #[test]
    fn items_fall_and_settle_on_the_surface() {
        let (position, velocity, on_ground, ticks) = drop_item("minecraft:grass_block", Vec3::new(8.5, 10.0, 8.5), Vec3::new(0.05, 0.2, 0.0));
        assert!(on_ground, "settled after {ticks} ticks at {position}");
        assert_eq!(velocity, Vec3::ZERO);
        assert!((position.y - 4.0).abs() < 1e-4, "resting on top of the 4 block floor, got {position}");
        assert!(ticks < 100, "took {ticks} ticks to settle");
    }

    #[test]
    fn ice_lets_items_slide_further() {
        let start = Vec3::new(1.5, 4.0, 8.5);
        let (on_grass, ..) = drop_item("minecraft:grass_block", start, Vec3::new(0.2, 0.0, 0.0));
        let (on_ice, ..) = drop_item("minecraft:ice", start, Vec3::new(0.2, 0.0, 0.0));
        let (grass, ice) = (on_grass.x - start.x, on_ice.x - start.x);
        assert!(grass > 0.2 && grass < 1.0, "grass slid {grass}");
        assert!(ice > grass * 4.0, "grass slid {grass}, ice slid {ice}");
    }
}
