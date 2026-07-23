use crate::command::command::{Command, CommandResult};
use crate::command::parameter::{CommandOverload, CommandParameter, CommandParameterType};
use crate::command::registry::CommandRegistry;
use crate::command::sender::CommandSender;
use bedrock::form::elems::button::Button;
use bedrock::form::forms::Form;
use bedrock::form::forms::simple::SimpleForm;
use tracing::info;

pub struct DebugCommand;

impl Command for DebugCommand {
    fn name(&self) -> &str {
        "debug"
    }

    fn description(&self) -> &str {
        "Used for debugging"
    }

    fn overloads(&self) -> Vec<CommandOverload> {
        vec![CommandOverload::new(vec![CommandParameter::new("feature", CommandParameterType::String, false)])]
    }

    fn execute(&self, _registry: &CommandRegistry, sender: &mut CommandSender, _args: &[&str]) -> CommandResult {
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

        Ok(())
    }
}
