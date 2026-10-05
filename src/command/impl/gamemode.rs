use crate::command::command_definition::CommandDefinition;
use crate::command::parameter::{ArgumentType, CommandEnum, CommandParameter};
use crate::entity::components::actor_id::ActorId;
use crate::entity::components::transform::Transform;
use crate::network::session::Session;
use crate::player::gamemode::Gamemode;
use crate::values;
use chorus_core::permission::PermissionLevel;

const GAME_MODES: CommandEnum = CommandEnum::new("GameMode", values!["survival", "creative", "adventure", "spectator", "default", "s", "c", "a", "d"]);

pub const GAMEMODE_COMMAND: CommandDefinition = CommandDefinition::new("gamemode", "Sets your game mode", |context, args| {
    let argument = args.string("gameMode").map(str::to_owned).or_else(|| args.int("gameModeId").map(|id| id.to_string())).unwrap_or_default();
    let gamemode = Gamemode::from_alias(&argument).ok_or_else(|| format!("\"{argument}\" is not a valid game mode."))?;

    let permission = context.permission_level();
    let Some((mut session, mut current, actor, transform)) = context.components_mut::<(&mut Session, &mut Gamemode, &ActorId, &Transform)>() else {
        return Err("must be sent by player!".to_owned());
    };
    if *current == gamemode {
        return Err("Your game mode was not changed.".to_owned());
    }
    current.set(&mut session, actor, transform, permission, gamemode);

    context.reply(format!("Set own game mode to {}", gamemode.name()));
    Ok(())
})
.aliases(values!["gm"])
.permission(PermissionLevel::Operator)
.overloads(crate::overloads![
    [CommandParameter::enumeration("gameMode", GAME_MODES)],
    [CommandParameter::new("gameModeId", ArgumentType::Int)],
]);
