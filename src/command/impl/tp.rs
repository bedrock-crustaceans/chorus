use crate::command::command_definition::CommandDefinition;
use crate::command::parameter::{CommandOverload, CommandParameter, CommandParameterType};
use crate::const_command;
use crate::entity::components::actor_id::ActorId;
use crate::entity::components::transform::Transform;
use crate::network::session::Session;
use crate::player::{PLAYER_EYE_HEIGHT, PendingTeleport, teleport};
use atomicow::CowArc;
use bedrock::protocol::v898::packets::CommandPermissionLevelString;
use glam::Vec3;

pub const TP_COMMAND: CommandDefinition = const_command! {
    name: "tp",
    description: "Teleports you to a position",
    aliases: ["teleport"],
    permission: CommandPermissionLevelString::GameDirectors,
    overloads: [
        CommandOverload {
            parameters: CowArc::Static(&[
                CommandParameter {
                    name: CowArc::Static("destination"),
                    kind: CommandParameterType::Position,
                    optional: false
                }
            ])
        }
    ],
    execute: |context, args| {
        let usage = || "Usage: /tp <x> <y> <z>, each optionally relative with ~".to_owned();
        let &[x, y, z] = args else {
            return Err(usage());
        };

        let Some((mut session, actor, mut transform, mut pending)) = context.components_mut::<(&mut Session, &ActorId, &mut Transform, &mut PendingTeleport)>() else {
            return Err("must be sent by player!".to_owned());
        };
        let feet = transform.position - Vec3::new(0.0, PLAYER_EYE_HEIGHT, 0.0);

        let (Some(x), Some(y), Some(z)) = (coordinate(x, feet.x, true), coordinate(y, feet.y, false), coordinate(z, feet.z, true)) else {
            return Err(usage());
        };
        let destination = Vec3::new(x, y, z);

        teleport(&mut session, actor, &mut transform, &mut pending, destination);

        context.reply(format!("Teleported to {:.2}, {:.2}, {:.2}", destination.x, destination.y, destination.z));
        Ok(())
    }
};

fn coordinate(argument: &str, current: f32, center_whole: bool) -> Option<f32> {
    if let Some(offset) = argument.strip_prefix('~') {
        let offset = if offset.is_empty() { 0.0 } else { offset.parse::<f64>().ok()? };
        return Some((current as f64 + offset) as f32);
    }

    let value = argument.parse::<f64>().ok()?;
    let centered = center_whole && !argument.contains('.');
    Some((if centered { value + 0.5 } else { value }) as f32)
}
