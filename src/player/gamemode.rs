use crate::entity::components::actor_id::ActorId;
use crate::entity::components::transform::Transform;
use crate::network::BedrockProtocol;
use crate::network::session::Session;
use bedrock::protocol::v662::enums::{ActorFlags, GameType};
use bedrock::protocol::v662::packets::{SetActorDataPacket, SetPlayerGameTypePacket, UpdateAbilitiesPacket};
use bedrock::protocol::v662::types::{ActorRuntimeID, DataItem, PropertySyncData};
use bedrock::protocol::v776::enums::AbilitiesIndex;
use bedrock::protocol::v776::types::{SerializedAbilitiesData, SerializedAbilitiesLayer, SerializedLayer};
use bedrock::protocol::v2168::enums::{DataItemType, PlayerPositionMode};
use bedrock::protocol::v2168::packets::MovePlayerPacket;
use bedrock::protocol::v2168::types::MovePlayerTeleportData;
use bevy_ecs::prelude::Component;
use chorus_core::permission::PermissionLevel;

#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Gamemode {
    #[default]
    Survival,
    Creative,
    Adventure,
    Spectator,
}

impl Gamemode {
    pub fn from_alias(alias: &str) -> Option<Self> {
        match alias.to_lowercase().as_str() {
            "survival" | "s" | "0" => Some(Self::Survival),
            "creative" | "c" | "1" => Some(Self::Creative),
            "adventure" | "a" | "2" => Some(Self::Adventure),
            "spectator" | "v" | "view" | "3" => Some(Self::Spectator),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Survival => "Survival",
            Self::Creative => "Creative",
            Self::Adventure => "Adventure",
            Self::Spectator => "Spectator",
        }
    }

    pub fn game_type(&self) -> GameType {
        match self {
            Self::Survival => GameType::Survival,
            Self::Creative => GameType::Creative,
            Self::Adventure => GameType::Adventure,
            Self::Spectator => GameType::Spectator,
        }
    }

    /// Whether the player may break and place blocks.
    pub fn allows_editing(&self) -> bool {
        matches!(self, Self::Survival | Self::Creative)
    }

    /// Whether the player touches the world at all: picking up and dropping items, picking blocks,
    /// using doors and containers, attacking.
    pub fn allows_interaction(&self) -> bool {
        *self != Self::Spectator
    }

    pub fn allows_flying(&self) -> bool {
        matches!(self, Self::Creative | Self::Spectator)
    }

    pub fn has_collision(&self) -> bool {
        *self != Self::Spectator
    }

    /// Whether players in other game modes can see this one. Spectators still see each other.
    pub fn visible(&self) -> bool {
        *self != Self::Spectator
    }

    pub fn invulnerable(&self) -> bool {
        matches!(self, Self::Creative | Self::Spectator)
    }

    fn ability_values(&self, flying: bool) -> u32 {
        let bit = |ability: AbilitiesIndex| 1u32 << ability as u32;
        let mut values = 0;
        if self.allows_flying() {
            values |= bit(AbilitiesIndex::MayFly);
            if flying {
                values |= bit(AbilitiesIndex::Flying);
            }
        }
        if !self.has_collision() {
            values |= bit(AbilitiesIndex::NoClip);
        }
        if self.invulnerable() {
            values |= bit(AbilitiesIndex::Invulnerable);
        }
        if *self == Self::Creative {
            values |= bit(AbilitiesIndex::Instabuild);
        }
        if self.allows_editing() {
            values |= bit(AbilitiesIndex::Build) | bit(AbilitiesIndex::Mine);
        }
        if self.allows_interaction() {
            values |= bit(AbilitiesIndex::DoorsAndSwitches) | bit(AbilitiesIndex::OpenContainers) | bit(AbilitiesIndex::AttackPlayers) | bit(AbilitiesIndex::AttackMobs);
        }
        values
    }

    fn actor_flags(&self) -> i64 {
        let mut flags = (1i64 << ActorFlags::HasGravity as i64) | (1i64 << ActorFlags::Breathing as i64);
        if self.has_collision() {
            flags |= 1i64 << ActorFlags::HasCollision as i64;
        } else {
            flags |= 1i64 << ActorFlags::Silent as i64;
        }
        flags
    }

    /// The abilities the client should run with in this mode. Pass `flying` when the player should
    /// already be in the air, a spectator always is.
    pub fn abilities_packet(&self, actor: &ActorId, permission: PermissionLevel, flying: bool) -> BedrockProtocol {
        BedrockProtocol::UpdateAbilitiesPacket(
            UpdateAbilitiesPacket {
                data: SerializedAbilitiesData {
                    target_player_raw_id: actor.unique_id,
                    player_permissions: 1,
                    command_permissions: permission.into(),
                    layers: vec![SerializedLayer {
                        serialized_layer: SerializedAbilitiesLayer::Base,
                        abilities_set: 0xFFFFF,
                        ability_values: self.ability_values(flying),
                        fly_speed: 0.05,
                        vertical_fly_speed: 1.0,
                        walk_speed: 0.1,
                    }],
                },
            }
            .into(),
        )
    }

    pub fn actor_data_packet(&self, actor: &ActorId) -> BedrockProtocol {
        BedrockProtocol::SetActorDataPacket(
            SetActorDataPacket {
                target_runtime_id: ActorRuntimeID(actor.runtime_id),
                actor_data: vec![DataItem {
                    data_item_id: 0,
                    data_item_type: DataItemType::Int64(self.actor_flags()),
                }],
                synced_properties: PropertySyncData {
                    int_entries_list: vec![],
                    float_entries_list: vec![],
                },
                tick: 0,
            }
            .into(),
        )
    }

    /// Switches the player to `gamemode` and sends the client everything it needs to actually play
    /// by the new rules, not just the label.
    pub fn set(&mut self, session: &mut Session, actor: &ActorId, transform: &Transform, permission: PermissionLevel, gamemode: Gamemode) {
        let previous = *self;
        *self = gamemode;

        session.send(BedrockProtocol::SetPlayerGameTypePacket(
            SetPlayerGameTypePacket {
                player_game_type: gamemode.game_type(),
            }
            .into(),
        ));

        // coming out of spectator mid air into creative shouldn't drop the player out of the sky
        let flying = gamemode.allows_flying() && (!gamemode.has_collision() || !previous.has_collision());
        session.send(gamemode.abilities_packet(actor, permission, flying));
        session.send(gamemode.actor_data_packet(actor));

        if !gamemode.has_collision() {
            // a client standing on the ground when it turns spectator can't sprint while flying,
            // moving it in place lifts it off the ground first
            let position = transform.position;
            session.send(BedrockProtocol::MovePlayerPacket(
                MovePlayerPacket {
                    player_runtime_id: ActorRuntimeID(actor.runtime_id),
                    position: (position.x, position.y, position.z),
                    rotation: (transform.rotation.x, transform.rotation.y),
                    y_head_rotation: transform.rotation.y,
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
    }
}
