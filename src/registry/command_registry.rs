use crate::command::command_definition::CommandDefinition;
use crate::command::context::CommandContext;
use crate::command::r#impl::DEFINITIONS;
use atomicow::CowArc;
use bedrock::protocol::v898::packets::{AvailableCommandsPacket, CommandsEntry, EnumDataEntry, OverloadsEntry};
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

    pub fn dispatch(context: &mut CommandContext, line: &str) {
        let line = line.trim().trim_start_matches('/');

        let mut parts = line.split_whitespace();
        let Some(name) = parts.next() else {
            return;
        };
        let args: Vec<&str> = parts.collect();

        let Some(execute) = context.registry().get(name).map(|command| command.execute) else {
            context.reply(format!("§cUnknown command: {name}"));
            return;
        };

        if let Err(err) = execute(context, &args) {
            context.reply(format!("§c{err}"));
        }
    }

    pub fn to_packet(&self) -> AvailableCommandsPacket {
        let mut enum_values = Vec::new();
        let mut enum_data = Vec::new();
        let commands = self
            .commands
            .iter()
            .map(|command| {
                let alias_enum = if command.aliases.is_empty() {
                    -1
                } else {
                    let values = std::iter::once(&command.name)
                        .chain(command.aliases.iter())
                        .map(|name| {
                            enum_values.push(name.to_string());
                            (enum_values.len() - 1) as u32
                        })
                        .collect();
                    enum_data.push(EnumDataEntry {
                        name: format!("{}Aliases", command.name),
                        values,
                    });
                    (enum_data.len() - 1) as i32
                };
                let mut overloads: Vec<_> = command.overloads.iter().map(|overload| overload.to_entry()).collect();
                if overloads.is_empty() {
                    overloads.push(OverloadsEntry {
                        is_chaining: false,
                        parameter_data: vec![],
                    });
                }
                CommandsEntry {
                    name: command.name.to_string(),
                    description: command.description.to_string(),
                    flags: 0,
                    permission_level: command.permission.clone(),
                    alias_enum,
                    chained_sub_command_indices: vec![],
                    overloads,
                }
            })
            .collect();

        AvailableCommandsPacket {
            enum_values,
            sub_command_values: vec![],
            post_fixes: vec![],
            enum_data,
            chained_sub_command_data: vec![],
            commands,
            soft_enums: vec![],
            constraints: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CommandRegistry;
    use crate::command::r#impl::DEFINITIONS;
    use bedrock::protocol::ProtoCodec;
    use bedrock::protocol::v898::packets::AvailableCommandsPacket;

    #[test]
    fn available_commands_round_trip() {
        let mut registry = CommandRegistry::new();
        registry.register_all(DEFINITIONS.iter().copied());
        let packet = registry.to_packet();

        let mut bytes = Vec::new();
        packet.serialize(&mut bytes).expect("serialize");
        let decoded = AvailableCommandsPacket::deserialize(&mut bytes.as_slice()).expect("deserialize");

        assert_eq!(decoded.commands.len(), registry.commands().count());
        for command in &decoded.commands {
            assert!(!command.overloads.is_empty(), "{} has no overloads", command.name);
            if command.alias_enum >= 0 {
                let aliases = &decoded.enum_data[command.alias_enum as usize];
                assert!(aliases.values.iter().all(|&index| (index as usize) < decoded.enum_values.len()));
                assert_eq!(decoded.enum_values[aliases.values[0] as usize], command.name);
            }
        }
    }
}
