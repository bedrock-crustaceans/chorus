use crate::command::command_definition::CommandDefinition;
use crate::config::Config;
use crate::const_command;
use crate::player::Player;
use crate::player::identity::PlayerIdentity;
use bedrock::protocol::v898::packets::CommandPermissionLevelString;

pub const LIST_COMMAND: CommandDefinition = const_command! {
    name: "list",
    description: "Lists the players currently online.",
    aliases: [],
    permission: CommandPermissionLevelString::Any,
    overloads: [],
    execute: |context, _| {
        let mut names: Vec<String> = context
            .world()
            .iter_entities()
            .filter(|entity| entity.contains::<Player>())
            .filter_map(|entity| entity.get::<PlayerIdentity>().map(|identity| identity.name().to_owned()))
            .collect();

        names.sort_unstable();

        let max_players = context.resource::<Config>().server.max_players;

        context.reply(format!("There are {}/{max_players} players online:", names.len()));

        // the client drops the connection on an empty system message
        if !names.is_empty() {
            context.reply(names.join(", "));
        }

        Ok(())
    },
};
