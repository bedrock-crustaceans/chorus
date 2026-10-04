use crate::command::command_definition::CommandDefinition;
use crate::command::parameter::{ArgumentType, CommandParameter};
use crate::level::DimensionId;
use crate::level::level::Level;
use chorus_core::permission::PermissionLevel;

pub const DIMENSION_COMMAND: CommandDefinition = CommandDefinition::new("dimension", "Debug: moves you to another dimension at the same position", |context, args| {
    let id = args.int("id").unwrap_or_default();
    let name = context.world().resource::<Level>().dimension(id).ok_or_else(|| format!("Dimension {id} isn't registered."))?.name();

    let mut current = context.get_mut::<DimensionId>().ok_or("must be sent by player!")?;
    if current.0 == id {
        return Err(format!("Already in {name}."));
    }
    current.0 = id;

    context.reply(format!("Moving to {name}"));
    Ok(())
})
.permission(PermissionLevel::Operator)
.overloads(crate::overloads![[CommandParameter::new("id", ArgumentType::Int)]]);
