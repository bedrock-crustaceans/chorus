use crate::block::component::mineable_component::MineableComponent;
use crate::block::component::replaceable_component::ReplaceableComponent;
use crate::core::math::enums::block_face::BlockFace;
use crate::entity::components::transform::Transform;
use crate::level::level::Level;
use crate::level::{BlockUpdatedMessage, LevelEventMessage, LevelSoundMessage};
use crate::network::BedrockProtocol;
use crate::network::session::Session;
use crate::network::session::state::SessionState;
use crate::player::block_break::{BlockBreakHandler, BlockBreaking, BreakTick, break_speed};
use crate::player::chunk_view::ChunkView;
use crate::player::gamemode::Gamemode;
use crate::player::inventory::PlayerInventory;
use crate::registry::block_registry::BlockRegistry;
use bedrock::protocol::v662::enums::PlayerActionType;
use bedrock::protocol::v662::packets::LevelEventPacket;
use bedrock::protocol::v662::packets::UpdateBlockPacket;
use bedrock::protocol::v766::enums::LevelEvent;
use bedrock::protocol::v944::types::NetworkBlockPosition;
use bedrock::protocol::v1001::packets::LevelSoundEventPacket;
use bevy_ecs::change_detection::ResMut;
use bevy_ecs::message::{Message, MessageReader, MessageWriter};
use bevy_ecs::prelude::{Commands, Entity, Event, FromWorld, Mut, Query, Resource, World};
use bevy_ecs::system::{Command, Res, SystemParam, SystemState};
use glam::{IVec3, Vec3};
use tracing::{debug, warn};

const fn level_event(event: LevelEvent) -> i32 {
    event as i32
}

const SOUND_HIT: &str = "hit";

const MAX_REACH_DISTANCE: f32 = 13.0;

#[derive(Message, Clone, Debug)]
pub struct BlockActionMessage {
    pub entity: Entity,
    pub action: PlayerActionType,
    pub position: IVec3,
    pub face: i32,
    pub slot: Option<u8>,
}

#[derive(Message, Clone, Debug)]
pub struct BlockBreakMessage {
    pub entity: Entity,
    pub position: IVec3,
    pub block_id: i32,
}

#[derive(Message, Clone, Debug)]
pub struct BlockPlaceMessage {
    pub entity: Entity,
    pub position: IVec3,
    pub block_id: i32,
    pub face: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RejectReason {
    NotAllowed,
    NotPlaceable,
    OutOfReach,
    Occupied,
    Unbreakable,
    Stale,
    Obstructed,
    Vetoed,
}

impl RejectReason {
    pub fn client_predicted(self) -> bool {
        self != Self::Obstructed
    }
}

#[derive(Message, Clone, Debug)]
pub struct BlockEditRejected {
    pub entity: Entity,
    pub position: IVec3,
    pub kind: BlockActionKind,
    pub reason: RejectReason,
}

fn reject(writer: &mut MessageWriter<BlockEditRejected>, entity: Entity, position: IVec3, kind: BlockActionKind, reason: RejectReason) {
    writer.write(BlockEditRejected { entity, position, kind, reason });
}

fn reject_action(action: &BlockActionMessage, writer: &mut MessageWriter<BlockEditRejected>) {
    match action.action {
        PlayerActionType::StartItemUseOn => {
            if let Ok(face) = BlockFace::from_index(action.face as usize) {
                reject(
                    writer,
                    action.entity,
                    action.position + face.get_unit_vec().as_ivec3(),
                    BlockActionKind::Place,
                    RejectReason::NotAllowed,
                );
            }
        }
        PlayerActionType::PredictDestroyBlock | PlayerActionType::CreativeDestroyBlock => {
            reject(writer, action.entity, action.position, BlockActionKind::Break, RejectReason::NotAllowed);
        }
        _ => {}
    }
}

fn is_replaceable(registry: &BlockRegistry, block_id: i32) -> bool {
    registry
        .get_components(block_id)
        .and_then(|components| components.get::<ReplaceableComponent>())
        .is_some_and(|component| component.replaceable)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockActionKind {
    Place,
    Break,
}

#[derive(Event, Clone, Debug)]
pub struct BlockEdit {
    pub entity: Entity,
    pub kind: BlockActionKind,
    pub dimension: i32,
    pub position: IVec3,
    pub block_id: i32,
    pub face: i32,
    rejection: Option<RejectReason>,
}

impl BlockEdit {
    pub fn new(entity: Entity, kind: BlockActionKind, dimension: i32, position: IVec3, block_id: i32, face: i32) -> Self {
        Self {
            entity,
            kind,
            dimension,
            position,
            block_id,
            face,
            rejection: None,
        }
    }

    pub fn cancel(&mut self) {
        self.reject(RejectReason::Vetoed);
    }

    pub fn reject(&mut self, reason: RejectReason) {
        self.rejection.get_or_insert(reason);
    }

    pub fn is_cancelled(&self) -> bool {
        self.rejection.is_some()
    }

    pub fn rejection(&self) -> Option<RejectReason> {
        self.rejection
    }
}

#[derive(SystemParam)]
struct EditParams<'w> {
    level: ResMut<'w, Level>,
    registry: Res<'w, BlockRegistry>,
    events: MessageWriter<'w, LevelEventMessage>,
    blocks: MessageWriter<'w, BlockUpdatedMessage>,
    broken: MessageWriter<'w, BlockBreakMessage>,
    placed: MessageWriter<'w, BlockPlaceMessage>,
    rejected: MessageWriter<'w, BlockEditRejected>,
}

#[derive(Resource)]
struct BlockEditState(SystemState<EditParams<'static>>);

impl FromWorld for BlockEditState {
    fn from_world(world: &mut World) -> Self {
        Self(SystemState::new(world))
    }
}

fn edit_command(mut edit: BlockEdit) -> impl Command {
    move |world: &mut World| {
        world.trigger_ref(&mut edit);
        if !world.contains_resource::<BlockEditState>() {
            world.init_resource::<BlockEditState>();
        }
        world.resource_scope(|world, mut state: Mut<BlockEditState>| {
            match state.0.get_mut(world) {
                Ok(mut params) => resolve_block_edit(&mut params, edit),
                Err(error) => warn!("a block edit was dropped: {error}"),
            }
            state.0.apply(world);
        });
    }
}

fn resolve_block_edit(params: &mut EditParams, edit: BlockEdit) {
    let EditParams {
        level,
        registry,
        events,
        blocks,
        broken,
        placed,
        rejected,
    } = params;
    let position = edit.position;
    if let Some(reason) = edit.rejection {
        debug!("block edit {:?} at {} was rejected: {:?}", edit.kind, position, reason);
        reject(rejected, edit.entity, position, edit.kind, reason);
        return;
    }
    let Some(air_id) = registry.get_block_id("minecraft:air") else {
        return;
    };
    let Some(current) = level.get_block(edit.dimension, position.x, position.y, position.z, 0) else {
        reject(rejected, edit.entity, position, edit.kind, RejectReason::Stale);
        return;
    };

    match edit.kind {
        BlockActionKind::Place => {
            if current == edit.block_id {
                return;
            }
            if current != air_id && !is_replaceable(registry, current) {
                reject(rejected, edit.entity, position, edit.kind, RejectReason::Occupied);
                return;
            }
            if !level.set_block(edit.dimension, position.x, position.y, position.z, 0, edit.block_id, blocks) {
                reject(rejected, edit.entity, position, edit.kind, RejectReason::Stale);
                return;
            }
            placed.write(BlockPlaceMessage {
                entity: edit.entity,
                position,
                block_id: edit.block_id,
                face: edit.face,
            });
        }
        BlockActionKind::Break => {
            if current != edit.block_id {
                reject(rejected, edit.entity, position, edit.kind, RejectReason::Stale);
                return;
            }
            events.write(LevelEventMessage {
                dimension_id: edit.dimension,
                event_id: level_event(LevelEvent::ParticlesDestroyBlock),
                position: position.as_vec3() + Vec3::splat(0.5),
                data: edit.block_id,
            });
            if !level.set_block(edit.dimension, position.x, position.y, position.z, 0, air_id, blocks) {
                reject(rejected, edit.entity, position, edit.kind, RejectReason::Stale);
                return;
            }
            broken.write(BlockBreakMessage {
                entity: edit.entity,
                position,
                block_id: edit.block_id,
            });
        }
    }
}

pub fn handle_block_actions(
    mut action_reader: MessageReader<BlockActionMessage>,
    mut query: Query<(&Session, &Transform, &mut BlockBreaking, &mut PlayerInventory, &ChunkView, &Gamemode)>,
    level: Res<Level>,
    registry: Res<BlockRegistry>,
    mut commands: Commands,
    mut event_writer: MessageWriter<LevelEventMessage>,
    mut rejected: MessageWriter<BlockEditRejected>,
) {
    for action in action_reader.read() {
        let Ok((session, transform, mut breaking, mut inventory, view, gamemode)) = query.get_mut(action.entity) else {
            continue;
        };
        let dimension = view.dimension;
        if session.get_state() != SessionState::Play {
            continue;
        }
        // the abilities already stop the client from trying, this catches the ones that don't listen
        if !gamemode.allows_editing() {
            stop_break(&mut breaking, dimension, &mut event_writer);
            reject_action(action, &mut rejected);
            continue;
        }
        if let (PlayerActionType::StartItemUseOn, Some(slot)) = (&action.action, action.slot)
            && slot != inventory.held_slot()
        {
            inventory.set_held_slot(slot);
        }

        {
            debug!("block action {:?} at {} face {}", action.action, action.position, action.face);

            match action.action {
                // the client repeats CrackBlock while mining, it only tells us which face the
                // punch particles should come out of
                PlayerActionType::CrackBlock => {
                    if let Some(handler) = breaking.current.as_mut()
                        && handler.targets(action.position)
                    {
                        handler.set_face(action.face);
                    }
                }
                PlayerActionType::StartDestroyBlock | PlayerActionType::ContinueDestroyBlock => {
                    attack_block(action, transform, &mut breaking, dimension, &level, &registry, &mut event_writer);
                }
                PlayerActionType::AbortDestroyBlock | PlayerActionType::StopDestroyBlock => {
                    stop_break(&mut breaking, dimension, &mut event_writer);
                }
                // vanilla treats this as telemetry, but it is the only placement signal chorus can
                // read: the item use transaction it belongs to arrives in a packet bedrock-rs
                // cannot decode yet
                PlayerActionType::StartItemUseOn => {
                    place_block(action.entity, action, transform, &inventory, dimension, &level, &registry, &mut commands, &mut rejected);
                }
                PlayerActionType::PredictDestroyBlock | PlayerActionType::CreativeDestroyBlock => {
                    break_block(
                        action.entity,
                        action,
                        transform,
                        &mut breaking,
                        dimension,
                        &level,
                        &registry,
                        &mut event_writer,
                        &mut commands,
                        &mut rejected,
                    );
                }
                _ => {}
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn place_block(
    player_entity: Entity,
    action: &BlockActionMessage,
    transform: &Transform,
    inventory: &PlayerInventory,
    dimension: i32,
    level: &Level,
    registry: &BlockRegistry,
    commands: &mut Commands,
    rejected: &mut MessageWriter<BlockEditRejected>,
) {
    let Ok(face) = BlockFace::from_index(action.face as usize) else {
        return;
    };
    let position = action.position + face.get_unit_vec().as_ivec3();

    // the block comes from the server's own inventory, never from what the client claims
    let block_id = inventory.held_item().block_runtime_id;
    if block_id == 0 {
        debug!("nothing placeable in the held slot");
        reject(rejected, player_entity, position, BlockActionKind::Place, RejectReason::NotPlaceable);
        return;
    }

    if !in_reach(transform, position) {
        reject(rejected, player_entity, position, BlockActionKind::Place, RejectReason::OutOfReach);
        return;
    }

    let air_id = registry.get_block_id("minecraft:air").unwrap_or(0);
    let current = level.get_block(dimension, position.x, position.y, position.z, 0);
    if current == Some(block_id) {
        return;
    }
    let occupied = current.is_none_or(|current| current != air_id && !is_replaceable(registry, current));
    if occupied {
        debug!("{} is not free", position);
        reject(rejected, player_entity, position, BlockActionKind::Place, RejectReason::Occupied);
        return;
    }

    debug!("placing block {} at {}", block_id, position);

    commands.queue(edit_command(BlockEdit::new(player_entity, BlockActionKind::Place, dimension, position, block_id, action.face)));
}

fn attack_block(
    action: &BlockActionMessage,
    transform: &Transform,
    breaking: &mut BlockBreaking,
    dimension: i32,
    level: &Level,
    registry: &BlockRegistry,
    event_writer: &mut MessageWriter<LevelEventMessage>,
) {
    if !in_reach(transform, action.position) {
        return;
    }

    let Some(block_id) = level.get_block(dimension, action.position.x, action.position.y, action.position.z, 0) else {
        return;
    };

    let speed = match registry.get_components(block_id).and_then(|components| components.get::<MineableComponent>()) {
        Some(mineable) => break_speed(mineable.hardness, !mineable.item_specific_hardness.is_empty()),
        None => return,
    };

    // already mining this block, restarting would reset the animation
    if breaking.current.as_ref().is_some_and(|handler| handler.targets(action.position)) {
        return;
    }

    stop_break(breaking, dimension, event_writer);

    if speed <= 0. {
        return;
    }

    breaking.current = Some(BlockBreakHandler::new(action.position, action.face, block_id, speed));

    event_writer.write(LevelEventMessage {
        dimension_id: dimension,
        event_id: level_event(LevelEvent::StartBlockCracking),
        position: action.position.as_vec3(),
        data: (65535. * speed) as i32,
    });
}

fn stop_break(breaking: &mut BlockBreaking, dimension: i32, event_writer: &mut MessageWriter<LevelEventMessage>) {
    let Some(handler) = breaking.current.take() else {
        return;
    };

    event_writer.write(LevelEventMessage {
        dimension_id: dimension,
        event_id: level_event(LevelEvent::StopBlockCracking),
        position: handler.position().as_vec3(),
        data: 0,
    });
}

#[allow(clippy::too_many_arguments)]
fn break_block(
    player_entity: Entity,
    action: &BlockActionMessage,
    transform: &Transform,
    breaking: &mut BlockBreaking,
    dimension: i32,
    level: &Level,
    registry: &BlockRegistry,
    event_writer: &mut MessageWriter<LevelEventMessage>,
    commands: &mut Commands,
    rejected: &mut MessageWriter<BlockEditRejected>,
) {
    stop_break(breaking, dimension, event_writer);

    let position = action.position;
    if !in_reach(transform, position) {
        reject(rejected, player_entity, position, BlockActionKind::Break, RejectReason::OutOfReach);
        return;
    }

    let Some(block_id) = level.get_block(dimension, position.x, position.y, position.z, 0) else {
        reject(rejected, player_entity, position, BlockActionKind::Break, RejectReason::Stale);
        return;
    };

    let unbreakable = registry
        .get_components(block_id)
        .and_then(|components| components.get::<MineableComponent>())
        .is_none_or(|mineable| mineable.hardness < 0.);

    let Some(air_id) = registry.get_block_id("minecraft:air") else {
        return;
    };
    if unbreakable || block_id == air_id {
        let reason = if unbreakable { RejectReason::Unbreakable } else { RejectReason::Stale };
        reject(rejected, player_entity, position, BlockActionKind::Break, reason);
        return;
    }

    debug!("breaking block {} at {}", block_id, position);

    commands.queue(edit_command(BlockEdit::new(player_entity, BlockActionKind::Break, dimension, position, block_id, action.face)));
}

pub fn update_block_breaking(mut query: Query<(&Transform, &mut BlockBreaking, &ChunkView)>, mut event_writer: MessageWriter<LevelEventMessage>, mut sound_writer: MessageWriter<LevelSoundMessage>) {
    for (transform, mut breaking, view) in query.iter_mut() {
        let dimension_id = view.dimension;
        let Some(handler) = breaking.current.as_mut() else {
            continue;
        };

        match handler.update(transform.position) {
            BreakTick::Continue { fx: false } => {}
            BreakTick::Continue { fx: true } => {
                let position = handler.position().as_vec3();

                event_writer.write(LevelEventMessage {
                    dimension_id,
                    event_id: level_event(LevelEvent::ParticlesCrackBlock),
                    position,
                    data: handler.block_id() | (handler.face() << 24),
                });

                sound_writer.write(LevelSoundMessage {
                    dimension_id,
                    name: SOUND_HIT,
                    position,
                    data: handler.block_id(),
                });
            }
            BreakTick::Stop => stop_break(&mut breaking, dimension_id, &mut event_writer),
        }
    }
}

pub fn broadcast_level_events(mut reader: MessageReader<LevelEventMessage>, mut sessions: Query<(&mut Session, &ChunkView)>) {
    for msg in reader.read() {
        for (mut session, view) in &mut sessions {
            if session.get_state() != SessionState::Play || view.dimension != msg.dimension_id {
                continue;
            }

            session.send(BedrockProtocol::LevelEventPacket(
                LevelEventPacket {
                    event_id: msg.event_id,
                    position: (msg.position.x, msg.position.y, msg.position.z),
                    data: msg.data,
                }
                .into(),
            ));
        }
    }
}

pub fn broadcast_level_sounds(mut reader: MessageReader<LevelSoundMessage>, mut sessions: Query<(&mut Session, &ChunkView)>) {
    for msg in reader.read() {
        for (mut session, view) in &mut sessions {
            if session.get_state() != SessionState::Play || view.dimension != msg.dimension_id {
                continue;
            }

            session.send(BedrockProtocol::LevelSoundEventPacket(
                LevelSoundEventPacket {
                    event_name: msg.name.to_string(),
                    position: (msg.position.x, msg.position.y, msg.position.z),
                    data: msg.data,
                    actor_identifier: ":".to_string(),
                    is_baby_mob: false,
                    is_global: false,
                    entity_unique_id: u64::MAX,
                    fire_at_position: None,
                }
                .into(),
            ));
        }
    }
}

fn in_reach(transform: &Transform, position: IVec3) -> bool {
    let center = position.as_vec3() + Vec3::splat(0.5);

    transform.position.distance_squared(center) <= MAX_REACH_DISTANCE * MAX_REACH_DISTANCE
}

pub fn resync_rejected_edits(mut reader: MessageReader<BlockEditRejected>, level: Res<Level>, mut players: Query<(&mut Session, &ChunkView)>) {
    for rejection in reader.read() {
        let Ok((mut session, view)) = players.get_mut(rejection.entity) else {
            continue;
        };
        let position = rejection.position;
        let Some(block_id) = level.get_block(view.dimension, position.x, position.y, position.z, 0) else {
            continue;
        };
        session.send(BedrockProtocol::UpdateBlockPacket(
            UpdateBlockPacket {
                block_position: NetworkBlockPosition {
                    x: position.x,
                    y: position.y,
                    z: position.z,
                },
                block_runtime_id: block_id as u32,
                flags: 0xB,
                layer: 0,
            }
            .into(),
        ));
    }
}

pub fn broadcast_block_updates(mut reader: MessageReader<BlockUpdatedMessage>, mut query: Query<(&mut Session, &ChunkView)>) {
    for msg in reader.read() {
        for (mut session, view) in &mut query {
            if view.dimension != msg.dimension_id {
                continue;
            }
            session.send(BedrockProtocol::UpdateBlockPacket(
                UpdateBlockPacket {
                    block_position: NetworkBlockPosition { x: msg.x, y: msg.y, z: msg.z },
                    block_runtime_id: msg.block_id as u32,
                    flags: 0xB,
                    layer: msg.layer as u32,
                }
                .into(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::dimension_type::DimensionType;
    use crate::level::generator::r#impl::flat::{FlatGenerator, FlatLayer};
    use bevy_ecs::message::Messages;
    use bevy_ecs::observer::On;

    struct Setup {
        world: World,
        entity: Entity,
        stone: i32,
        air: i32,
    }

    fn setup() -> Setup {
        bevy_tasks::AsyncComputeTaskPool::get_or_init(bevy_tasks::TaskPool::default);

        let mut blocks = BlockRegistry::new();
        blocks.register_all(crate::block::r#impl::DEFINITIONS.iter().copied());
        let air = blocks.get_block_id("minecraft:air").expect("air");
        let stone = blocks.get_block_id("minecraft:stone").expect("stone");

        let mut level = Level::in_memory("block_edit_test", 0);
        let dimension = level.insert_dimension(
            DimensionType::Overworld,
            FlatGenerator {
                layers: vec![FlatLayer { block_id: stone, height: 4 }],
                biome: 1,
                air_id: air,
                min_sub_chunk_y: DimensionType::Overworld.min_sub_chunk_y(),
                sub_chunk_count: DimensionType::Overworld.sub_chunk_count(),
            },
        );
        dimension.request_chunk(0, 0);
        while dimension.tick().is_empty() {
            std::thread::yield_now();
        }

        let mut world = World::new();
        world.insert_resource(blocks);
        world.insert_resource(level);
        world.insert_resource(Messages::<LevelEventMessage>::default());
        world.insert_resource(Messages::<BlockUpdatedMessage>::default());
        world.insert_resource(Messages::<BlockBreakMessage>::default());
        world.insert_resource(Messages::<BlockPlaceMessage>::default());
        world.insert_resource(Messages::<BlockEditRejected>::default());
        let entity = world.spawn_empty().id();
        Setup { world, entity, stone, air }
    }

    fn submit(setup: &mut Setup, kind: BlockActionKind, position: IVec3, block_id: i32) {
        let edit = BlockEdit::new(setup.entity, kind, 0, position, block_id, 0);
        setup.world.commands().queue(edit_command(edit));
        setup.world.flush();
    }

    fn block_at(setup: &Setup, position: IVec3) -> Option<i32> {
        setup.world.resource::<Level>().get_block(0, position.x, position.y, position.z, 0)
    }

    fn rejections(setup: &mut Setup) -> Vec<RejectReason> {
        setup.world.resource_mut::<Messages<BlockEditRejected>>().drain().map(|rejection| rejection.reason).collect()
    }

    #[test]
    fn an_edit_nobody_cancels_reaches_the_level() {
        let mut setup = setup();
        let stone = setup.stone;
        let position = IVec3::new(8, 4, 8);
        submit(&mut setup, BlockActionKind::Place, position, stone);
        assert_eq!(block_at(&setup, position), Some(setup.stone));
        assert!(rejections(&mut setup).is_empty());

        submit(&mut setup, BlockActionKind::Break, position, stone);
        assert_eq!(block_at(&setup, position), Some(setup.air));
    }

    #[test]
    fn an_observer_can_cancel_an_edit_before_it_is_applied() {
        let mut setup = setup();
        let stone = setup.stone;
        setup.world.add_observer(|mut edit: On<BlockEdit>| {
            if edit.position.x == 9 {
                edit.cancel();
            }
        });

        submit(&mut setup, BlockActionKind::Place, IVec3::new(9, 4, 8), stone);
        assert_eq!(block_at(&setup, IVec3::new(9, 4, 8)), Some(setup.air), "a cancelled placement must not touch the level");
        assert_eq!(rejections(&mut setup), vec![RejectReason::Vetoed]);

        submit(&mut setup, BlockActionKind::Place, IVec3::new(10, 4, 8), stone);
        assert_eq!(block_at(&setup, IVec3::new(10, 4, 8)), Some(setup.stone), "the observer leaves other edits alone");
    }

    #[test]
    fn the_first_reason_to_reject_an_edit_is_kept() {
        let mut edit = BlockEdit::new(Entity::PLACEHOLDER, BlockActionKind::Place, 0, IVec3::ZERO, 1, 0);
        assert!(!edit.is_cancelled());
        edit.reject(RejectReason::Obstructed);
        edit.cancel();
        assert_eq!(edit.rejection(), Some(RejectReason::Obstructed));
        assert!(!RejectReason::Obstructed.client_predicted());
        assert!(RejectReason::Vetoed.client_predicted());
    }

    #[test]
    fn placing_the_same_block_twice_is_quietly_ignored() {
        let mut setup = setup();
        let stone = setup.stone;
        let position = IVec3::new(8, 4, 9);
        submit(&mut setup, BlockActionKind::Place, position, stone);
        submit(&mut setup, BlockActionKind::Place, position, stone);
        assert_eq!(block_at(&setup, position), Some(setup.stone));
        assert!(rejections(&mut setup).is_empty());
    }

    #[test]
    fn placing_into_an_occupied_cell_is_rejected() {
        let mut setup = setup();
        let stone = setup.stone;
        let dirt = setup.world.resource::<BlockRegistry>().get_block_id("minecraft:dirt").expect("dirt");
        submit(&mut setup, BlockActionKind::Place, IVec3::new(8, 2, 8), dirt);
        assert_eq!(rejections(&mut setup), vec![RejectReason::Occupied]);
        assert_eq!(block_at(&setup, IVec3::new(8, 2, 8)), Some(stone));
    }
}
