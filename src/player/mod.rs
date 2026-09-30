use crate::entity::components::actor_id::ActorId;
use crate::entity::components::transform::Transform;
use crate::level::DimensionId;
use crate::network::BedrockProtocol;
use crate::network::session::Session;
use crate::player::block_break::BlockBreaking;
use crate::player::chunk_view::ChunkView;
use crate::player::forms::PendingForms;
use crate::player::gamemode::Gamemode;
use crate::player::inventory::PlayerInventory;
use bedrock::protocol::v662::types::ActorRuntimeID;
use bedrock::protocol::v2168::enums::PlayerPositionMode;
use bedrock::protocol::v2168::packets::MovePlayerPacket;
use bedrock::protocol::v2168::types::MovePlayerTeleportData;
use bevy_ecs::prelude::*;
use glam::Vec3;

pub mod block_break;
pub mod chunk_view;
pub mod forms;
pub mod gamemode;
pub mod identity;
pub mod inventory;

pub const PLAYER_EYE_HEIGHT: f32 = 1.62;

const TELEPORT_ARRIVAL_DISTANCE: f32 = 1.0;

#[derive(Component, Default)]
#[require(DimensionId, ChunkView, BlockBreaking, PlayerInventory, PendingForms, Gamemode, PendingTeleport)]
pub struct Player;

#[derive(Component, Default)]
pub struct PendingTeleport(Option<Vec3>);

impl PendingTeleport {
    pub(crate) fn accepts(&mut self, reported: Vec3) -> bool {
        match self.0 {
            Some(destination) if reported.distance_squared(destination) > TELEPORT_ARRIVAL_DISTANCE * TELEPORT_ARRIVAL_DISTANCE => false,
            Some(_) => {
                self.0 = None;
                true
            }
            None => true,
        }
    }
}

pub fn teleport(session: &mut Session, actor: &ActorId, transform: &mut Transform, pending: &mut PendingTeleport, feet: Vec3) {
    let eye = feet + Vec3::new(0.0, PLAYER_EYE_HEIGHT, 0.0);
    transform.position = eye;
    pending.0 = Some(eye);

    session.send(BedrockProtocol::MovePlayerPacket(
        MovePlayerPacket {
            player_runtime_id: ActorRuntimeID(actor.runtime_id),
            position: (eye.x, eye.y, eye.z),
            rotation: (0.0, 0.0),
            y_head_rotation: 0.0,
            position_mode: PlayerPositionMode::Teleport,
            on_ground: false,
            riding_runtime_id: ActorRuntimeID(0),
            teleport_data: Some(MovePlayerTeleportData {
                teleportation_cause: 0,
                source_actor_type: 0,
            }),
            tick: 0,
        }
        .into(),
    ));
}
