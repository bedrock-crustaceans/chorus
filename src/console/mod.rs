use crate::command::dispatch::CommandRequestedMessage;
use bevy_app::{App, AppExit, Plugin, PreUpdate, Startup};
use bevy_ecs::entity::Entity;
use bevy_ecs::message::MessageWriter;
use bevy_ecs::prelude::{Commands, Component, Res, ResMut, Resource};
use bevy_ecs::system::Local;
use std::io::IsTerminal;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{info, warn};

mod prompt;

pub use prompt::ConsoleWriter;

#[cfg(unix)]
mod unix;
#[cfg(unix)]
use unix as platform;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as platform;

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

#[derive(Component)]
pub struct Console;

#[derive(Resource)]
pub struct ConsoleSender(pub Entity);

#[derive(Resource, Default)]
struct ConsoleInput {
    terminal: bool,
    pending: Vec<u8>,
    closed: bool,
}

pub struct ConsolePlugin;

impl Plugin for ConsolePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ConsoleInput>()
            .add_systems(Startup, setup_console)
            .add_systems(PreUpdate, (read_console, handle_interrupt));
    }
}

fn interrupt() {
    if INTERRUPTED.swap(true, Ordering::SeqCst) {
        prompt::stop();
        std::process::exit(130);
    }
}

pub fn shutdown() {
    prompt::stop();
}

fn setup_console(mut commands: Commands, mut input: ResMut<ConsoleInput>) {
    let console = commands.spawn(Console).id();
    commands.insert_resource(ConsoleSender(console));
    if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
        match prompt::start() {
            Ok(()) => {
                input.terminal = true;
                let hook = std::panic::take_hook();
                std::panic::set_hook(Box::new(move |info| {
                    prompt::stop();
                    hook(info);
                }));
            }
            Err(err) => warn!("failed to set up the console prompt, falling back to plain input: {err}"),
        }
    }
    if !platform::install_interrupt_handler(interrupt) {
        warn!("failed to install the interrupt handler, ctrl+c will exit without saving");
    }
}

fn read_console(mut input: ResMut<ConsoleInput>, sender: Option<Res<ConsoleSender>>, mut writer: MessageWriter<CommandRequestedMessage>) {
    let Some(sender) = sender else { return };
    if input.terminal {
        for line in prompt::read() {
            match line {
                prompt::Input::Line(line) => {
                    writer.write(CommandRequestedMessage { entity: sender.0, line });
                }
                prompt::Input::Interrupt => interrupt(),
            }
        }
        return;
    }
    if input.closed {
        return;
    }
    let mut buffer = [0u8; 4096];
    loop {
        match platform::read_available(&mut buffer) {
            Some(0) => break,
            Some(read) => input.pending.extend_from_slice(&buffer[..read]),
            None => {
                input.closed = true;
                break;
            }
        }
    }

    while let Some(end) = input.pending.iter().position(|&byte| byte == b'\n') {
        let line: Vec<u8> = input.pending.drain(..=end).collect();
        let line = String::from_utf8_lossy(&line);
        let line = line.trim();
        if !line.is_empty() {
            writer.write(CommandRequestedMessage {
                entity: sender.0,
                line: line.to_owned(),
            });
        }
    }
}

fn handle_interrupt(mut exit: MessageWriter<AppExit>, mut handled: Local<bool>) {
    if !*handled && INTERRUPTED.load(Ordering::SeqCst) {
        *handled = true;
        info!("stopping, press ctrl+c again to exit without saving");
        exit.write(AppExit::Success);
    }
}

pub fn reply(message: &str) {
    let text = if std::io::stdout().is_terminal() { to_ansi(message) } else { strip_formatting(message) };
    prompt::print(format!("{text}\n").as_bytes());
}

fn ansi_code(code: char) -> Option<&'static str> {
    Some(match code {
        '0' => "\x1b[30m",
        '1' => "\x1b[34m",
        '2' => "\x1b[32m",
        '3' => "\x1b[36m",
        '4' => "\x1b[31m",
        '5' => "\x1b[35m",
        '6' => "\x1b[33m",
        '7' => "\x1b[37m",
        '8' => "\x1b[90m",
        '9' => "\x1b[94m",
        'a' => "\x1b[92m",
        'b' => "\x1b[96m",
        'c' => "\x1b[91m",
        'd' => "\x1b[95m",
        'e' => "\x1b[93m",
        'f' => "\x1b[97m",
        'g' => "\x1b[38;2;221;214;5m",
        'h' => "\x1b[38;2;227;212;209m",
        'i' => "\x1b[38;2;206;202;202m",
        'j' => "\x1b[38;2;68;58;59m",
        'm' => "\x1b[38;2;151;22;7m",
        'n' => "\x1b[38;2;180;104;77m",
        'p' => "\x1b[38;2;222;177;45m",
        'q' => "\x1b[38;2;71;160;54m",
        's' => "\x1b[38;2;44;186;168m",
        't' => "\x1b[38;2;33;73;123m",
        'u' => "\x1b[38;2;154;92;198m",
        'v' => "\x1b[38;2;235;113;20m",
        'l' => "\x1b[1m",
        'o' => "\x1b[3m",
        'r' => "\x1b[0m",
        _ => return None,
    })
}

fn to_ansi(message: &str) -> String {
    let mut out = String::with_capacity(message.len() + 16);
    let mut chars = message.chars();
    while let Some(char) = chars.next() {
        if char != '§' {
            out.push(char);
            continue;
        }
        let Some(code) = chars.next() else { break };
        if let Some(ansi) = ansi_code(code.to_ascii_lowercase()) {
            out.push_str(ansi);
        }
    }
    out.push_str("\x1b[0m");
    out
}

pub fn strip_formatting(message: &str) -> String {
    let mut stripped = String::with_capacity(message.len());
    let mut chars = message.chars();
    while let Some(char) = chars.next() {
        if char == '§' {
            chars.next();
        } else {
            stripped.push(char);
        }
    }
    stripped
}
