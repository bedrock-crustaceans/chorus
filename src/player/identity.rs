use bevy_ecs::prelude::*;

#[derive(Component, Clone, Debug)]
pub struct PlayerIdentity {
    name: String,
    xuid: String,
}

impl PlayerIdentity {
    pub fn new(name: String, xuid: String) -> Self {
        Self { name, xuid }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn xuid(&self) -> &str {
        &self.xuid
    }
}
