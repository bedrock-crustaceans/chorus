use crate::registry::block_registry::BlockRegistry;
use crate::resource::ResourcePacks;
use bevy_app::{App, Plugin, Startup};

pub mod block_registry;

pub struct Registry;

impl Plugin for Registry {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (BlockRegistry::init, ResourcePacks::load));
    }
}
