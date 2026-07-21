use crate::command::command::{Command, CommandResult};
use crate::command::registry::CommandRegistry;
use crate::command::sender::CommandSender;

pub struct PingCommand;

impl Command for PingCommand {
    fn name(&self) -> &str {
        "ping"
    }

    fn description(&self) -> &str {
        "Replies with pong"
    }

    fn execute(&self, _registry: &CommandRegistry, sender: &mut CommandSender, _args: &[&str]) -> CommandResult {
        let name = sender.name().to_string();
        sender.reply(format!("Pong, {name}!"));
        Ok(())
    }
}
