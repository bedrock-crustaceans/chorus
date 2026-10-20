use crate::command::command_definition::CommandDefinition;
use crate::command::parameter::{ArgumentType, CommandParameter};
use crate::values;
use chorus_core::permission::PermissionLevel;

const PAGE_SIZE: usize = 7;

pub static HELP_COMMAND: CommandDefinition = CommandDefinition::new("help", "Lists commands or shows how to use one", |context, args| {
    let level = context.permission_level();
    let registry = context.registry();

    let lines = match args.string("command") {
        Some(name) => {
            let command = registry.get(name).filter(|command| command.permission <= level).ok_or_else(|| format!("No help for {}", name.to_lowercase()))?;
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

    let mut lines = vec![format!("§2--- Showing help page {page} of {total_pages} (/help <page>) ---")];
    for command in commands.iter().skip((page - 1) * PAGE_SIZE).take(PAGE_SIZE) {
        lines.extend(usage_lines(command));
    }
    lines.push("§2Tip: Use the <tab> key while typing a command to auto-complete the command or its arguments".to_owned());
    lines
}

fn describe(command: &CommandDefinition) -> Vec<String> {
    let mut lines = vec![format!("§e{}", command.name), format!("§e{}", command.description), "Usage:".to_owned()];
    lines.extend(usage_lines(command));
    lines
}

fn usage_lines(command: &CommandDefinition) -> impl Iterator<Item = String> {
    command.usage().lines().map(|usage| format!("- {usage}")).collect::<Vec<_>>().into_iter()
}
