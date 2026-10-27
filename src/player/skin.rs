use crate::network::BedrockProtocol;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use bedrock::auth::ClientData;
use bedrock::auth::client_data::{PersonaPiece, PersonaPieceTintColour, SkinAnimation};
use bedrock::protocol::v2168::enums::{AnimatedTextureType, AnimationExpression, ArmSizeType};
use bedrock::protocol::v2225::enums::PersonaPieceType;
use bedrock::protocol::v2225::types::{PersonaPiecesEntry, PieceTintColorsEntry, SerializedSkin, SerializedSkinAnimationFrame};
use bevy_ecs::prelude::Component;
use uuid::Uuid;

#[derive(Component, Clone, Debug)]
pub struct PlayerAppearance {
    pub uuid: Uuid,
    pub skin: SerializedSkin<BedrockProtocol>,
    pub play_fab_id: String,
    pub device_id: String,
}

fn bytes(base64: &String) -> Vec<u8> {
    BASE64_STANDARD.decode(base64).unwrap_or_default()
}

fn decoded_text(base64: &String) -> String {
    String::from_utf8(bytes(base64)).unwrap_or_default()
}

fn animation(frame: &SkinAnimation) -> SerializedSkinAnimationFrame<BedrockProtocol> {
    SerializedSkinAnimationFrame {
        image_width: frame.image_width as u32,
        image_height: frame.image_height as u32,
        image_bytes: bytes(&frame.image),
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

fn persona_piece_type(piece_type: &str) -> PersonaPieceType {
    match piece_type.strip_prefix("persona_").unwrap_or(piece_type) {
        "skeleton" => PersonaPieceType::Skeleton,
        "body" => PersonaPieceType::Body,
        "skin" => PersonaPieceType::Skin,
        "bottom" => PersonaPieceType::Bottom,
        "feet" => PersonaPieceType::Feet,
        "dress" => PersonaPieceType::Dress,
        "top" => PersonaPieceType::Top,
        "high_pants" | "highpants" => PersonaPieceType::HighPants,
        "hand" | "hands" => PersonaPieceType::Hands,
        "outerwear" => PersonaPieceType::Outerwear,
        "facial_hair" | "facialhair" => PersonaPieceType::FacialHair,
        "mouth" => PersonaPieceType::Mouth,
        "eyes" => PersonaPieceType::Eyes,
        "hair" => PersonaPieceType::Hair,
        "hood" => PersonaPieceType::Hood,
        "back" => PersonaPieceType::Back,
        "face_accessory" | "faceaccessory" => PersonaPieceType::FaceAccessory,
        "head" => PersonaPieceType::Head,
        "legs" => PersonaPieceType::Legs,
        "left_leg" | "leftleg" => PersonaPieceType::LeftLeg,
        "right_leg" | "rightleg" => PersonaPieceType::RightLeg,
        "arms" => PersonaPieceType::Arms,
        "left_arm" | "leftarm" => PersonaPieceType::LeftArm,
        "right_arm" | "rightarm" => PersonaPieceType::RightArm,
        "cape" | "capes" => PersonaPieceType::Capes,
        "classic_skin" | "classicskin" => PersonaPieceType::ClassicSkin,
        "emote" => PersonaPieceType::Emote,
        "coco" => PersonaPieceType::Coco,
        "unsupported" => PersonaPieceType::Unsupported,
        _ => PersonaPieceType::Unknown,
    }
}

fn persona_piece(piece: &PersonaPiece) -> PersonaPiecesEntry<BedrockProtocol> {
    PersonaPiecesEntry {
        piece_id: piece.piece_id.clone(),
        piece_type: persona_piece_type(&piece.piece_type),
        pack_id: Uuid::try_parse(&piece.pack_id).unwrap_or_default(),
        is_default_piece: piece.default,
        product_id: piece.product_id.clone(),
    }
}

fn colour(value: &str) -> i32 {
    u32::from_str_radix(value.trim_start_matches('#'), 16).unwrap_or_default() as i32
}

fn tint_piece_type(piece_type: &str) -> String {
    match piece_type.strip_prefix("persona_").unwrap_or(piece_type) {
        "hand" => "hands".into(),
        other => other.into(),
    }
}

fn tint_colours(tint: &PersonaPieceTintColour) -> PieceTintColorsEntry {
    PieceTintColorsEntry {
        piece_type: tint_piece_type(&tint.piece_type),
        colors: std::array::from_fn(|index| colour(&tint.colours[index])),
    }
}

impl PlayerAppearance {
    pub fn from_client_data(claims: &ClientData) -> Self {
        let skin_id = claims.skin_id.clone();
        let cape_id = claims.cape_id.clone();
        let skin = SerializedSkin {
            full_id: format!("{skin_id}{cape_id}"),
            skin_id,
            skin_resource_patch: claims.skin_resource_patch.clone(),
            skin_image_width: claims.skin_image_width as u32,
            skin_image_height: claims.skin_image_height as u32,
            skin_image_bytes: bytes(&claims.skin_data),
            animations: claims.animated_image_data.iter().map(animation).collect(),
            cape_image_width: claims.cape_image_width as u32,
            cape_image_height: claims.cape_image_height as u32,
            cape_image_bytes: bytes(&claims.cape_data),
            geometry_data: decoded_text(&claims.skin_geometry),
            geometry_data_engine_version: decoded_text(&claims.skin_geometry_version),
            animation_data: decoded_text(&claims.skin_animation_data),
            cape_id,
            arm_size: if claims.arm_size == "slim" { ArmSizeType::Slim } else { ArmSizeType::Wide },
            skin_color: colour(&claims.skin_colour),
            persona_pieces: claims.persona_pieces.iter().map(persona_piece).collect(),
            piece_tint_colors: claims.piece_tint_colours.iter().map(tint_colours).collect(),
            is_premium_skin: claims.premium_skin,
            is_persona_skin: claims.persona_skin,
            is_persona_cape_on_classic_skin: claims.cape_on_classic_skin,
            is_primary_user: true,
            overrides_player_appearance: claims.override_skin,
            trusted_skin_flag: "Unset".into(),
            profile_hash: claims.profile_hash.clone(),
        };
        Self {
            uuid: Uuid::new_v4(),
            skin,
            play_fab_id: claims.play_fab_id.clone(),
            device_id: claims.device_id.clone(),
        }
    }
}
