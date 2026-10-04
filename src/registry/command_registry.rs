use crate::command::args;
use crate::command::command_definition::CommandDefinition;
use crate::command::context::CommandContext;
use crate::command::r#impl::DEFINITIONS;
use crate::command::parameter::{COMMAND_NAME_ENUM, ChainedSubcommand, CommandEnum, CommandParameter, ParameterKind};
use crate::network::BedrockProtocol;
use crate::network::session::Session;
use crate::network::session::state::SessionState;
use atomicow::CowArc;
use bedrock::protocol::ProtoVersionPackets;
use bedrock::protocol::v662::enums::SoftEnumUpdateType;
use bedrock::protocol::v898::packets::{AvailableCommandsPacket, ChainedSubCommandDataEntry, CommandsEntry, ConstraintsEntry, EnumDataEntry, OverloadsEntry, SoftEnumsEntry, SubCommandValues};
use bevy_ecs::prelude::{Commands, Query, ResMut, Resource};
use indexmap::IndexMap;
use std::collections::HashMap;
use tracing::{debug, info};

type UpdateSoftEnumPacket = <BedrockProtocol as ProtoVersionPackets>::UpdateSoftEnumPacket;

const ARG_FLAG_VALID: u32 = 0x100000;
const ARG_FLAG_ENUM: u32 = 0x200000;
const ARG_FLAG_POSTFIX: u32 = 0x1000000;
const ARG_FLAG_SOFT_ENUM: u32 = 0x4000000;

#[derive(Resource, Default)]
pub struct CommandRegistry {
    commands: Vec<CowArc<'static, CommandDefinition>>,
    index: HashMap<String, usize>,
    soft_enums: IndexMap<String, Vec<String>>,
    soft_enum_updates: Vec<(String, Vec<String>, SoftEnumUpdateType)>,
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

        self.index.insert(command.name.to_ascii_lowercase(), position);
        for alias in command.aliases.iter() {
            self.index.insert(alias.to_ascii_lowercase(), position);
        }
        for parameter in command.overloads.iter().flat_map(|overload| overload.parameters.iter()) {
            if let ParameterKind::SoftEnum(name) = &parameter.kind {
                self.soft_enums.entry(name.to_string()).or_default();
            }
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
        self.index.get(&name.to_ascii_lowercase()).and_then(|&position| self.commands.get(position).map(|c| c.as_ref()))
    }

    pub fn commands(&self) -> impl Iterator<Item = &CommandDefinition> {
        self.commands.iter().map(|c| c.as_ref())
    }

    pub fn soft_enum(&self, name: &str) -> Option<&[String]> {
        self.soft_enums.get(name).map(Vec::as_slice)
    }

    pub fn set_soft_enum(&mut self, name: impl Into<String>, values: impl IntoIterator<Item = impl Into<String>>) {
        let (name, values): (String, Vec<String>) = (name.into(), values.into_iter().map(Into::into).collect());
        self.soft_enums.insert(name.clone(), values.clone());
        self.soft_enum_updates.push((name, values, SoftEnumUpdateType::Replace));
    }

    pub fn add_soft_enum_values(&mut self, name: impl Into<String>, values: impl IntoIterator<Item = impl Into<String>>) {
        let name = name.into();
        let existing = self.soft_enums.entry(name.clone()).or_default();
        let added: Vec<String> = values.into_iter().map(Into::into).filter(|value| !existing.contains(value)).collect();
        if added.is_empty() {
            return;
        }
        existing.extend(added.iter().cloned());
        self.soft_enum_updates.push((name, added, SoftEnumUpdateType::Add));
    }

    pub fn remove_soft_enum_values(&mut self, name: impl Into<String>, values: impl IntoIterator<Item = impl Into<String>>) {
        let name = name.into();
        let Some(existing) = self.soft_enums.get_mut(&name) else { return };
        let removed: Vec<String> = values.into_iter().map(Into::into).filter(|value| existing.contains(value)).collect();
        if removed.is_empty() {
            return;
        }
        existing.retain(|value| !removed.contains(value));
        self.soft_enum_updates.push((name, removed, SoftEnumUpdateType::Remove));
    }

    pub fn broadcast_soft_enum_updates(mut registry: ResMut<CommandRegistry>, mut sessions: Query<&mut Session>) {
        if registry.soft_enum_updates.is_empty() {
            return;
        }
        for (name, values, update_type) in std::mem::take(&mut registry.soft_enum_updates) {
            for mut session in sessions.iter_mut().filter(|session| session.get_state() == SessionState::Play) {
                session.send(BedrockProtocol::UpdateSoftEnumPacket(
                    UpdateSoftEnumPacket {
                        enum_name: name.clone(),
                        values: values.clone(),
                        update_type: update_type.clone(),
                    }
                    .into(),
                ));
            }
        }
    }

    pub fn dispatch(context: &mut CommandContext, line: &str) {
        let line = line.trim().trim_start_matches('/');
        let (name, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        if name.is_empty() {
            return;
        }

        let registry = context.registry();
        let mut lookups: HashMap<String, Vec<String>> = registry.soft_enums.clone().into_iter().collect();
        lookups.insert(COMMAND_NAME_ENUM.to_owned(), registry.index.keys().cloned().collect());
        let command = registry.get(name).cloned();
        let Some(command) = command else {
            context.reply(format!("§cUnknown command: {name}. Type /help for a list of commands."));
            return;
        };
        if context.permission_level() < command.permission {
            context.reply(format!("§cYou don't have permission to use /{}.", command.name));
            return;
        }

        let parsed = match args::parse(&command.name, &command.overloads, rest, &lookups, context.permission_level()) {
            Ok(parsed) => parsed,
            Err(error) => {
                context.reply(format!("§cSyntax error: {}", error.message));
                for usage in error.usages {
                    context.reply(format!("§7Usage: {usage}"));
                }
                return;
            }
        };

        if let Err(err) = (command.execute)(context, &parsed) {
            context.reply(format!("§c{err}"));
        }
    }

    pub fn to_packet(&self) -> AvailableCommandsPacket {
        let mut packet = PacketBuilder::new(&self.soft_enums, self.commands().map(|command| command.name.to_string()).collect());
        let commands = self.commands.iter().map(|command| packet.command(command)).collect();
        packet.finish(commands)
    }
}

struct PacketBuilder {
    enum_values: Vec<String>,
    value_index: HashMap<String, u32>,
    enum_data: Vec<EnumDataEntry>,
    enum_index: HashMap<String, u32>,
    postfixes: Vec<String>,
    soft_enums: Vec<SoftEnumsEntry>,
    constraints: Vec<ConstraintsEntry>,
    sub_command_values: Vec<String>,
    chained: Vec<ChainedSubCommandDataEntry>,
    command_names: Vec<String>,
}

impl PacketBuilder {
    fn new(soft_enums: &IndexMap<String, Vec<String>>, mut command_names: Vec<String>) -> Self {
        command_names.sort_unstable();
        Self {
            command_names,
            enum_values: Vec::new(),
            value_index: HashMap::new(),
            enum_data: Vec::new(),
            enum_index: HashMap::new(),
            postfixes: Vec::new(),
            constraints: Vec::new(),
            sub_command_values: Vec::new(),
            chained: Vec::new(),
            soft_enums: soft_enums
                .iter()
                .map(|(name, values)| SoftEnumsEntry {
                    enum_name: name.clone(),
                    enum_options: values.clone(),
                })
                .collect(),
        }
    }

    fn enumeration<'a>(&mut self, name: &str, values: impl IntoIterator<Item = &'a str>) -> u32 {
        if let Some(&index) = self.enum_index.get(name) {
            return index;
        }
        let values = values
            .into_iter()
            .map(|value| {
                *self.value_index.entry(value.to_owned()).or_insert_with(|| {
                    self.enum_values.push(value.to_owned());
                    (self.enum_values.len() - 1) as u32
                })
            })
            .collect();
        self.enum_data.push(EnumDataEntry { name: name.to_owned(), values });
        let index = (self.enum_data.len() - 1) as u32;
        self.enum_index.insert(name.to_owned(), index);
        index
    }

    fn command_enum(&mut self, values: &CommandEnum) -> u32 {
        let known = self.enum_index.contains_key(values.name.as_ref());
        let index = self.enumeration(&values.name, values.values.iter().map(|value| value.as_ref()));
        if !known {
            for value in values.values.iter() {
                let constraints = values.constraints_of(value);
                if !constraints.is_empty() {
                    self.constraints.push(ConstraintsEntry {
                        enum_value_symbol: self.value_index[value.as_ref()],
                        enum_symbol: index,
                        constraint_indices: constraints.ids(),
                    });
                }
            }
        }
        index
    }

    fn sub_command_value(&mut self, value: &str) -> u32 {
        match self.sub_command_values.iter().position(|existing| existing == value) {
            Some(index) => index as u32,
            None => {
                self.sub_command_values.push(value.to_owned());
                (self.sub_command_values.len() - 1) as u32
            }
        }
    }

    fn chained_subcommand(&mut self, chained: &ChainedSubcommand) -> i32 {
        if let Some(index) = self.chained.iter().position(|entry| entry.sub_command_name == chained.name.as_ref()) {
            return index as i32;
        }
        let sub_command_values = chained
            .values
            .iter()
            .map(|(first, second)| SubCommandValues {
                index: self.sub_command_value(first),
                value: self.sub_command_value(second),
            })
            .collect();
        self.chained.push(ChainedSubCommandDataEntry {
            sub_command_name: chained.name.to_string(),
            sub_command_values,
        });
        (self.chained.len() - 1) as i32
    }

    fn symbol(&mut self, parameter: &CommandParameter) -> u32 {
        match &parameter.kind {
            ParameterKind::Argument(kind) => ARG_FLAG_VALID | kind.id(),
            ParameterKind::Enum(values) => ARG_FLAG_VALID | ARG_FLAG_ENUM | self.command_enum(values),
            ParameterKind::Literal(word) => ARG_FLAG_VALID | ARG_FLAG_ENUM | self.enumeration(word, [word.as_ref()]),
            ParameterKind::CommandName => {
                let names = std::mem::take(&mut self.command_names);
                let index = self.enumeration(COMMAND_NAME_ENUM, names.iter().map(String::as_str));
                self.command_names = names;
                ARG_FLAG_VALID | ARG_FLAG_ENUM | index
            }
            ParameterKind::SoftEnum(name) => {
                let index = match self.soft_enums.iter().position(|entry| entry.enum_name == name.as_ref()) {
                    Some(index) => index,
                    None => {
                        self.soft_enums.push(SoftEnumsEntry {
                            enum_name: name.to_string(),
                            enum_options: Vec::new(),
                        });
                        self.soft_enums.len() - 1
                    }
                };
                ARG_FLAG_VALID | ARG_FLAG_SOFT_ENUM | index as u32
            }
            ParameterKind::Postfix(postfix) => {
                let index = match self.postfixes.iter().position(|existing| existing == postfix.as_ref()) {
                    Some(index) => index,
                    None => {
                        self.postfixes.push(postfix.to_string());
                        self.postfixes.len() - 1
                    }
                };
                ARG_FLAG_POSTFIX | index as u32
            }
        }
    }

    fn command(&mut self, command: &CommandDefinition) -> CommandsEntry {
        let alias_enum = if command.aliases.is_empty() {
            -1
        } else {
            let names: Vec<&str> = std::iter::once(command.name.as_ref()).chain(command.aliases.iter().map(|alias| alias.as_ref())).collect();
            self.enumeration(&format!("{}Aliases", command.name), names) as i32
        };
        let mut overloads: Vec<OverloadsEntry> = command.overloads.iter().map(|overload| overload.to_entry(|parameter| self.symbol(parameter))).collect();
        if overloads.is_empty() {
            overloads.push(OverloadsEntry {
                is_chaining: false,
                parameter_data: vec![],
            });
        }
        CommandsEntry {
            name: command.name.to_string(),
            description: command.description.to_string(),
            flags: command.flags.0,
            permission_level: command.permission.into(),
            alias_enum,
            chained_sub_command_indices: command.chained_subcommands.iter().map(|chained| self.chained_subcommand(chained)).collect(),
            overloads,
        }
    }

    fn finish(self, commands: Vec<CommandsEntry>) -> AvailableCommandsPacket {
        AvailableCommandsPacket {
            enum_values: self.enum_values,
            sub_command_values: self.sub_command_values,
            post_fixes: self.postfixes,
            enum_data: self.enum_data,
            chained_sub_command_data: self.chained,
            commands,
            soft_enums: self.soft_enums,
            constraints: self.constraints,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CommandRegistry;
    use crate::command::r#impl::DEFINITIONS;
    use bedrock::protocol::ProtoCodec;
    use bedrock::protocol::v898::packets::AvailableCommandsPacket;

    const ARG_FLAG_ENUM: u32 = super::ARG_FLAG_ENUM;
    const ARG_FLAG_SOFT_ENUM: u32 = super::ARG_FLAG_SOFT_ENUM;
    const ARG_FLAG_POSTFIX: u32 = super::ARG_FLAG_POSTFIX;

    #[test]
    fn chained_subcommands_and_constraints_are_encoded() {
        use crate::command::command_definition::CommandDefinition;
        use crate::command::parameter::{ArgumentType, ChainedSubcommand, CommandEnum, CommandParameter, EnumConstraints};
        use crate::{chained, constraints, overloads, values};

        const MODES: CommandEnum =
            CommandEnum::new("TestMode", values!["normal", "secret"]).constrained(constraints![("secret", EnumConstraints::OPERATOR_PERMISSIONS.with(EnumConstraints::CHEATS_ENABLED))]);
        const CHAINED: &[ChainedSubcommand] = &[ChainedSubcommand::new("TestChainedOption_0", chained![("as", "origin"), ("at", "origin")])];
        static TEST: CommandDefinition = CommandDefinition::new("test", "test", |_, _| Ok(()))
            .overloads(overloads![[CommandParameter::enumeration("mode", MODES)], [CommandParameter::new("origin", ArgumentType::Target)]])
            .chained_subcommands(CHAINED);

        let mut registry = CommandRegistry::new();
        registry.register(&TEST);
        let mut bytes = Vec::new();
        registry.to_packet().serialize(&mut bytes).expect("serialize");
        let decoded = AvailableCommandsPacket::deserialize(&mut bytes.as_slice()).expect("deserialize");

        assert_eq!(decoded.sub_command_values, ["as", "origin", "at"]);
        assert_eq!(decoded.chained_sub_command_data.len(), 1);
        let chained = &decoded.chained_sub_command_data[0];
        assert_eq!(chained.sub_command_name, "TestChainedOption_0");
        let pairs: Vec<(u32, u32)> = chained.sub_command_values.iter().map(|value| (value.index, value.value)).collect();
        assert_eq!(pairs, [(0, 1), (2, 1)]);
        assert_eq!(decoded.commands[0].chained_sub_command_indices, [0]);

        assert_eq!(decoded.constraints.len(), 1);
        let constraint = &decoded.constraints[0];
        assert_eq!(decoded.enum_values[constraint.enum_value_symbol as usize], "secret");
        assert_eq!(decoded.enum_data[constraint.enum_symbol as usize].name, "TestMode");
        assert_eq!(constraint.constraint_indices, [0, 1]);
    }

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
            for parameter in command.overloads.iter().flat_map(|overload| &overload.parameter_data) {
                let index = (parameter.parse_symbol & 0xFFFF) as usize;
                if parameter.parse_symbol & ARG_FLAG_SOFT_ENUM != 0 {
                    assert!(index < decoded.soft_enums.len(), "{}: soft enum {index}", command.name);
                } else if parameter.parse_symbol & ARG_FLAG_ENUM != 0 {
                    let data = &decoded.enum_data[index];
                    assert!(data.values.iter().all(|&value| (value as usize) < decoded.enum_values.len()), "{}: enum {}", command.name, data.name);
                } else if parameter.parse_symbol & ARG_FLAG_POSTFIX != 0 {
                    assert!(index < decoded.post_fixes.len(), "{}: postfix {index}", command.name);
                }
            }
        }
    }
}
