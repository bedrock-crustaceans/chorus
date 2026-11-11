use crate::camera::{preset_has_offsets, preset_index};
use crate::command::args::CommandArgs;
use crate::command::command_definition::CommandDefinition;
use crate::command::context::CommandContext;
use crate::command::parameter::{ArgumentType, CommandEnum, CommandParameter};
use crate::entity::components::actor_id::ActorId;
use crate::entity::components::transform::Transform;
use crate::network::BedrockProtocol;
use crate::network::session::Session;
use crate::player::identity::PlayerIdentity;
use crate::player::{PLAYER_EYE_HEIGHT, Player};
use crate::{overloads, values};
use bedrock::protocol::v662::enums::EasingType;
use bedrock::protocol::v662::types::ActorUniqueID;
use bedrock::protocol::v924::packets::CameraInstructionPacket;
use bedrock::protocol::v924::types::{AttachInstruction, Color, EaseData, FadeInstruction, FieldOfViewInstruction, SetInstruction, TargetInstruction, TimeData};
use atomicow::CowArc;
use bevy_ecs::prelude::Entity;
use chorus_core::permission::PermissionLevel;
use glam::{DVec3, Vec2, Vec3};

macro_rules! easings {
    ($($name:literal => $variant:ident),* $(,)?) => {
        const EASE_TYPES: CommandEnum = CommandEnum::new("CameraEaseType", values![$($name),*]);

        fn easing(name: &str) -> Option<EasingType> {
            match name {
                $($name => Some(EasingType::$variant),)*
                _ => None,
            }
        }
    };
}

easings! {
    "linear" => Linear,
    "spring" => Spring,
    "in_quad" => InQuad,
    "out_quad" => OutQuad,
    "in_out_quad" => InOutQuad,
    "in_cubic" => InCubic,
    "out_cubic" => OutCubic,
    "in_out_cubic" => InOutCubic,
    "in_quart" => InQuart,
    "out_quart" => OutQuart,
    "in_out_quart" => InOutQuart,
    "in_quint" => InQuint,
    "out_quint" => OutQuint,
    "in_out_quint" => InOutQuint,
    "in_sine" => InSine,
    "out_sine" => OutSine,
    "in_out_sine" => InOutSine,
    "in_expo" => InExpo,
    "out_expo" => OutExpo,
    "in_out_expo" => InOutExpo,
    "in_circ" => InCirc,
    "out_circ" => OutCirc,
    "in_out_circ" => InOutCirc,
    "in_bounce" => InBounce,
    "out_bounce" => OutBounce,
    "in_out_bounce" => InOutBounce,
    "in_back" => InBack,
    "out_back" => OutBack,
    "in_out_back" => InOutBack,
    "in_elastic" => InElastic,
    "out_elastic" => OutElastic,
    "in_out_elastic" => InOutElastic,
}

const PRESET_NAMES: &[CowArc<'static, str>] = values![
    "minecraft:first_person",
    "minecraft:fixed_boom",
    "minecraft:follow_orbit",
    "minecraft:free",
    "minecraft:third_person",
    "minecraft:third_person_front"
];

const PLAYERS: CommandParameter = CommandParameter::new("players", ArgumentType::Target);
const SET: CommandParameter = CommandParameter::literal("set");
const PRESET: CommandParameter = CommandParameter::soft_enum_with("preset", "CameraPreset", PRESET_NAMES);
const EASE: CommandParameter = CommandParameter::literal("ease");
const EASE_TIME: CommandParameter = CommandParameter::new("easeTime", ArgumentType::Float);
const EASE_TYPE: CommandParameter = CommandParameter::enumeration("easeType", EASE_TYPES);
const POS: CommandParameter = CommandParameter::literal("pos");
const POSITION: CommandParameter = CommandParameter::new("position", ArgumentType::Position);
const ROT: CommandParameter = CommandParameter::literal("rot");
const X_ROT: CommandParameter = CommandParameter::new("xRot", ArgumentType::Value);
const Y_ROT: CommandParameter = CommandParameter::new("yRot", ArgumentType::Value);
const FACING: CommandParameter = CommandParameter::literal("facing");
const FACING_POSITION: CommandParameter = CommandParameter::new("facingPosition", ArgumentType::Position);
const VIEW_OFFSET: CommandParameter = CommandParameter::literal("view_offset");
const VIEW_X: CommandParameter = CommandParameter::new("viewOffsetX", ArgumentType::Float);
const VIEW_Y: CommandParameter = CommandParameter::new("viewOffsetY", ArgumentType::Float);
const ENTITY_OFFSET: CommandParameter = CommandParameter::literal("entity_offset");
const ENTITY_X: CommandParameter = CommandParameter::new("entityOffsetX", ArgumentType::Float);
const ENTITY_Y: CommandParameter = CommandParameter::new("entityOffsetY", ArgumentType::Float);
const ENTITY_Z: CommandParameter = CommandParameter::new("entityOffsetZ", ArgumentType::Float);
const DEFAULT: CommandParameter = CommandParameter::literal("default");

const CLEAR: CommandParameter = CommandParameter::literal("clear");

const FADE: CommandParameter = CommandParameter::literal("fade");
const TIME: CommandParameter = CommandParameter::literal("time");
const FADE_IN: CommandParameter = CommandParameter::new("fadeInSeconds", ArgumentType::Float);
const HOLD: CommandParameter = CommandParameter::new("holdSeconds", ArgumentType::Float);
const FADE_OUT: CommandParameter = CommandParameter::new("fadeOutSeconds", ArgumentType::Float);
const COLOR: CommandParameter = CommandParameter::literal("color");
const RED: CommandParameter = CommandParameter::new("red", ArgumentType::Int);
const GREEN: CommandParameter = CommandParameter::new("green", ArgumentType::Int);
const BLUE: CommandParameter = CommandParameter::new("blue", ArgumentType::Int);

const FOV_SET: CommandParameter = CommandParameter::literal("fov_set");
const FOV_CLEAR: CommandParameter = CommandParameter::literal("fov_clear");
const FOV_VALUE: CommandParameter = CommandParameter::new("fov_value", ArgumentType::Float);
const FOV_EASE_TIME: CommandParameter = CommandParameter::new("easeTime", ArgumentType::Float).optional();
const FOV_EASE_TYPE: CommandParameter = CommandParameter::enumeration("easeType", EASE_TYPES).optional();

const TARGET: CommandParameter = CommandParameter::literal("target");
const TARGET_ENTITY: CommandParameter = CommandParameter::new("targetEntity", ArgumentType::Target);
const CENTER_OFFSET: CommandParameter = CommandParameter::literal("center_offset");
const CENTER_X: CommandParameter = CommandParameter::new("centerOffsetX", ArgumentType::Float);
const CENTER_Y: CommandParameter = CommandParameter::new("centerOffsetY", ArgumentType::Float);
const CENTER_Z: CommandParameter = CommandParameter::new("centerOffsetZ", ArgumentType::Float);

const ATTACH: CommandParameter = CommandParameter::literal("attach_to_entity");
const ATTACH_ENTITY: CommandParameter = CommandParameter::new("attachEntity", ArgumentType::Target);
const DETACH: CommandParameter = CommandParameter::literal("detach_from_entity");

const MAX_FADE_SECONDS: f32 = 10.0;
const MAX_OFFSET: f32 = 64.0;

struct Candidate {
    entity: Entity,
    name: Option<String>,
    actor: ActorId,
    position: Vec3,
    is_player: bool,
}

fn candidates(context: &CommandContext) -> Vec<Candidate> {
    context
        .world()
        .iter_entities()
        .filter_map(|entity| {
            let is_player = entity.contains::<Player>() && entity.contains::<Session>();
            Some(Candidate {
                entity: entity.id(),
                name: entity.get::<PlayerIdentity>().filter(|_| is_player).map(|identity| identity.name().to_owned()),
                actor: *entity.get::<ActorId>()?,
                position: entity.get::<Transform>()?.position,
                is_player,
            })
        })
        .collect()
}

fn select(context: &CommandContext, selector: &str, players_only: bool) -> Result<Vec<Candidate>, String> {
    let mut found = candidates(context);
    found.retain(|candidate| candidate.is_player || !players_only);
    let origin = context.get::<Transform>().map_or(Vec3::ZERO, |transform| transform.position);
    found.sort_by(|a, b| a.position.distance_squared(origin).total_cmp(&b.position.distance_squared(origin)));
    let selected: Vec<Candidate> = if selector.starts_with('@') {
        match selector.to_ascii_lowercase().as_str() {
            "@a" => found.into_iter().filter(|candidate| candidate.is_player).collect(),
            "@e" => found,
            "@s" => found.into_iter().filter(|candidate| candidate.entity == context.sender()).collect(),
            "@p" => found.into_iter().filter(|candidate| candidate.is_player).take(1).collect(),
            _ => return Err(format!("Unsupported selector {selector}")),
        }
    } else {
        found.into_iter().filter(|candidate| candidate.name.as_deref().is_some_and(|name| name.eq_ignore_ascii_case(selector))).collect()
    };
    if selected.is_empty() { Err("No targets matched selector".to_owned()) } else { Ok(selected) }
}

fn single_entity(context: &CommandContext, selector: &str) -> Result<Candidate, String> {
    let mut selected = select(context, selector, false)?;
    selected.truncate(1);
    Ok(selected.remove(0))
}

type Instruction = CameraInstructionPacket<BedrockProtocol>;

fn empty_packet() -> Instruction {
    Instruction {
        set: None,
        clear: None,
        fade: None,
        target: None,
        remove_target: None,
        field_of_view: None,
        spline: None,
        attach: None,
        detach: None,
    }
}

struct Origin {
    feet: DVec3,
    rotation: Vec2,
}

fn origin(context: &CommandContext) -> Origin {
    context.get::<Transform>().map_or(Origin { feet: DVec3::ZERO, rotation: Vec2::ZERO }, |transform| Origin {
        feet: (transform.position - Vec3::new(0.0, PLAYER_EYE_HEIGHT, 0.0)).as_dvec3(),
        rotation: transform.rotation,
    })
}

fn ease_type(args: &CommandArgs) -> Result<EasingType, String> {
    let name = args.string("easeType").unwrap_or("linear");
    easing(name).ok_or_else(|| format!("Invalid easing type {name}"))
}

fn ease_time(args: &CommandArgs) -> Result<Option<f32>, String> {
    match args.float("easeTime") {
        Some(seconds) if seconds >= 0.0 => Ok(Some(seconds)),
        Some(_) => Err("Ease time must not be negative".to_owned()),
        None => Ok(None),
    }
}

fn within(value: f32, limit: f32, what: &str) -> Result<f32, String> {
    if (-limit..=limit).contains(&value) { Ok(value) } else { Err(format!("{what} must be between -{limit} and {limit}")) }
}

fn set_instruction(context: &CommandContext, args: &CommandArgs) -> Result<Instruction, String> {
    let name = args.string("preset").unwrap_or_default();
    let runtime_id = preset_index(name).ok_or("Unknown camera preset name")?;
    let origin = origin(context);

    let ease_data = match ease_time(args)? {
        Some(ease_time) => Some(EaseData { ease_type: ease_type(args)?, ease_time }),
        None => None,
    };

    let resolve = |name: &str| args.position(name).map(|position| position.resolve(origin.feet, origin.rotation).as_vec3().to_array().into());
    let position = resolve("position");
    let facing = resolve("facingPosition");

    let rotation = match (args.value("xRot"), args.value("yRot")) {
        (Some(pitch), Some(yaw)) => {
            let pitch = pitch.resolve(origin.rotation.x as f64) as f32;
            let yaw = yaw.resolve(origin.rotation.y as f64) as f32;
            Some((within(pitch, 90.0, "Camera pitch (X rotation)")?, yaw))
        }
        _ => None,
    };

    let view_offset = match (args.float("viewOffsetX"), args.float("viewOffsetY")) {
        (Some(x), Some(y)) => Some((within(x, MAX_OFFSET, "View offset")?, within(y, MAX_OFFSET, "View offset")?)),
        _ => None,
    };
    let entity_offset = match (args.float("entityOffsetX"), args.float("entityOffsetY"), args.float("entityOffsetZ")) {
        (Some(x), Some(y), Some(z)) => Some((within(x, MAX_OFFSET, "Entity offset")?, within(y, MAX_OFFSET, "Entity offset")?, within(z, MAX_OFFSET, "Entity offset")?)),
        _ => None,
    };
    if (view_offset.is_some() || entity_offset.is_some()) && !preset_has_offsets(runtime_id) {
        return Err("Entity and view offset can only be applied to the follow_orbit camera".to_owned());
    }

    Ok(Instruction {
        set: Some(SetInstruction {
            runtime_id: runtime_id as i32,
            ease_data,
            position,
            rotation,
            facing,
            view_offset,
            entity_offset,
            default_preset: args.has("default").then_some(true),
            remove_ignore_starting_values_component: true,
        }),
        ..empty_packet()
    })
}

fn fade_instruction(args: &CommandArgs) -> Result<Instruction, String> {
    let time_data = match (args.float("fadeInSeconds"), args.float("holdSeconds"), args.float("fadeOutSeconds")) {
        (Some(fade_in_time), Some(wait_time), Some(fade_out_time)) => {
            if ![fade_in_time, wait_time, fade_out_time].iter().all(|seconds| (0.0..=MAX_FADE_SECONDS).contains(seconds)) {
                return Err(format!("Fade durations must be between 0 and {MAX_FADE_SECONDS} seconds"));
            }
            Some(TimeData { fade_in_time, wait_time, fade_out_time })
        }
        _ => None,
    };
    let color = match (args.int("red"), args.int("green"), args.int("blue")) {
        (Some(r), Some(g), Some(b)) => {
            if ![r, g, b].iter().all(|channel| (0..=255).contains(channel)) {
                return Err("Fade color values must be integers from 0 to 255".to_owned());
            }
            Some(Color { r: r as f32 / 255.0, g: g as f32 / 255.0, b: b as f32 / 255.0 })
        }
        _ => None,
    };
    Ok(Instruction {
        fade: Some(FadeInstruction { time_data, color }),
        ..empty_packet()
    })
}

fn fov_instruction(args: &CommandArgs, clear: bool) -> Result<Instruction, String> {
    let field_of_view = args.float("fov_value").unwrap_or(0.0);
    if !clear && !(1.0..=179.0).contains(&field_of_view) {
        return Err("Field of view must be between 1 and 179 degrees".to_owned());
    }
    Ok(Instruction {
        field_of_view: Some(FieldOfViewInstruction {
            field_of_view,
            ease_time: ease_time(args)?.unwrap_or(0.0),
            ease_type: ease_type(args)?,
            clear,
        }),
        ..empty_packet()
    })
}

fn target_instruction(context: &CommandContext, args: &CommandArgs) -> Result<Instruction, String> {
    if args.has("clear") {
        return Ok(Instruction { remove_target: Some(true), ..empty_packet() });
    }
    let entity = single_entity(context, args.string("targetEntity").unwrap_or_default())?;
    let target_center_offset = match (args.float("centerOffsetX"), args.float("centerOffsetY"), args.float("centerOffsetZ")) {
        (Some(x), Some(y), Some(z)) => Some((x, y, z)),
        _ => None,
    };
    let instruction = Instruction {
        target: Some(TargetInstruction {
            target_center_offset,
            actor_unique_id: ActorUniqueID(entity.actor.unique_id),
        }),
        ..empty_packet()
    };
    Ok(instruction)
}

fn build(context: &CommandContext, args: &CommandArgs) -> Result<Instruction, String> {
    if args.has("set") {
        set_instruction(context, args)
    } else if args.has("target") {
        target_instruction(context, args)
    } else if args.has("clear") {
        Ok(Instruction { clear: Some(true), ..empty_packet() })
    } else if args.has("fade") {
        fade_instruction(args)
    } else if args.has("fov_set") {
        fov_instruction(args, false)
    } else if args.has("fov_clear") {
        fov_instruction(args, true)
    } else if args.has("attach_to_entity") {
        let entity = single_entity(context, args.string("attachEntity").unwrap_or_default())?;
        Ok(Instruction {
            attach: Some(AttachInstruction { actor_unique_id: entity.actor.unique_id }),
            ..empty_packet()
        })
    } else if args.has("detach_from_entity") {
        Ok(Instruction { detach: Some(true), ..empty_packet() })
    } else {
        Err("Invalid camera command mode".to_owned())
    }
}

pub static CAMERA_COMMAND: CommandDefinition = CommandDefinition::new("camera", "Issues a camera instruction", |context, args| {
    let targets = select(context, args.string("players").unwrap_or_default(), true)?;
    let instruction = build(context, args)?;

    let world = context.world_mut();
    for target in &targets {
        if let Some(mut session) = world.get_mut::<Session>(target.entity) {
            session.send(BedrockProtocol::CameraInstructionPacket(instruction.clone().into()));
        }
    }

    let names: Vec<&str> = targets.iter().filter_map(|target| target.name.as_deref()).collect();
    context.reply(format!("Sent a camera instruction to the following players: {}", names.join(", ")));
    Ok(())
})
.permission(PermissionLevel::Operator)
.overloads(overloads![
    [PLAYERS, CLEAR],
    [PLAYERS, FADE],
    [PLAYERS, FADE, TIME, FADE_IN, HOLD, FADE_OUT],
    [PLAYERS, FADE, COLOR, RED, GREEN, BLUE],
    [PLAYERS, FADE, TIME, FADE_IN, HOLD, FADE_OUT, COLOR, RED, GREEN, BLUE],
    [PLAYERS, FADE, COLOR, RED, GREEN, BLUE, TIME, FADE_IN, HOLD, FADE_OUT],
    [PLAYERS, FOV_SET, FOV_VALUE, FOV_EASE_TIME, FOV_EASE_TYPE],
    [PLAYERS, FOV_CLEAR, FOV_EASE_TIME, FOV_EASE_TYPE],
    [PLAYERS, TARGET, CLEAR],
    [PLAYERS, TARGET, TARGET_ENTITY],
    [PLAYERS, TARGET, TARGET_ENTITY, CENTER_OFFSET, CENTER_X, CENTER_Y, CENTER_Z],
    [PLAYERS, ATTACH, ATTACH_ENTITY],
    [PLAYERS, DETACH],
    [PLAYERS, SET, PRESET],
    [PLAYERS, SET, PRESET, DEFAULT],
    [PLAYERS, SET, PRESET, POS, POSITION],
    [PLAYERS, SET, PRESET, ROT, X_ROT, Y_ROT],
    [PLAYERS, SET, PRESET, FACING, FACING_POSITION],
    [PLAYERS, SET, PRESET, POS, POSITION, ROT, X_ROT, Y_ROT],
    [PLAYERS, SET, PRESET, POS, POSITION, FACING, FACING_POSITION],
    [PLAYERS, SET, PRESET, VIEW_OFFSET, VIEW_X, VIEW_Y],
    [PLAYERS, SET, PRESET, ENTITY_OFFSET, ENTITY_X, ENTITY_Y, ENTITY_Z],
    [PLAYERS, SET, PRESET, VIEW_OFFSET, VIEW_X, VIEW_Y, ENTITY_OFFSET, ENTITY_X, ENTITY_Y, ENTITY_Z],
    [PLAYERS, SET, PRESET, EASE, EASE_TIME, EASE_TYPE],
    [PLAYERS, SET, PRESET, EASE, EASE_TIME, EASE_TYPE, DEFAULT],
    [PLAYERS, SET, PRESET, EASE, EASE_TIME, EASE_TYPE, POS, POSITION],
    [PLAYERS, SET, PRESET, EASE, EASE_TIME, EASE_TYPE, ROT, X_ROT, Y_ROT],
    [PLAYERS, SET, PRESET, EASE, EASE_TIME, EASE_TYPE, FACING, FACING_POSITION],
    [PLAYERS, SET, PRESET, EASE, EASE_TIME, EASE_TYPE, POS, POSITION, ROT, X_ROT, Y_ROT],
    [PLAYERS, SET, PRESET, EASE, EASE_TIME, EASE_TYPE, POS, POSITION, FACING, FACING_POSITION],
    [PLAYERS, SET, PRESET, EASE, EASE_TIME, EASE_TYPE, VIEW_OFFSET, VIEW_X, VIEW_Y],
    [PLAYERS, SET, PRESET, EASE, EASE_TIME, EASE_TYPE, ENTITY_OFFSET, ENTITY_X, ENTITY_Y, ENTITY_Z],
    [PLAYERS, SET, PRESET, EASE, EASE_TIME, EASE_TYPE, VIEW_OFFSET, VIEW_X, VIEW_Y, ENTITY_OFFSET, ENTITY_X, ENTITY_Y, ENTITY_Z],
]);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_soft_enum_defaults_list_the_bundled_presets_in_runtime_id_order() {
        let listed: Vec<&str> = PRESET_NAMES.iter().map(|value| &**value).collect();
        assert_eq!(listed, crate::camera::preset_names().collect::<Vec<_>>());
    }

    #[test]
    fn registering_the_command_seeds_the_soft_enum() {
        let mut registry = crate::registry::command_registry::CommandRegistry::new();
        registry.register(&CAMERA_COMMAND);
        assert_eq!(registry.soft_enum("CameraPreset").map(indexmap::IndexSet::len), Some(PRESET_NAMES.len()));
    }
}
