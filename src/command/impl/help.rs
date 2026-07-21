use crate::command::command::{Command, CommandResult};
use crate::command::parameter::{CommandOverload, CommandParameter, CommandParameterType};
use crate::command::registry::CommandRegistry;
use crate::command::sender::CommandSender;

const PAGE_SIZE: usize = 7;

pub struct HelpCommand;

impl HelpCommand {
    fn list(&self, registry: &CommandRegistry, sender: &mut CommandSender, page: usize) {
        let mut commands: Vec<&dyn Command> = registry.commands().collect();
        commands.sort_by(|a, b| a.name().to_lowercase().cmp(&b.name().to_lowercase()));

        let total_pages = commands.len().div_ceil(PAGE_SIZE).max(1);
        let page = page.min(total_pages);

        sender.reply(format!("§2--- Showing help page {page} of {total_pages} (/help <page>) ---"));

        for command in commands.iter().skip((page - 1) * PAGE_SIZE).take(PAGE_SIZE) {
            sender.reply(format!("§2/{}: §r{}", command.name(), command.description()));
        }
    }

    fn describe(&self, command: &dyn Command, sender: &mut CommandSender) {
        sender.reply(format!("§e--------- §fHelp: /{} §e---------", command.name()));
        sender.reply(format!("§6Description: §f{}", command.description()));
        sender.reply(format!("§6Usage: §f{}", command.usage().replace('\n', "\n§f")));

        let mut aliases: Vec<&str> = command.aliases().to_vec();
        aliases.sort_unstable();
        if !aliases.is_empty() {
            sender.reply(format!("§6Aliases: §f{}", aliases.join(", ")));
        }
    }
}

impl Command for HelpCommand {
    fn name(&self) -> &str {
        "help"
    }

    fn description(&self) -> &str {
        "Provides help/list of commands"
    }

    fn aliases(&self) -> &[&'static str] {
        &["?"]
    }

    fn overloads(&self) -> Vec<CommandOverload> {
        vec![
            CommandOverload::new(vec![CommandParameter::new("page", CommandParameterType::Int, true)]),
            CommandOverload::new(vec![CommandParameter::new("command", CommandParameterType::String, true)]),
        ]
    }

    fn execute(&self, registry: &CommandRegistry, sender: &mut CommandSender, args: &[&str]) -> CommandResult {
        let mut name_parts = args.to_vec();
        let mut page = 1;

        if let Some(last) = name_parts.last()
            && let Ok(parsed) = last.parse::<usize>()
        {
            page = parsed.max(1);
            name_parts.pop();
        }

        let name = name_parts.join(" ");
        if name.is_empty() {
            self.list(registry, sender, page);
            return Ok(());
        }

        let Some(command) = registry.get(&name) else {
            return Err(format!("No command matching \"{name}\" found."));
        };

        self.describe(command, sender);
        Ok(())
    }
}
