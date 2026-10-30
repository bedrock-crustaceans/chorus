use crate::command::args::Coordinate;
use crate::command::command_definition::CommandDefinition;
use crate::command::parameter::{ArgumentType, CommandParameter};
use crate::entity::components::actor_id::ActorId;
use crate::entity::components::transform::Transform;
use crate::network::session::Session;
use crate::player::identity::PlayerIdentity;
use crate::player::{PLAYER_EYE_HEIGHT, PendingTeleport, TeleportOptions, teleport};
use chorus_core::permission::PermissionLevel;
use glam::Vec2;
use glam::Vec3;

pub static TP_COMMAND: CommandDefinition = CommandDefinition::new("tp", "Teleports you to a position or a player", |context, args| {
    let feet = context.get::<Transform>().ok_or("must be sent by player!")?.position - Vec3::new(0.0, PLAYER_EYE_HEIGHT, 0.0);
    let rotation = context.get::<Transform>().map(|transform| transform.rotation).unwrap_or_default();

    let centered = |coordinate: Coordinate| match coordinate {
        Coordinate::Absolute(value) if value.fract() == 0.0 => Coordinate::Absolute(value + 0.5),
        other => other,
    };
    let destination = match args.position("destination") {
        Some(position) => {
            let position = if position.is_local() { position } else { crate::command::args::CommandPosition { x: centered(position.x), z: centered(position.z), ..position } };
            position.resolve(feet.as_dvec3(), rotation).as_vec3()
        }
        None => {
            let name = args.string("player").unwrap_or_default();
            let target = context
                .world()
                .iter_entities()
                .find(|entity| entity.get::<PlayerIdentity>().is_some_and(|identity| identity.name().eq_ignore_ascii_case(name)))
                .and_then(|entity| entity.get::<Transform>())
                .ok_or_else(|| format!("No player named \"{name}\" is online."))?;
            target.position - Vec3::new(0.0, PLAYER_EYE_HEIGHT, 0.0)
        }
    };

    let facing_rotation = args.position("lookAtPosition").map(|look_position| {
        let look_position = if look_position.is_local() {
            look_position
        } else {
            crate::command::args::CommandPosition {
                x: centered(look_position.x),
                z: centered(look_position.z),
                ..look_position
            }
        };
        let look_at = look_position.resolve(feet.as_dvec3(), rotation).as_vec3();
        let delta = look_at - destination;
        if delta.length_squared() <= f32::EPSILON * f32::EPSILON {
            rotation
        } else {
            let horizontal = (delta.x * delta.x + delta.z * delta.z).sqrt();
            Vec2::new(
                (-delta.y).atan2(horizontal).to_degrees(),
                (-delta.x).atan2(delta.z).to_degrees(),
            )
        }
    });

    let Some((mut session, actor, mut transform, mut pending)) = context.components_mut::<(&mut Session, &ActorId, &mut Transform, &mut PendingTeleport)>() else {
        return Err("must be sent by player!".to_owned());
    };
    teleport(&mut session, actor, &mut transform, &mut pending, destination, Some(TeleportOptions { rotation: facing_rotation }));

    context.reply(format!("Teleported to {:.2}, {:.2}, {:.2}", destination.x, destination.y, destination.z));
    Ok(())
})
.aliases(crate::values!["teleport"])
.permission(PermissionLevel::Operator)
.overloads(crate::overloads![
    [CommandParameter::new("destination", ArgumentType::Position)],
    [CommandParameter::new("destination", ArgumentType::Position), CommandParameter::literal("facing"), CommandParameter::new("lookAtPosition", ArgumentType::Position)],
    [CommandParameter::new("player", ArgumentType::Target)],
]);
