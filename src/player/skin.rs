use crate::network::BedrockProtocol;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use bedrock::auth::ClientData;
use bedrock::auth::client_data::SkinAnimation;
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

fn animation(frame: &SkinAnimation) -> SerializedSkinAnimationFrame<BedrockProtocol> {
    SerializedSkinAnimationFrame {
        image_width: frame.image_width as u32,
        image_height: frame.image_height as u32,
        image_bytes: frame.image.as_bytes().to_vec(),
        animation_type: match frame.animation_type {
            1 => AnimatedTextureType::Face,
            2 => AnimatedTextureType::Body32x32,
            3 => AnimatedTextureType::Body128x128,
            _ => AnimatedTextureType::None,
        },
        frame_count: frame.frames as f32,
        animation_expression: if frame.animation_expression == 1 {
            AnimationExpression::Blinking
        } else {
            AnimationExpression::Linear
        },
    }
}

impl PlayerAppearance {
    pub fn from_client_data(claims: &ClientData) -> Self {
        let skin_id = claims.skin_id.clone();
        let cape_id = claims.cape_id.clone();
        let skin = SerializedSkin {
            full_id: format!("{skin_id}{cape_id}"),
            skin_id,
            play_fab_id: claims.play_fab_id.clone(),
            skin_resource_patch: claims.skin_resource_patch.clone(),
            skin_image_width: claims.skin_image_width as u32,
            skin_image_height: claims.skin_image_height as u32,
            skin_image_bytes: claims.skin_data.as_bytes().to_vec(),
            animations: claims.animated_image_data.iter().map(animation).collect(),
            cape_image_width: claims.cape_image_width as u32,
            cape_image_height: claims.cape_image_height as u32,
            cape_image_bytes: claims.cape_data.as_bytes().to_vec(),
            geometry_data: claims.skin_geometry.clone(),
            geometry_data_engine_version: claims.skin_geometry_version.clone(),
            animation_data: claims.skin_animation_data.clone(),
            cape_id,
            arm_size: if claims.arm_size == "slim" { ArmSizeType::Slim } else { ArmSizeType::Wide },
            skin_color: i32::from_str_radix(claims.skin_colour.trim_start_matches('#'), 16).unwrap_or_default(),
            persona_pieces: vec![],
            piece_tint_colors: vec![],
            is_premium_skin: claims.premium_skin,
            is_persona_skin: claims.persona_skin,
            is_persona_cape_on_classic_skin: claims.cape_on_classic_skin,
            is_primary_user: true,
            overrides_player_appearance: claims.override_skin,
            trusted_skin_flag: String::new(),
            profile_hash: String::new(),
        };
        Self {
            uuid: Uuid::new_v4(),
            skin,
            device_id: claims.device_id.clone(),
        }
    }
}
