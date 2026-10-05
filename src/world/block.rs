use crate::block::component::mineable_component::MineableComponent;
use crate::entity::components::transform::Transform;
use crate::level::level::Level;
use crate::level::{BlockUpdatedMessage, LevelEventMessage, LevelSoundMessage};
use crate::math::enums::block_face::BlockFace;
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
use bevy_ecs::prelude::{Entity, Query};
use bevy_ecs::system::Res;
use glam::{IVec3, Vec3};
use tracing::debug;

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

pub fn handle_block_actions(
    mut action_reader: MessageReader<BlockActionMessage>,
    mut query: Query<(&Session, &Transform, &mut BlockBreaking, &PlayerInventory, &ChunkView, &Gamemode)>,
    mut level: ResMut<Level>,
    registry: Res<BlockRegistry>,
    mut event_writer: MessageWriter<LevelEventMessage>,
    mut block_writer: MessageWriter<BlockUpdatedMessage>,
    mut break_writer: MessageWriter<BlockBreakMessage>,
    mut place_writer: MessageWriter<BlockPlaceMessage>,
) {
    for action in action_reader.read() {
        let Ok((session, transform, mut breaking, inventory, view, gamemode)) = query.get_mut(action.entity) else {
            continue;
        };
        let dimension = view.dimension;
        if session.get_state() != SessionState::Play {
            continue;
        }
        // the abilities already stop the client from trying, this catches the ones that don't listen
        if !gamemode.allows_editing() {
            stop_break(&mut breaking, dimension, &mut event_writer);
            continue;
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
                    place_block(action.entity, action, transform, inventory, dimension, &mut level, &registry, &mut block_writer, &mut place_writer);
                }
                PlayerActionType::PredictDestroyBlock | PlayerActionType::CreativeDestroyBlock => {
                    break_block(
                        action.entity,
                        action,
                        transform,
                        &mut breaking,
                        dimension,
                        &mut level,
                        &registry,
                        &mut event_writer,
                        &mut block_writer,
                        &mut break_writer,
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
    level: &mut Level,
    registry: &BlockRegistry,
    block_writer: &mut MessageWriter<BlockUpdatedMessage>,
    place_writer: &mut MessageWriter<BlockPlaceMessage>,
) {
    // the block comes from the server's own inventory, never from what the client claims
    let block_id = inventory.held_item().block_runtime_id;
    if block_id == 0 {
        debug!("nothing placeable in the held slot");
        return;
    }

    let Ok(face) = BlockFace::from_index(action.face as usize) else {
        return;
    };

    let position = action.position + face.get_unit_vec().as_ivec3();
    if !in_reach(transform, position) {
        return;
    }

    let air_id = registry.get_block_id("minecraft:air").unwrap_or(0);
    if level.get_block(dimension, position.x, position.y, position.z, 0) != Some(air_id) {
        debug!("{} is not free", position);
        return;
    }

    debug!("placing block {} at {}", block_id, position);

    level.set_block(dimension, position.x, position.y, position.z, 0, block_id, block_writer);

    place_writer.write(BlockPlaceMessage {
        entity: player_entity,
        position,
        block_id,
        face: action.face,
    });
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
    level: &mut Level,
    registry: &BlockRegistry,
    event_writer: &mut MessageWriter<LevelEventMessage>,
    block_writer: &mut MessageWriter<BlockUpdatedMessage>,
    break_writer: &mut MessageWriter<BlockBreakMessage>,
) {
    stop_break(breaking, dimension, event_writer);

    if !in_reach(transform, action.position) {
        return;
    }

    let position = action.position;
    let Some(block_id) = level.get_block(dimension, position.x, position.y, position.z, 0) else {
        return;
    };

    let unbreakable = registry
        .get_components(block_id)
        .and_then(|components| components.get::<MineableComponent>())
        .is_none_or(|mineable| mineable.hardness < 0.);

    if unbreakable {
        return;
    }

    let Some(air_id) = registry.get_block_id("minecraft:air") else {
        return;
    };
    if block_id == air_id {
        return;
    }

    event_writer.write(LevelEventMessage {
        dimension_id: dimension,
        event_id: level_event(LevelEvent::ParticlesDestroyBlock),
        position: position.as_vec3() + Vec3::splat(0.5),
        data: block_id,
    });

    level.set_block(dimension, position.x, position.y, position.z, 0, air_id, block_writer);

    break_writer.write(BlockBreakMessage {
        entity: player_entity,
        position,
        block_id,
    });
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
