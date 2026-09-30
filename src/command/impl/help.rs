use crate::command::command_definition::CommandDefinition;
use crate::command::parameter::{CommandOverload, CommandParameter, CommandParameterType};
use crate::const_command;
use crate::registry::command_registry::CommandRegistry;
use atomicow::CowArc;
use bedrock::protocol::v898::packets::CommandPermissionLevelString;

const PAGE_SIZE: usize = 7;

pub const HELP_COMMAND: CommandDefinition = const_command! {
    name: "help",
    description: "Provides help/list of commands",
    aliases: ["?"],
    permission: CommandPermissionLevelString::Any,
    overloads: [
        CommandOverload {
            parameters: CowArc::Static(&[
                CommandParameter {
                    name: CowArc::Static("page"),
                    kind: CommandParameterType::Int,
                    optional: true
                }
            ]),
        },
        CommandOverload {
            parameters: CowArc::Static(&[
                CommandParameter {
                    name: CowArc::Static("command"),
                    kind: CommandParameterType::String,
                    optional: true
                }
            ])
        },
    ],
    execute: |context, args| {
        let registry = context.registry();

        let mut name_parts = args.to_vec();
        let mut page = 1;

        if let Some(last) = name_parts.last()
            && let Ok(parsed) = last.parse::<usize>()
        {
            page = parsed.max(1);
            name_parts.pop();
        }

        let name = name_parts.join(" ");
        let lines = if name.is_empty() {
            list(registry, page)
        } else {
            let Some(command) = registry.get(&name) else {
                return Err(format!("No command matching \"{name}\" found."));
            };
            describe(command)
        };

        for line in lines {
            context.reply(line);
        }
        Ok(())
    }
};

fn list(registry: &CommandRegistry, page: usize) -> Vec<String> {
    let mut commands: Vec<&CommandDefinition> = registry.commands().collect();
    commands.sort_by_key(|a| a.name.to_lowercase());

    let total_pages = commands.len().div_ceil(PAGE_SIZE).max(1);
    let page = page.min(total_pages);

    let mut lines = vec![format!("§2--- Showing help page {page} of {total_pages} (/help <page>) ---")];
    for command in commands.iter().skip((page - 1) * PAGE_SIZE).take(PAGE_SIZE) {
        lines.push(format!("§2/{}: §r{}", command.name, command.description));
    }
    lines
}

fn describe(command: &CommandDefinition) -> Vec<String> {
    let mut lines = vec![format!("§e--------- §fHelp: /{} §e---------", command.name), format!("§6Description: §f{}", command.description), format!("§6Usage: §f{}", command.usage().replace('\n', "\n§f"))];

    let mut aliases: Vec<_> = command.aliases.to_vec();
    aliases.sort_unstable();
    if !aliases.is_empty() {
        lines.push(format!("§6Aliases: §f{}", aliases.join(", ")));
    }
    lines
}
