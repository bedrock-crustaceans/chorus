use bevy_ecs::prelude::Component;
use glam::{Vec2, Vec3};

#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Vec2,
    pub velocity: Vec3,
}

impl Transform {
    pub fn at(position: Vec3) -> Self {
        Self { position, ..Self::default() }
    }
}
