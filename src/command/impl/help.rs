use crate::command::command_definition::CommandDefinition;
use crate::command::parameter::{ArgumentType, CommandParameter};
use crate::values;
use chorus_core::permission::PermissionLevel;

const PAGE_SIZE: usize = 7;

pub const HELP_COMMAND: CommandDefinition = CommandDefinition::new("help", "Lists commands or shows how to use one", |context, args| {
    let level = context.permission_level();
    let registry = context.registry();

    let lines = match args.string("command") {
        Some(name) => {
            let command = registry.get(name).filter(|command| command.permission <= level).ok_or_else(|| format!("No command matching \"{name}\" found."))?;
            describe(command)
        }
        None => {
            let mut commands: Vec<&CommandDefinition> = registry.commands().filter(|command| command.permission <= level).collect();
            commands.sort_by_key(|command| command.name.to_lowercase());
            list(&commands, args.int("page").unwrap_or(1).max(1) as usize)
        }
    };

    for line in lines {
        context.reply(line);
    }
    Ok(())
})
.aliases(values!["?"])
.permission(PermissionLevel::Member)
.overloads(crate::overloads![
    [CommandParameter::new("page", ArgumentType::Int).optional()],
    [CommandParameter::command_name("command").optional()],
]);

fn list(commands: &[&CommandDefinition], page: usize) -> Vec<String> {
    let total_pages = commands.len().div_ceil(PAGE_SIZE).max(1);
    let page = page.min(total_pages);

    let mut lines = vec![format!("§3» §b§lHelp§r §8(page {page} of {total_pages}, /help <page>)")];
    for command in commands.iter().skip((page - 1) * PAGE_SIZE).take(PAGE_SIZE) {
        lines.push(format!("  §f/{}§8: §7{}", command.name, command.description));
    }
    lines
}

fn describe(command: &CommandDefinition) -> Vec<String> {
    let mut lines = vec![format!("§3» §b§l/{}§r §8- §7{}", command.name, command.description)];
    lines.extend(command.usage().lines().map(|usage| format!("  §f{usage}")));
    if !command.aliases.is_empty() {
        let mut aliases: Vec<&str> = command.aliases.iter().map(|alias| alias.as_ref()).collect();
        aliases.sort_unstable();
        lines.push(format!("  §7Aliases§8: §f{}", aliases.join(", ")));
    }
    lines.push(format!("  §7Permission§8: §f{}", command.permission.name()));
    lines
}
