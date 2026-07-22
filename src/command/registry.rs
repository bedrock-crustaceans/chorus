use crate::command::command::Command;
use crate::command::r#impl::dev::DevCommand;
use crate::command::r#impl::help::HelpCommand;
use crate::command::r#impl::ping::PingCommand;
use crate::command::sender::CommandSender;
use bedrock::protocol::v898::packets::{AvailableCommandsPacket, CommandsEntry};
use bevy_ecs::prelude::{Commands, Resource};
use std::collections::HashMap;
use tracing::debug;

#[derive(Resource, Default)]
pub struct CommandRegistry {
    commands: Vec<Box<dyn Command>>,
    index: HashMap<String, usize>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(mut commands: Commands) {
        let mut registry = Self::new();

        registry.register(HelpCommand);
        registry.register(PingCommand);
        registry.register(DevCommand);

        commands.insert_resource(registry);
    }

    pub fn register<C>(&mut self, command: C)
    where
        C: Command + 'static,
    {
        let position = self.commands.len();

        self.index.insert(command.name().to_string(), position);
        for alias in command.aliases() {
            self.index.insert(alias.to_string(), position);
        }

        debug!("registered command {:?}", command.name());

        self.commands.push(Box::new(command));
    }

    pub fn get(&self, name: &str) -> Option<&dyn Command> {
        self.index.get(name).map(|&position| self.commands[position].as_ref())
    }

    pub fn commands(&self) -> impl Iterator<Item = &dyn Command> {
        self.commands.iter().map(|command| command.as_ref())
    }

    pub fn dispatch(&self, line: &str, sender: &mut CommandSender) {
        let line = line.trim().trim_start_matches('/');

        let mut parts = line.split_whitespace();
        let Some(name) = parts.next() else {
            return;
        };
        let args: Vec<&str> = parts.collect();

        let Some(command) = self.get(name) else {
            sender.reply(format!("§cUnknown command: {name}"));
            return;
        };

        if let Err(err) = command.execute(self, sender, &args) {
            sender.reply(format!("§c{err}"));
        }
    }

    pub fn to_packet(&self) -> AvailableCommandsPacket {
        let commands = self
            .commands
            .iter()
            .map(|command| CommandsEntry {
                name: command.name().to_string(),
                description: command.description().to_string(),
                flags: 0,
                permission_level: command.permission(),
                alias_enum: -1,
                chained_sub_command_indices: vec![],
                overloads: command.overloads().iter().map(|overload| overload.to_entry()).collect(),
            })
            .collect();

        AvailableCommandsPacket {
            enum_values: vec![],
            sub_command_values: vec![],
            post_fixes: vec![],
            enum_data: vec![],
            chained_sub_command_data: vec![],
            commands,
            soft_enums: vec![],
            constraints: vec![],
        }
    }
}
