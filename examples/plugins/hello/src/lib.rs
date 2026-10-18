//! The smallest plugin that touches all three v0.1 pillars: a command, an observed event, and a
//! blocking event it actually changes. Doubles as the host's test fixture.

use std::cell::Cell;

use chorus_plugin_api::{command, logging, text};
use chorus_plugin_api::{CommandSender, ConsumedArgs, Definition, Event, LogLevel, Metadata, Overload, PermissionLevel, Plugin};

struct Hello {
    greet_command: Cell<u32>,
}

impl Plugin for Hello {
    fn new() -> Self {
        Self { greet_command: Cell::new(u32::MAX) }
    }

    fn metadata(&self) -> Metadata {
        Metadata {
            name: "hello".to_owned(),
            version: "0.1.0".to_owned(),
            authors: vec!["Chorus".to_owned()],
            description: "Example plugin: a command, a join message, and a chat filter".to_owned(),
        }
    }

    fn on_load(&self) -> Result<(), String> {
        let id = command::register(&Definition {
            name: "hello".to_owned(),
            description: "Says hello from a wasm plugin".to_owned(),
            aliases: vec![],
            permission: PermissionLevel::Member,
            overloads: vec![Overload { parameters: vec![] }],
        })?;
        self.greet_command.set(id);
        logging::log(LogLevel::Info, "hello plugin loaded");
        Ok(())
    }

    fn on_event(&self, event: Event) -> Event {
        match event {
            Event::PlayerJoin(data) => {
                logging::log(LogLevel::Info, &format!("{} joined", data.player.name));
                Event::PlayerJoin(data)
            }
            // yeah, the dumbest possible chat filter, but it proves the host reads the event back
            Event::PlayerChat(mut data) => {
                if data.message.contains("badword") {
                    data.cancelled = true;
                } else if data.message.contains("hug") {
                    data.message = data.message.replace("hug", "§dhug§r");
                }
                Event::PlayerChat(data)
            }
            other => other,
        }
    }

    fn on_command(&self, id: u32, sender: CommandSender, _args: ConsumedArgs) -> Result<i32, String> {
        if id != self.greet_command.get() {
            return Err(format!("unknown command id {id}"));
        }
        if sender.is_console {
            logging::log(LogLevel::Info, "Hello from a wasm plugin");
        } else {
            text::send_message(&sender.name, "Hello from a wasm plugin")?;
        }
        Ok(1)
    }
}

chorus_plugin_api::register_plugin!(Hello);
