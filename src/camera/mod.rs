use crate::network::BedrockProtocol;
use bedrock::protocol::v662::packets::CameraPresetsPacket;
use bedrock::protocol::v662::types::CameraPresets;
use bedrock::protocol::v2193::types::CameraPreset;
use serde::Deserialize;
use std::sync::LazyLock;

const PRESET_FILE: &str = include_str!("presets.json");

#[derive(Deserialize)]
struct PresetFile {
    identifier: String,
    pos_x: Option<f32>,
    pos_y: Option<f32>,
    pos_z: Option<f32>,
    rot_x: Option<f32>,
    rot_y: Option<f32>,
    view_offset: Option<(f32, f32)>,
    entity_offset: Option<(f32, f32, f32)>,
    radius: Option<f32>,
}

impl From<PresetFile> for CameraPreset<BedrockProtocol> {
    fn from(file: PresetFile) -> Self {
        Self {
            name: file.identifier,
            inherit_from: String::new(),
            pos_x: file.pos_x,
            pos_y: file.pos_y,
            pos_z: file.pos_z,
            rot_x: file.rot_x,
            rot_y: file.rot_y,
            rot_speed: None,
            snap_to_target: None,
            horizontal_rot_limit: None,
            vertical_rot_limit: None,
            continue_targeting: None,
            block_listening_radius: None,
            view_offset: file.view_offset,
            entity_offset: file.entity_offset,
            radius: file.radius,
            min_yaw_limit: None,
            max_yaw_limit: None,
            listener: None,
            player_effects: None,
            aim_assist_preset: None,
            control_scheme: None,
            apply_inherited_starting_rotation: false,
            starting_rotation: None,
        }
    }
}

static PRESETS: LazyLock<Vec<CameraPreset<BedrockProtocol>>> = LazyLock::new(|| {
    let files: Vec<PresetFile> = serde_json::from_str(PRESET_FILE).expect("bundled camera presets are valid json");
    files.into_iter().map(Into::into).collect()
});

pub fn preset_names() -> impl Iterator<Item = &'static str> {
    PRESETS.iter().map(|preset| preset.name.as_str())
}

pub fn preset_index(identifier: &str) -> Option<usize> {
    PRESETS.iter().position(|preset| preset.name.eq_ignore_ascii_case(identifier))
}

pub fn preset_has_offsets(index: usize) -> bool {
    PRESETS.get(index).is_some_and(|preset| preset.view_offset.is_some() || preset.entity_offset.is_some())
}

pub fn presets_packet() -> BedrockProtocol {
    BedrockProtocol::CameraPresetsPacket(
        CameraPresetsPacket {
            camera_presets: CameraPresets { presets: PRESETS.clone() },
        }
        .into(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index(identifier: &str) -> usize {
        preset_index(identifier).expect("bundled preset")
    }

    #[test]
    fn lookup_ignores_case_and_matches_the_packet_order() {
        let free = index("MINECRAFT:free");
        assert_eq!(PRESETS[free].name, "minecraft:free");
        assert_eq!(PRESETS[free].pos_x, Some(0.0));
        assert_eq!(preset_index("minecraft:missing"), None);
    }

    #[test]
    fn only_orbit_style_presets_take_offsets() {
        assert!(preset_has_offsets(index("minecraft:follow_orbit")));
        assert!(!preset_has_offsets(index("minecraft:free")));
        assert!(!preset_has_offsets(usize::MAX));
    }
}
