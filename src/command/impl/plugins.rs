use crate::command::command_definition::CommandDefinition;
use crate::plugin::PluginHostResource;
use chorus_core::permission::PermissionLevel;
use chorus_plugin::host::PluginStatus;

pub const PLUGINS_COMMAND: CommandDefinition = CommandDefinition::new("plugins", "Lists the loaded plugins", |context, _| {
    let Some(host) = context.world().get_resource::<PluginHostResource>() else {
        context.reply("§cPlugins are disabled on this server.");
        return Ok(());
    };

    let lines: Vec<String> = host
        .host()
        .plugins()
        .map(|plugin| match &plugin.state {
            PluginStatus::Running => format!("§a{} §7{}", plugin.metadata.name, plugin.metadata.version),
            PluginStatus::Quarantined(reason) => format!("§c{} §7{} §8(quarantined: {reason})", plugin.metadata.name, plugin.metadata.version),
        })
        .collect();

    context.reply(format!("Plugins ({}):", lines.len()));

    // the client drops the connection on an empty system message
    if !lines.is_empty() {
        context.reply(lines.join("\n"));
    }
    Ok(())
})
.permission(PermissionLevel::Operator);
