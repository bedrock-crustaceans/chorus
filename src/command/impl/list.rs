use crate::command::command_definition::CommandDefinition;
use crate::config::Config;
use crate::const_command;
use crate::entity::entity::Entity as PlayerEntity;
use crate::player::identity::PlayerIdentity;
use bedrock::protocol::v898::packets::CommandPermissionLevelString;

pub const LIST_COMMAND: CommandDefinition = const_command! {
    name: "list",
    description: "Lists the players currently online.",
    aliases: [],
    permission: CommandPermissionLevelString::Any,
    overloads: [],
    execute: |context, sender, _| {
        let mut names: Vec<&str> = context
            .world()
            .iter_entities()
            .filter(|entity| entity.contains::<PlayerEntity>())
            .filter_map(|entity| entity.get::<PlayerIdentity>().map(|identity| identity.name()))
            .collect();

        names.sort_unstable();

        let max_players = context.resource::<Config>().max_players;

        sender.reply(format!("There are {}/{max_players} players online:", names.len()));

        // the client drops the connection on an empty system message
        if !names.is_empty() {
            sender.reply(names.join(", "));
        }

        Ok(())
    },
};
