use crate::command::command_definition::CommandDefinition;
use crate::command::parameter::{CommandOverload, CommandParameter, CommandParameterType};
use crate::const_command;
use crate::level::level::Level;
use atomicow::CowArc;
use bedrock::protocol::v898::packets::CommandPermissionLevelString;

pub const DIMENSION_COMMAND: CommandDefinition = const_command! {
    name: "dimension",
    description: "Debug: moves you to another dimension at the same position",
    aliases: [],
    permission: CommandPermissionLevelString::GameDirectors,
    overloads: [
        CommandOverload {
            parameters: CowArc::Static(&[
                CommandParameter {
                    name: CowArc::Static("id"),
                    kind: CommandParameterType::Int,
                    optional: false
                }
            ])
        }
    ],
    execute: |context, sender, args| {
        let Some(id) = args.first().and_then(|argument| argument.parse::<i32>().ok()) else {
            return Err("Usage: /dimension <id: int>".to_owned());
        };

        let Some(dimension) = context.world().resource::<Level>().dimension(id) else {
            return Err(format!("Dimension {id} isn't registered."));
        };
        let name = dimension.name();

        let (_, player) = sender.split();
        let Some(player) = player else {
            return Err("must be sent by player!".to_owned());
        };
        if player.dimension == id {
            return Err(format!("Already in {name}."));
        }
        player.dimension = id;

        sender.reply(format!("Moving to {name}"));
        Ok(())
    }
};
