use crate::command::command_definition::CommandDefinition;
use crate::command::context::CommandContext;
use crate::command::parameter::{ArgumentType, CommandParameter};
use crate::entity::components::transform::Transform;
use crate::level::{DimensionId, Level};
use crate::server::pregen::{MAX_CHUNKS, Pregen};
use chorus_core::permission::PermissionLevel;

pub const GENERATE_COMMAND: CommandDefinition = CommandDefinition::new("generate", "Pregenerates chunks around you or in a rectangle", |context, args| {
    if args.has("stop") {
        let pregen = context.world_mut().remove_resource::<Pregen>().ok_or("nothing is being pregenerated")?;
        context.reply(format!("Stopped pregenerating at {}", pregen.progress()));
        return Ok(());
    }
    if args.has("status") {
        let progress = context.world().get_resource::<Pregen>().map(Pregen::progress);
        context.reply(progress.map_or_else(|| "Nothing is being pregenerated".to_owned(), |progress| format!("Pregenerating {progress}")));
        return Ok(());
    }

    let (origin, dimension) = origin(context);
    let (from, to) = match args.int("radius") {
        Some(radius) if radius < 0 => return Err("the radius can't be negative".to_owned()),
        Some(radius) => {
            let (x, z) = (origin.0 >> 4, origin.1 >> 4);
            ((x - radius, z - radius), (x + radius, z + radius))
        }
        None => {
            let coordinate = |name: &str, current: i32| args.value(name).map_or(current, |value| value.resolve(current as f64).floor() as i32);
            let (from_x, from_z) = (coordinate("fromX", origin.0), coordinate("fromZ", origin.1));
            let (to_x, to_z) = (coordinate("toX", origin.0), coordinate("toZ", origin.1));
            ((from_x.min(to_x) >> 4, from_z.min(to_z) >> 4), (from_x.max(to_x) >> 4, from_z.max(to_z) >> 4))
        }
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
})
.aliases(crate::values!["pregen"])
.permission(PermissionLevel::Admin)
.overloads(crate::overloads![
    [CommandParameter::new("radius", ArgumentType::Int)],
    [
        CommandParameter::new("fromX", ArgumentType::Value),
        CommandParameter::new("fromZ", ArgumentType::Value),
        CommandParameter::new("toX", ArgumentType::Value),
        CommandParameter::new("toZ", ArgumentType::Value),
    ],
    [CommandParameter::literal("stop")],
    [CommandParameter::literal("status")],
]);

/// The sender's block x and z, or the spawn for the console, and their dimension.
fn origin(context: &CommandContext) -> ((i32, i32), i32) {
    let dimension = context.get::<DimensionId>().map_or(0, |dimension| dimension.0);
    if let Some(transform) = context.get::<Transform>() {
        return ((transform.position.x.floor() as i32, transform.position.z.floor() as i32), dimension);
    }
    let spawn = context.world().get_resource::<Level>().map_or_else(Default::default, |level| level.spawn);
    ((spawn.x, spawn.z), dimension)
}
