use crate::command::dispatch::CommandRequestedMessage;
use bevy_app::{App, AppExit, Plugin, PreUpdate, Startup};
use bevy_ecs::entity::Entity;
use bevy_ecs::message::MessageWriter;
use bevy_ecs::prelude::{Commands, Component, Res, ResMut, Resource};
use bevy_ecs::system::Local;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{info, warn};

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
        std::process::exit(130);
    }
}

fn setup_console(mut commands: Commands) {
    let console = commands.spawn(Console).id();
    commands.insert_resource(ConsoleSender(console));
    if !platform::install_interrupt_handler(interrupt) {
        warn!("failed to install the interrupt handler, ctrl+c will exit without saving");
    }
}

fn read_console(mut input: ResMut<ConsoleInput>, sender: Option<Res<ConsoleSender>>, mut writer: MessageWriter<CommandRequestedMessage>) {
    let Some(sender) = sender else { return };
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
