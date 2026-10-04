use crate::network::BedrockProtocol;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use bedrock::protocol::v2168::enums::{AnimatedTextureType, AnimationExpression, ArmSizeType};
use bedrock::protocol::v2168::types::{SerializedSkin, SerializedSkinAnimationFrame};
use bevy_ecs::prelude::Component;
use serde_json::Value;
use uuid::Uuid;

#[derive(Component, Clone, Debug)]
pub struct PlayerAppearance {
    pub uuid: Uuid,
    pub skin: SerializedSkin<BedrockProtocol>,
    pub device_id: String,
}

fn text(claims: &Value, key: &str) -> String {
    claims.get(key).and_then(Value::as_str).unwrap_or_default().to_owned()
}

fn number(claims: &Value, key: &str) -> u32 {
    claims.get(key).and_then(Value::as_u64).unwrap_or_default() as u32
}

fn flag(claims: &Value, key: &str) -> bool {
    claims.get(key).and_then(Value::as_bool).unwrap_or_default()
}

fn bytes(claims: &Value, key: &str) -> Vec<u8> {
    BASE64_STANDARD.decode(text(claims, key)).unwrap_or_default()
}

fn decoded_text(claims: &Value, key: &str) -> String {
    String::from_utf8(bytes(claims, key)).unwrap_or_default()
}

fn animation(frame: &Value) -> SerializedSkinAnimationFrame<BedrockProtocol> {
    SerializedSkinAnimationFrame {
        image_width: number(frame, "ImageWidth"),
        image_height: number(frame, "ImageHeight"),
        image_bytes: bytes(frame, "Image"),
        animation_type: match number(frame, "Type") {
            1 => AnimatedTextureType::Face,
            2 => AnimatedTextureType::Body32x32,
            3 => AnimatedTextureType::Body128x128,
            _ => AnimatedTextureType::None,
        },
        frame_count: frame.get("Frames").and_then(Value::as_f64).unwrap_or(1.0) as f32,
        animation_expression: if number(frame, "AnimationExpression") == 1 {
            AnimationExpression::Blinking
        } else {
            AnimationExpression::Linear
        },
    }
}

impl PlayerAppearance {
    pub fn from_client_data(claims: &Value) -> Self {
        let skin_id = text(claims, "SkinId");
        let cape_id = text(claims, "CapeId");
        let skin = SerializedSkin {
            full_id: format!("{skin_id}{cape_id}"),
            skin_id,
            play_fab_id: text(claims, "PlayFabId"),
            skin_resource_patch: decoded_text(claims, "SkinResourcePatch"),
            skin_image_width: number(claims, "SkinImageWidth"),
            skin_image_height: number(claims, "SkinImageHeight"),
            skin_image_bytes: bytes(claims, "SkinData"),
            animations: claims
                .get("AnimatedImageData")
                .and_then(Value::as_array)
                .map(|frames| frames.iter().map(animation).collect())
                .unwrap_or_default(),
            cape_image_width: number(claims, "CapeImageWidth"),
            cape_image_height: number(claims, "CapeImageHeight"),
            cape_image_bytes: bytes(claims, "CapeData"),
            geometry_data: decoded_text(claims, "SkinGeometryData"),
            geometry_data_engine_version: decoded_text(claims, "SkinGeometryDataEngineVersion"),
            animation_data: decoded_text(claims, "SkinAnimationData"),
            cape_id,
            arm_size: if text(claims, "ArmSize") == "slim" { ArmSizeType::Slim } else { ArmSizeType::Wide },
            skin_color: i32::from_str_radix(text(claims, "SkinColor").trim_start_matches('#'), 16).unwrap_or_default(),
            persona_pieces: vec![],
            piece_tint_colors: vec![],
            is_premium_skin: flag(claims, "PremiumSkin"),
            is_persona_skin: flag(claims, "PersonaSkin"),
            is_persona_cape_on_classic_skin: flag(claims, "CapeOnClassicSkin"),
            is_primary_user: true,
            overrides_player_appearance: flag(claims, "OverrideSkin"),
            trusted_skin_flag: String::new(),
            profile_hash: String::new(),
        };
        Self {
            uuid: Uuid::new_v4(),
            skin,
            device_id: text(claims, "DeviceId"),
        }
    }
}
