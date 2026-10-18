//! Letting plugins register commands.
//!
//! Chorus's `CommandExecutor` is a bare `fn` pointer with no identity, so every plugin command
//! shares one executor that looks up its owner by the name the sender typed. Registrations are
//! staged during `on_load` and only committed once the plugin returns cleanly, which is what keeps
//! a plugin that fails halfway from leaving commands behind.

use std::collections::HashMap;
use std::sync::Arc;

use atomicow::CowArc;
use bevy_ecs::prelude::{Mut, Resource, World};
use chorus_core::permission::PermissionLevel;
use chorus_entity::components::transform::Transform;
use chorus_plugin::chorus::plugin::command::{Argument, CommandSender, ConsumedArgs, Definition, ParameterType};
use chorus_plugin::chorus::plugin::common::PermissionLevel as WitPermission;
use tracing::debug;

use crate::command::args::CommandArgs;
use crate::command::command_definition::{CommandDefinition, CommandFlags};
use crate::command::command_result::CommandResult;
use crate::command::context::CommandContext;
use crate::command::parameter::{ArgumentType, CommandOverload, CommandParameter, ParameterKind, ParameterOptions};
use crate::player::gamemode::Gamemode;
use crate::player::identity::PlayerIdentity;
use crate::plugin::PluginHostResource;
use crate::plugin::host_impl::snapshot;
use crate::registry::command_registry::CommandRegistry;

/// What a plugin command needs at dispatch time: who owns it, which id to hand back, and the
/// parameter names and kinds per overload so the typed arguments can be rebuilt.
struct PluginCommand {
    plugin: String,
    id: u32,
    overloads: Vec<Vec<(String, ParameterType)>>,
}

/// Plugin commands, keyed by every name they answer to, lowercased.
#[derive(Resource, Default)]
pub struct PluginCommands {
    commands: HashMap<String, PluginCommand>,
    staged: Vec<(String, Vec<String>, CommandDefinition, PluginCommand)>,
    next_id: HashMap<String, u32>,
    /// Registration closes once `on_load` is done for everyone. Without this a plugin could keep
    /// staging commands from a later call, where nothing would ever commit them: they would hold
    /// their names hostage and the list would grow for the life of the process.
    closed: bool,
}

impl PluginCommands {
    fn taken(&self, name: &str) -> bool {
        let name = name.to_ascii_lowercase();
        self.commands.contains_key(&name) || self.staged.iter().any(|(_, names, _, _)| names.contains(&name))
    }
}

/// Validates a plugin's command and stages it.
///
/// Nothing reaches `CommandRegistry` here: [`commit_plugin_commands`] does that once the plugin's
/// `on_load` has returned. Errors come back to the plugin as the `register` import's error, so a
/// plugin author sees why their command was refused.
pub fn register_plugin_command(world: &mut World, plugin: &str, definition: Definition) -> Result<u32, String> {
    if world.get_resource::<PluginCommands>().is_some_and(|pending| pending.closed) {
        return Err(String::from("commands can only be registered from on_load"));
    }

    let name = definition.name.trim().to_owned();
    if name.is_empty() || name.contains(char::is_whitespace) {
        return Err(format!("{name:?} is not a usable command name"));
    }

    let mut names = vec![name.to_ascii_lowercase()];
    names.extend(definition.aliases.iter().map(|alias| alias.trim().to_ascii_lowercase()));

    // this runs inside a wasm host call, so it returns errors rather than unwrapping its way to a
    // panic that would unwind through wasmtime
    let Some(registry) = world.get_resource::<CommandRegistry>() else {
        return Err(String::from("this server has no command registry"));
    };
    if let Some(taken) = names.iter().find(|name| registry.get(name).is_some()) {
        return Err(format!("/{taken} is already a command on this server"));
    }

    let Some(pending) = world.get_resource::<PluginCommands>() else {
        return Err(String::from("this server has no plugin command table"));
    };
    if let Some(taken) = names.iter().find(|name| pending.taken(name)) {
        return Err(format!("/{taken} is already registered by a plugin"));
    }

    let overloads: Vec<Vec<(String, ParameterType)>> = definition
        .overloads
        .iter()
        .map(|overload| overload.parameters.iter().map(|parameter| (parameter.name.clone(), parameter.kind)).collect())
        .collect();

    let built = CommandDefinition {
        name: CowArc::Owned(Arc::from(name.as_str())),
        description: CowArc::Owned(Arc::from(definition.description.as_str())),
        aliases: CowArc::Owned(definition.aliases.iter().map(|alias| CowArc::Owned(Arc::from(alias.as_str()))).collect()),
        permission: permission_from_wit(definition.permission),
        flags: CommandFlags::NOT_CHEAT,
        overloads: CowArc::Owned(definition.overloads.iter().map(build_overload).collect()),
        chained_subcommands: CowArc::Static(&[]),
        execute: plugin_command_executor,
    };

    let mut pending = world.resource_mut::<PluginCommands>();
    let id = pending.next_id.entry(plugin.to_owned()).or_default();
    let assigned = *id;
    *id += 1;

    pending.staged.push((
        plugin.to_owned(),
        names,
        built,
        PluginCommand {
            plugin: plugin.to_owned(),
            id: assigned,
            overloads,
        },
    ));

    Ok(assigned)
}

/// Moves staged commands into the real registry for every plugin in `keep`, and throws away the
/// rest. A plugin that failed `on_load` is simply not in `keep`.
pub fn commit_plugin_commands(world: &mut World, keep: &[String]) {
    if !world.contains_resource::<PluginCommands>() || !world.contains_resource::<CommandRegistry>() {
        return;
    }
    let staged = {
        let mut pending = world.resource_mut::<PluginCommands>();
        pending.closed = true;
        std::mem::take(&mut pending.staged)
    };

    for (plugin, names, definition, command) in staged {
        if !keep.contains(&plugin) {
            debug!(plugin = %plugin, "dropped the staged command /{} because the plugin did not load", definition.name);
            continue;
        }

        world.resource_mut::<CommandRegistry>().register(CowArc::Owned(Arc::new(definition)));

        let mut pending = world.resource_mut::<PluginCommands>();
        for name in names {
            pending.commands.insert(
                name,
                PluginCommand {
                    plugin: command.plugin.clone(),
                    id: command.id,
                    overloads: command.overloads.clone(),
                },
            );
        }
    }
}

/// The one executor every plugin command points at.
pub fn plugin_command_executor(context: &mut CommandContext, args: &CommandArgs) -> CommandResult {
    let name = context.command_name().to_ascii_lowercase();

    let (plugin, id, parameters) = {
        let Some(pending) = context.world().get_resource::<PluginCommands>() else {
            return Err(String::from("That command's plugin is no longer loaded."));
        };
        let Some(command) = pending.commands.get(&name) else {
            return Err(String::from("That command's plugin is no longer loaded."));
        };
        (command.plugin.clone(), command.id, command.overloads.get(args.overload()).cloned().unwrap_or_default())
    };

    let sender = build_sender(context);
    let consumed = ConsumedArgs {
        overload: args.overload() as u32,
        values: parameters
            .iter()
            .filter_map(|(name, kind)| match kind {
                ParameterType::Text => args.string(name).map(|value| Argument::Text(value.to_owned())),
                ParameterType::Integer => args.int(name).map(|value| Argument::Integer(i64::from(value))),
                ParameterType::Decimal => args.float(name).map(|value| Argument::Decimal(f64::from(value))),
            })
            .collect(),
    };

    let world = context.world_mut();
    let outcome = world.resource_scope(|world, host: Mut<PluginHostResource>| {
        let erased = world as *mut World as *mut ();
        host.host().call(&plugin, erased, |store, bindings| bindings.call_handle_command(store, id, &sender, &consumed))
    });

    match outcome {
        Some(Ok(_)) => Ok(()),
        Some(Err(err)) => Err(err),
        None => Err(String::from("That command's plugin is unavailable.")),
    }
}

fn build_sender(context: &CommandContext) -> CommandSender {
    let player = match (context.get::<PlayerIdentity>(), context.get::<Transform>(), context.get::<Gamemode>()) {
        (Some(identity), Some(transform), Some(gamemode)) => Some(snapshot(identity, transform, gamemode)),
        _ => None,
    };

    CommandSender {
        name: context.sender_name().to_owned(),
        is_console: context.is_console(),
        permission: permission_to_wit(context.permission_level()),
        player,
    }
}

fn build_overload(overload: &chorus_plugin::chorus::plugin::command::Overload) -> CommandOverload {
    CommandOverload::owned(
        overload
            .parameters
            .iter()
            .map(|parameter| CommandParameter {
                name: CowArc::Owned(Arc::from(parameter.name.as_str())),
                kind: ParameterKind::Argument(match parameter.kind {
                    ParameterType::Text => ArgumentType::String,
                    ParameterType::Integer => ArgumentType::Int,
                    ParameterType::Decimal => ArgumentType::Float,
                }),
                optional: parameter.optional,
                options: ParameterOptions::NONE,
            })
            .collect(),
    )
}

/// A plugin cannot ask for `Internal`: the WIT enum does not have it.
fn permission_from_wit(level: WitPermission) -> PermissionLevel {
    match level {
        WitPermission::Member => PermissionLevel::Member,
        WitPermission::Operator => PermissionLevel::Operator,
        WitPermission::Admin => PermissionLevel::Admin,
        WitPermission::Host => PermissionLevel::Host,
        WitPermission::Owner => PermissionLevel::Owner,
    }
}

/// `Internal` has no WIT case, and the only sender that could carry it is the server itself, so it
/// reads as `owner` to a plugin.
fn permission_to_wit(level: PermissionLevel) -> WitPermission {
    match level {
        PermissionLevel::Member => WitPermission::Member,
        PermissionLevel::Operator => WitPermission::Operator,
        PermissionLevel::Admin => WitPermission::Admin,
        PermissionLevel::Host => WitPermission::Host,
        PermissionLevel::Owner | PermissionLevel::Internal => WitPermission::Owner,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chorus_plugin::chorus::plugin::command::{Overload as WitOverload, Parameter as WitParameter};

    fn world_with_registry() -> World {
        let mut world = World::new();
        let mut registry = CommandRegistry::new();
        registry.register_all(crate::command::r#impl::DEFINITIONS.iter().copied());
        world.insert_resource(registry);
        world.init_resource::<PluginCommands>();
        world
    }

    fn definition(name: &str, aliases: &[&str]) -> Definition {
        Definition {
            name: name.to_owned(),
            description: String::from("a plugin command"),
            aliases: aliases.iter().map(|alias| (*alias).to_owned()).collect(),
            permission: WitPermission::Member,
            overloads: vec![WitOverload {
                parameters: vec![WitParameter {
                    name: String::from("who"),
                    kind: ParameterType::Text,
                    optional: true,
                }],
            }],
        }
    }

    #[test]
    fn refuses_a_name_a_builtin_already_owns() {
        let mut world = world_with_registry();

        assert!(register_plugin_command(&mut world, "hello", definition("help", &[])).is_err());
        assert!(world.resource::<CommandRegistry>().get("help").is_some());
    }

    #[test]
    fn refuses_an_alias_a_builtin_already_owns() {
        let mut world = world_with_registry();

        assert!(register_plugin_command(&mut world, "hello", definition("greet", &["stop"])).is_err());
    }

    #[test]
    fn refuses_a_name_another_plugin_staged() {
        let mut world = world_with_registry();
        register_plugin_command(&mut world, "first", definition("greet", &[])).expect("staged");

        assert!(register_plugin_command(&mut world, "second", definition("greet", &[])).is_err());
    }

    #[test]
    fn refuses_a_blank_name() {
        let mut world = world_with_registry();

        assert!(register_plugin_command(&mut world, "hello", definition("  ", &[])).is_err());
    }

    #[test]
    fn ids_count_up_per_plugin() {
        let mut world = world_with_registry();

        assert_eq!(register_plugin_command(&mut world, "hello", definition("one", &[])), Ok(0));
        assert_eq!(register_plugin_command(&mut world, "hello", definition("two", &[])), Ok(1));
        assert_eq!(register_plugin_command(&mut world, "other", definition("three", &[])), Ok(0));
    }

    #[test]
    fn a_staged_command_is_not_in_the_registry_yet() {
        let mut world = world_with_registry();
        register_plugin_command(&mut world, "hello", definition("greet", &[])).expect("staged");

        assert!(world.resource::<CommandRegistry>().get("greet").is_none());
    }

    #[test]
    fn committing_registers_the_command_and_its_alias() {
        let mut world = world_with_registry();
        register_plugin_command(&mut world, "hello", definition("greet", &["hi"])).expect("staged");

        commit_plugin_commands(&mut world, &[String::from("hello")]);

        assert!(world.resource::<CommandRegistry>().get("greet").is_some());
        assert!(world.resource::<CommandRegistry>().get("hi").is_some());
    }

    #[test]
    fn registration_closes_once_commands_are_committed() {
        let mut world = world_with_registry();
        commit_plugin_commands(&mut world, &[]);

        assert!(register_plugin_command(&mut world, "hello", definition("greet", &[])).is_err());
    }

    #[test]
    fn a_plugin_that_did_not_load_leaves_no_commands_behind() {
        let mut world = world_with_registry();
        register_plugin_command(&mut world, "hello", definition("greet", &[])).expect("staged");

        commit_plugin_commands(&mut world, &[]);

        assert!(world.resource::<CommandRegistry>().get("greet").is_none());
    }
}
