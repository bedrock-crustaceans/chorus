use bedrock::form::elems::button::Button;
use bedrock::form::forms::Form;
use bedrock::form::forms::simple::SimpleForm;
use tracing::info;
use crate::command::command::{Command, CommandResult};
use crate::command::registry::CommandRegistry;
use crate::command::sender::CommandSender;

pub struct DevCommand;

impl Command for DevCommand {
    fn name(&self) -> &str {
        "dev"
    }

    fn description(&self) -> &str {
        "Used for testing of internal features"
    }

    fn execute(&self, _registry: &CommandRegistry, sender: &mut CommandSender, _args: &[&str]) -> CommandResult {
        let name = sender.name().to_owned();
        let (session, player) = sender.split();
        
        let Some(player) = player else { return Err("must be sent by player!".to_owned()); };
        
        player.send_form(session, Form::Simple(SimpleForm { 
            body: format!("Hello {}!", name),
            buttons: vec![
                Button {
                    text: "Hey!".to_owned(),
                    image: None,
                },
                Button {
                    text: "Fuck you".to_owned(),
                    image: None,
                }
            ],
            title: "Simple Form".to_owned(),
        }), move || { info!("{} responded!", name); });
        
        Ok(())
    }
}