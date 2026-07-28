use crate::level::level::Level;
use crate::registry::block_registry::BlockRegistry;
use crate::resource::ResourcePacks;
use bevy_app::{App, Plugin, Startup};
use bevy_ecs::prelude::IntoScheduleConfigs;
use command_registry::CommandRegistry;

pub mod block_registry;
pub mod command_registry;

pub struct Registry;

impl Plugin for Registry {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (BlockRegistry::init, CommandRegistry::init, ResourcePacks::load, Level::init.after(BlockRegistry::init)));
    }
}
