use crate::command::parameter::CommandOverload;
use crate::command::registry::CommandRegistry;
use crate::command::sender::CommandSender;
use bedrock::protocol::v898::packets::CommandPermissionLevelString;

pub type CommandResult = Result<(), String>;

pub trait Command: Send + Sync {
    fn name(&self) -> &str;

    fn description(&self) -> &str;

    fn aliases(&self) -> &[&'static str] {
        &[]
    }

    fn permission(&self) -> CommandPermissionLevelString {
        CommandPermissionLevelString::Any
    }

    fn overloads(&self) -> Vec<CommandOverload> {
        vec![CommandOverload::default()]
    }

    fn usage(&self) -> String {
        let overloads = self.overloads();
        if overloads.is_empty() {
            return format!("/{}", self.name());
        }

        overloads.iter().map(|overload| format!("/{} {}", self.name(), overload.usage())).collect::<Vec<_>>().join("\n")
    }

    fn execute(&self, registry: &CommandRegistry, sender: &mut CommandSender, args: &[&str]) -> CommandResult;
}
