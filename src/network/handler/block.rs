use crate::block::component::mineable_component::MineableComponent;
use crate::entity::entity::Entity as PlayerEntity;
use crate::item::item_stack::ItemStack;
use crate::level::level::Level;
use crate::level::{BlockUpdatedMessage, LevelEventMessage, LevelSoundMessage};
use crate::math::enums::block_face::BlockFace;
use crate::network::BedrockProtocol;
use crate::network::handler::PacketReceivedMessage;
use crate::network::session::Session;
use crate::network::session::state::SessionState;
use crate::player::Player;
use crate::player::block_break::{BlockBreakHandler, BreakTick, break_speed};
use crate::registry::block_registry::BlockRegistry;
use bedrock::protocol::v662::enums::PlayerActionType;
use bedrock::protocol::v662::packets::LevelEventPacket;
use bedrock::protocol::v766::enums::LevelEvent;
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

struct BlockAction {
    action: PlayerActionType,
    position: IVec3,
    face: i32,
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

#[derive(Message, Clone, Debug)]
pub struct PlayerDropItemMessage {
    pub entity: Entity,
    pub item: ItemStack,
}

#[derive(Message, Clone, Debug)]
pub struct BlockInteractMessage {
    pub entity: Entity,
    pub position: IVec3,
    pub face: i32,
}

pub fn handle_block_actions(
    mut packet_reader: MessageReader<PacketReceivedMessage>,
    mut query: Query<(&Session, &PlayerEntity, &mut Player)>,
    mut level: ResMut<Level>,
    registry: Res<BlockRegistry>,
    mut event_writer: MessageWriter<LevelEventMessage>,
    mut block_writer: MessageWriter<BlockUpdatedMessage>,
    mut break_writer: MessageWriter<BlockBreakMessage>,
    mut place_writer: MessageWriter<BlockPlaceMessage>,
    mut drop_writer: MessageWriter<PlayerDropItemMessage>,
    mut interact_writer: MessageWriter<BlockInteractMessage>,
) {
    for ev in packet_reader.read() {
        let Ok((session, entity, mut player)) = query.get_mut(ev.entity) else {
            continue;
        };
        if session.get_state() != SessionState::Play {
            continue;
        }

        for action in collect_actions(&ev.packet) {
            debug!("block action {:?} at {} face {}", action.action, action.position, action.face);

            match action.action {
                // the client repeats CrackBlock while mining, it only tells us which face the
                // punch particles should come out of
                PlayerActionType::CrackBlock => {
                    if let Some(handler) = player.block_break.as_mut()
                        && handler.targets(action.position)
                    {
                        handler.set_face(action.face);
                    }
                }
                PlayerActionType::StartDestroyBlock | PlayerActionType::ContinueDestroyBlock => {
                    attack_block(&action, entity, &mut player, &level, &registry, &mut event_writer);
                }
                PlayerActionType::AbortDestroyBlock | PlayerActionType::StopDestroyBlock => {
                    stop_break(&mut player, &mut event_writer);
                }
                // vanilla treats this as telemetry, but it is the only placement signal chorus can
                // read: the item use transaction it belongs to arrives in a packet bedrock-rs
                // cannot decode yet
                PlayerActionType::StartItemUseOn => {
                    place_block(ev.entity, &action, entity, &player, &mut level, &registry, &mut block_writer, &mut place_writer);
                }
                PlayerActionType::PredictDestroyBlock | PlayerActionType::CreativeDestroyBlock => {
                    break_block(ev.entity, &action, entity, &mut player, &mut level, &registry, &mut event_writer, &mut block_writer, &mut break_writer);
                }
                PlayerActionType::DropItem => {
                    drop_writer.write(PlayerDropItemMessage {
                        entity: ev.entity,
                        item: *player.inventory.held_item(),
                    });
                }
                PlayerActionType::InteractWithBlock => {
                    interact_writer.write(BlockInteractMessage {
                        entity: ev.entity,
                        position: action.position,
                        face: action.face,
                    });
                }
                _ => {}
            }
        }
    }
}

fn collect_actions(packet: &BedrockProtocol) -> Vec<BlockAction> {
    match packet {
        BedrockProtocol::PlayerActionPacket(packet) => vec![BlockAction {
            action: packet.action.clone(),
            position: IVec3::new(packet.block_position.x, packet.block_position.y, packet.block_position.z),
            face: packet.face,
        }],
        // only populated when the client runs with server authoritative block breaking
        BedrockProtocol::PlayerAuthInputPacket(packet) => packet
            .player_block_actions
            .iter()
            .flatten()
            .map(|action| BlockAction {
                action: action.action_type.clone(),
                position: {
                    let position = &action.position;

                    IVec3::new(position.x, position.y, position.z)
                },
                face: action.facing,
            })
            .collect(),
        _ => vec![],
    }
}

/// Puts the held block against the face the player clicked.
#[allow(clippy::too_many_arguments)]
fn place_block(
    player_entity: Entity,
    action: &BlockAction,
    entity: &PlayerEntity,
    player: &Player,
    level: &mut Level,
    registry: &BlockRegistry,
    block_writer: &mut MessageWriter<BlockUpdatedMessage>,
    place_writer: &mut MessageWriter<BlockPlaceMessage>,
) {
    // the block comes from the server's own inventory, never from what the client claims
    let block_id = player.inventory.held_item().block_runtime_id;
    if block_id == 0 {
        debug!("nothing placeable in the held slot");
        return;
    }

    let Ok(face) = BlockFace::from_index(action.face as usize) else {
        return;
    };

    let position = action.position + face.get_unit_vec().as_ivec3();
    if !in_reach(entity, position) {
        return;
    }

    let air_id = registry.get_block_id("minecraft:air").unwrap_or(0);
    if level.get_block(0, position.x, position.y, position.z, 0) != Some(air_id) {
        debug!("{} is not free", position);
        return;
    }

    debug!("placing block {} at {}", block_id, position);

    level.set_block(0, position.x, position.y, position.z, 0, block_id, block_writer);

    place_writer.write(BlockPlaceMessage {
        entity: player_entity,
        position,
        block_id,
        face: action.face,
    });
}

fn attack_block(action: &BlockAction, entity: &PlayerEntity, player: &mut Player, level: &Level, registry: &BlockRegistry, event_writer: &mut MessageWriter<LevelEventMessage>) {
    if !in_reach(entity, action.position) {
        return;
    }

    let Some(block_id) = level.get_block(0, action.position.x, action.position.y, action.position.z, 0) else {
        return;
    };

    let speed = match registry.get_components(block_id).and_then(|components| components.get::<MineableComponent>()) {
        Some(mineable) => break_speed(mineable.hardness, !mineable.item_specific_hardness.is_empty()),
        None => return,
    };

    // already mining this block, restarting would reset the animation
    if player.block_break.as_ref().is_some_and(|handler| handler.targets(action.position)) {
        return;
    }

    stop_break(player, event_writer);

    if speed <= 0. {
        return;
    }

    player.block_break = Some(BlockBreakHandler::new(action.position, action.face, block_id, speed));

    event_writer.write(LevelEventMessage {
        event_id: level_event(LevelEvent::StartBlockCracking),
        position: action.position.as_vec3(),
        data: (65535. * speed) as i32,
    });
}

fn stop_break(player: &mut Player, event_writer: &mut MessageWriter<LevelEventMessage>) {
    let Some(handler) = player.block_break.take() else {
        return;
    };

    event_writer.write(LevelEventMessage {
        event_id: level_event(LevelEvent::StopBlockCracking),
        position: handler.position().as_vec3(),
        data: 0,
    });
}

#[allow(clippy::too_many_arguments)]
fn break_block(
    player_entity: Entity,
    action: &BlockAction,
    entity: &PlayerEntity,
    player: &mut Player,
    level: &mut Level,
    registry: &BlockRegistry,
    event_writer: &mut MessageWriter<LevelEventMessage>,
    block_writer: &mut MessageWriter<BlockUpdatedMessage>,
    break_writer: &mut MessageWriter<BlockBreakMessage>,
) {
    stop_break(player, event_writer);

    if !in_reach(entity, action.position) {
        return;
    }

    let position = action.position;
    let Some(block_id) = level.get_block(0, position.x, position.y, position.z, 0) else {
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
        event_id: level_event(LevelEvent::ParticlesDestroyBlock),
        position: position.as_vec3() + Vec3::splat(0.5),
        data: block_id,
    });

    level.set_block(0, position.x, position.y, position.z, 0, air_id, block_writer);

    break_writer.write(BlockBreakMessage {
        entity: player_entity,
        position,
        block_id,
    });
}

pub fn update_block_breaking(mut query: Query<(&PlayerEntity, &mut Player)>, mut event_writer: MessageWriter<LevelEventMessage>, mut sound_writer: MessageWriter<LevelSoundMessage>) {
    for (entity, mut player) in query.iter_mut() {
        let Some(handler) = player.block_break.as_mut() else {
            continue;
        };

        match handler.update(entity.position) {
            BreakTick::Continue { fx: false } => {}
            BreakTick::Continue { fx: true } => {
                let position = handler.position().as_vec3();

                event_writer.write(LevelEventMessage {
                    event_id: level_event(LevelEvent::ParticlesCrackBlock),
                    position,
                    data: handler.block_id() | (handler.face() << 24),
                });

                sound_writer.write(LevelSoundMessage {
                    name: SOUND_HIT,
                    position,
                    data: handler.block_id(),
                });
            }
            BreakTick::Stop => stop_break(&mut player, &mut event_writer),
        }
    }
}

pub fn broadcast_level_events(mut reader: MessageReader<LevelEventMessage>, mut sessions: Query<&mut Session>) {
    for msg in reader.read() {
        for mut session in &mut sessions {
            if session.get_state() != SessionState::Play {
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

pub fn broadcast_level_sounds(mut reader: MessageReader<LevelSoundMessage>, mut sessions: Query<&mut Session>) {
    for msg in reader.read() {
        for mut session in &mut sessions {
            if session.get_state() != SessionState::Play {
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

fn in_reach(entity: &PlayerEntity, position: IVec3) -> bool {
    let center = position.as_vec3() + Vec3::splat(0.5);

    entity.position.distance_squared(center) <= MAX_REACH_DISTANCE * MAX_REACH_DISTANCE
}
