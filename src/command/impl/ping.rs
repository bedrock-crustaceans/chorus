use crate::command::command_definition::CommandDefinition;

pub const PING_COMMAND: CommandDefinition = CommandDefinition::new("ping", "Replies with pong", |context, _| {
    let name = context.sender_name().to_string();
    context.reply(format!("Pong, {name}!"));
    Ok(())
});
