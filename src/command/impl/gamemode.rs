use crate::command::command_definition::CommandDefinition;
use crate::command::parameter::{CommandOverload, CommandParameter, CommandParameterType};
use crate::const_command;
use crate::network::session::Session;
use crate::player::gamemode::Gamemode;
use atomicow::CowArc;
use bedrock::protocol::v898::packets::CommandPermissionLevelString;

pub const GAMEMODE_COMMAND: CommandDefinition = const_command! {
    name: "gamemode",
    description: "Sets a player's game mode",
    aliases: [],
    permission: CommandPermissionLevelString::GameDirectors,
    overloads: [
        CommandOverload {
            parameters: CowArc::Static(&[
                CommandParameter {
                    name: CowArc::Static("gameMode"),
                    kind: CommandParameterType::String,
                    optional: false
                }
            ])
        }
    ],
    execute: |context, args| {
        let Some(&argument) = args.first() else {
            return Err("Usage: /gamemode <gameMode: string>".to_owned());
        };

        let Some(gamemode) = Gamemode::from_alias(argument) else {
            return Err(format!("\"{argument}\" is not a valid game mode."));
        };

        let Some((mut session, mut current)) = context.components_mut::<(&mut Session, &mut Gamemode)>() else {
            return Err("must be sent by player!".to_owned());
        };

        if *current == gamemode {
            return Err("Your game mode was not changed.".to_owned());
        }

        current.set(&mut session, gamemode);

        context.reply(format!("Set own game mode to {}", gamemode.name()));
        Ok(())
    }
};
