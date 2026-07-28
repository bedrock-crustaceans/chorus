use crate::command::command_definition::CommandDefinition;
use crate::command::command_registry::CommandRegistry;
use crate::command::command_result::CommandResult;
use crate::command::parameter::{CommandOverload, CommandParameter, CommandParameterType};
use crate::command::sender::CommandSender;
use crate::const_command;
use atomicow::CowArc;
use bedrock::form::elems::button::Button;
use bedrock::form::forms::Form;
use bedrock::form::forms::simple::SimpleForm;
use bedrock::protocol::v898::packets::CommandPermissionLevelString;
use tracing::info;

pub const DEBUG_COMMAND: CommandDefinition = const_command! {
    name: "debug",
    description: "Used for debugging",
    aliases: [],
    permission: CommandPermissionLevelString::Any,
    overloads: [
        CommandOverload {
            parameters: CowArc::Static(&[
                CommandParameter {
                    name: CowArc::Static("feature"),
                    kind: CommandParameterType::String,
                    optional: false
                }
            ])
        }
    ],
    execute: execute,
};

fn execute(_registry: &CommandRegistry, sender: &mut CommandSender, args: &[&str]) -> CommandResult {
    match args.first() {
        Some(&"unhandled") => sender.reply(format!("Unhandled Packets: {:#?}", sender.session().unhandled_packets)),
        Some(&"form") => {
            let name = sender.name().to_owned();
            let (session, player) = sender.split();

            let Some(player) = player else {
                return Err("must be sent by player!".to_owned());
            };

            player.send_form(
                session,
                Form::Simple(SimpleForm {
                    body: format!("Hello {}!", name),
                    buttons: vec![
                        Button { text: "Hey!".to_owned(), image: None },
                        Button {
                            text: "Fuck you".to_owned(),
                            image: None,
                        },
                    ],
                    title: "Simple Form".to_owned(),
                }),
                move || {
                    info!("{} responded!", name);
                },
            );
        }
        _ => {}
    }
    Ok(())
}
