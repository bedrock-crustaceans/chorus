use crate::command::command_definition::CommandDefinition;
use crate::command::context::CommandContext;
use crate::command::r#impl::DEFINITIONS;
use crate::command::sender::CommandSender;
use atomicow::CowArc;
use bedrock::protocol::v898::packets::{AvailableCommandsPacket, CommandsEntry};
use bevy_ecs::prelude::{Commands, Resource};
use std::collections::HashMap;
use tracing::{debug, info};

#[derive(Resource, Default)]
pub struct CommandRegistry {
    commands: Vec<CowArc<'static, CommandDefinition>>,
    index: HashMap<CowArc<'static, str>, usize>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(mut commands: Commands) {
        let mut registry = Self::new();

        registry.register_all(DEFINITIONS.iter().copied());

        commands.insert_resource(registry);
    }

    pub fn register<C>(&mut self, command: C)
    where
        C: Into<CowArc<'static, CommandDefinition>>,
    {
        let command = command.into();

        let position = self.commands.len();

        self.index.insert(command.name.clone(), position);
        for alias in command.aliases.iter() {
            self.index.insert(alias.clone(), position);
        }

        debug!("registered command {:?}", command.name);

        self.commands.push(command);
    }

    pub fn register_all<I, C>(&mut self, commands: I)
    where
        I: IntoIterator<Item = C>,
        C: Into<CowArc<'static, CommandDefinition>>,
    {
        let before = self.commands.len();

        for command in commands {
            self.register(command);
        }

        info!("registered {} commands", self.commands.len() - before);
    }

    pub fn get(&self, name: &str) -> Option<&CommandDefinition> {
        self.index.get(name).and_then(|&position| self.commands.get(position).map(|c| c.as_ref()))
    }

    pub fn commands(&self) -> impl Iterator<Item = &CommandDefinition> {
        self.commands.iter().map(|c| c.as_ref())
    }

    pub fn dispatch(&self, context: &CommandContext, line: &str, sender: &mut CommandSender) {
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

        if let Err(err) = (command.execute)(context, sender, &args) {
            sender.reply(format!("§c{err}"));
        }
    }

    pub fn to_packet(&self) -> AvailableCommandsPacket {
        let commands = self
            .commands
            .iter()
            .map(|command| CommandsEntry {
                name: command.name.to_string(),
                description: command.description.to_string(),
                flags: 0,
                permission_level: command.permission.clone(),
                alias_enum: -1,
                chained_sub_command_indices: vec![],
                overloads: command.overloads.iter().map(|overload| overload.to_entry()).collect(),
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
