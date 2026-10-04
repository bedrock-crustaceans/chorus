use crate::command::command_definition::CommandDefinition;
use crate::command::context::CommandContext;
use crate::command::parameter::{CommandOverload, CommandParameter, CommandParameterType};
use crate::const_command;
use crate::entity::components::transform::Transform;
use crate::level::{DimensionId, Level};
use crate::server::pregen::{MAX_CHUNKS, Pregen};
use atomicow::CowArc;
use bedrock::protocol::v898::packets::CommandPermissionLevelString;

const USAGE: &str = "Usage: /generate <radius>, /generate <fromX> <fromZ> <toX> <toZ> (block coordinates, ~ for relative), /generate status or /generate stop";

pub const GENERATE_COMMAND: CommandDefinition = const_command! {
    name: "generate",
    description: "Pregenerates chunks around you or in a rectangle",
    aliases: ["pregen"],
    permission: CommandPermissionLevelString::Admin,
    overloads: [
        CommandOverload {
            parameters: CowArc::Static(&[
                CommandParameter {
                    name: CowArc::Static("radius"),
                    kind: CommandParameterType::Int,
                    optional: false
                }
            ])
        },
        CommandOverload {
            parameters: CowArc::Static(&[
                CommandParameter {
                    name: CowArc::Static("fromX"),
                    kind: CommandParameterType::Value,
                    optional: false
                },
                CommandParameter {
                    name: CowArc::Static("fromZ"),
                    kind: CommandParameterType::Value,
                    optional: false
                },
                CommandParameter {
                    name: CowArc::Static("toX"),
                    kind: CommandParameterType::Value,
                    optional: false
                },
                CommandParameter {
                    name: CowArc::Static("toZ"),
                    kind: CommandParameterType::Value,
                    optional: false
                }
            ])
        },
        CommandOverload {
            parameters: CowArc::Static(&[
                CommandParameter {
                    name: CowArc::Static("action"),
                    kind: CommandParameterType::String,
                    optional: false
                }
            ])
        }
    ],
    execute: |context, args| {
        let (origin, dimension) = origin(context);
        let (from, to) = match *args {
            ["stop"] => {
                let Some(pregen) = context.world_mut().remove_resource::<Pregen>() else {
                    return Err("nothing is being pregenerated".to_owned());
                };
                context.reply(format!("Stopped pregenerating at {}", pregen.progress()));
                return Ok(());
            }
            ["status"] => {
                let progress = context.world().get_resource::<Pregen>().map(Pregen::progress);
                context.reply(progress.map_or_else(|| "Nothing is being pregenerated".to_owned(), |progress| format!("Pregenerating {progress}")));
                return Ok(());
            }
            [radius] => {
                let radius: i32 = radius.parse().ok().filter(|radius| *radius >= 0).ok_or_else(|| USAGE.to_owned())?;
                let (x, z) = (origin.0 >> 4, origin.1 >> 4);
                ((x - radius, z - radius), (x + radius, z + radius))
            }
            [from_x, from_z, to_x, to_z] => {
                let parse = |argument: &str, current: i32| coordinate(argument, current).ok_or_else(|| USAGE.to_owned());
                let (from_x, from_z) = (parse(from_x, origin.0)?, parse(from_z, origin.1)?);
                let (to_x, to_z) = (parse(to_x, origin.0)?, parse(to_z, origin.1)?);
                ((from_x.min(to_x) >> 4, from_z.min(to_z) >> 4), (from_x.max(to_x) >> 4, from_z.max(to_z) >> 4))
            }
            _ => return Err(USAGE.to_owned()),
        };

        let count = Pregen::chunk_count(to.0 as i64 - from.0 as i64 + 1, to.1 as i64 - from.1 as i64 + 1);
        if count > MAX_CHUNKS {
            return Err(format!("that is {count} chunks, the limit is {MAX_CHUNKS}"));
        }
        if context.world().get_resource::<Pregen>().is_some() {
            return Err("already pregenerating, use /generate stop first".to_owned());
        }
        if context.world().get_resource::<Level>().and_then(|level| level.dimension(dimension)).is_none() {
            return Err(format!("dimension {dimension} does not exist"));
        }

        context.world_mut().insert_resource(Pregen::new(dimension, from, to));
        context.reply(format!("Pregenerating {count} chunks from chunk {}, {} to {}, {}", from.0, from.1, to.0, to.1));
        Ok(())
    },
};

fn origin(context: &CommandContext) -> ((i32, i32), i32) {
    let dimension = context.get::<DimensionId>().map_or(0, |dimension| dimension.0);
    if let Some(transform) = context.get::<Transform>() {
        return ((transform.position.x.floor() as i32, transform.position.z.floor() as i32), dimension);
    }
    let spawn = context.world().get_resource::<Level>().map_or_else(Default::default, |level| level.spawn);
    ((spawn.x, spawn.z), dimension)
}

fn coordinate(argument: &str, current: i32) -> Option<i32> {
    match argument.strip_prefix('~') {
        Some("") => Some(current),
        Some(offset) => current.checked_add(offset.parse().ok()?),
        None => argument.parse().ok(),
    }
}
