use crate::command::command_definition::CommandDefinition;
use crate::command::context::CommandContext;
use crate::command::r#impl::aimassist::AIMASSIST_COMMAND;
use crate::command::r#impl::camera::CAMERA_COMMAND;
use crate::command::r#impl::camerashake::CAMERASHAKE_COMMAND;
use crate::command::r#impl::debug::DEBUG_COMMAND;
use crate::command::r#impl::gamemode::GAMEMODE_COMMAND;
use crate::command::r#impl::help::HELP_COMMAND;
use crate::command::r#impl::list::LIST_COMMAND;
use crate::command::r#impl::ping::PING_COMMAND;
use crate::command::r#impl::status::STATUS_COMMAND;
use crate::command::sender::CommandSender;
use atomicow::CowArc;
use bedrock::protocol::v898::packets::{AvailableCommandsPacket, CommandsEntry};
use bevy_ecs::prelude::{Commands, Resource};
use std::collections::HashMap;
use tracing::debug;

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

        registry.register(&HELP_COMMAND);
        registry.register(&PING_COMMAND);
        registry.register(&DEBUG_COMMAND);
        registry.register(&STATUS_COMMAND);
        registry.register(&LIST_COMMAND);

        registry.register(&GAMEMODE_COMMAND);

        registry.register(&AIMASSIST_COMMAND);
        registry.register(&CAMERA_COMMAND);
        registry.register(&CAMERASHAKE_COMMAND);

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
