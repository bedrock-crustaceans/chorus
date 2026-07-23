use crate::block::component::block_component::BlockComponent;
use vek::Vec3;

#[derive(Clone, Debug)]
pub struct CollisionBoxComponent {
    pub origin: Vec3<f32>,
    pub size: Vec3<f32>,
    pub enabled: bool,
}

impl BlockComponent for CollisionBoxComponent {}

impl CollisionBoxComponent {
    pub const fn default() -> CollisionBoxComponent {
        Self {
            origin: Vec3::new(0.0, 0.0, 0.0),
            size: Vec3::new(1.0, 1.0, 1.0),
            enabled: true,
        }
    }

    pub const fn new(origin: Vec3<f32>, size: Vec3<f32>) -> CollisionBoxComponent {
        Self { origin, size, enabled: true }
    }

    pub const fn enabled(enabled: bool) -> CollisionBoxComponent {
        Self {
            origin: Vec3::new(0.0, 0.0, 0.0),
            size: Vec3::new(1.0, 1.0, 1.0),
            enabled,
        }
    }
}
