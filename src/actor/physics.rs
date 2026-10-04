use crate::block::component::collision_box_component::CollisionBoxComponent;
use crate::block::component::friction_component::FrictionComponent;
use crate::entity::components::transform::Transform;
use crate::level::DimensionId;
use crate::level::Level;
use crate::registry::block_registry::BlockRegistry;
use bevy_ecs::prelude::{Component, Query, Res};
use glam::{IVec3, Vec3};

const DEFAULT_FRICTION: f32 = 0.6;

/// Simple vanilla style motion for an entity: gravity, drag and collision with blocks. The entity's
/// `Transform::position` is the bottom centre of its box and `Transform::velocity` its motion per tick.
#[derive(Component, Clone, Copy, Debug)]
pub struct Physics {
    pub width: f32,
    pub height: f32,
    pub gravity: f32,
    pub drag: f32,
    pub on_ground: bool,
}

impl Physics {
    pub const fn new(width: f32, height: f32, gravity: f32, drag: f32) -> Self {
        Self {
            width,
            height,
            gravity,
            drag,
            on_ground: false,
        }
    }

    /// Dropped items, like vanilla.
    pub const fn item() -> Self {
        Self::new(0.25, 0.25, 0.04, 0.98)
    }
}

/// Collision boxes of the blocks an entity can't pass through.
pub struct BlockCollision<'a> {
    pub level: &'a Level,
    pub blocks: &'a BlockRegistry,
    pub dimension: i32,
    pub air: Option<i32>,
}

impl BlockCollision<'_> {
    /// The block's collision box in world space; unloaded chunks count as solid so items never fall out of the world.
    pub fn collision_box(&self, block: IVec3) -> Option<(Vec3, Vec3)> {
        let origin = block.as_vec3();
        let Some(id) = self.level.get_block(self.dimension, block.x, block.y, block.z, 0) else {
            return Some((origin, origin + Vec3::ONE));
        };
        if Some(id) == self.air {
            return None;
        }
        match self.blocks.get_components(id).and_then(|components| components.get::<CollisionBoxComponent>()) {
            Some(collision) if !collision.enabled => None,
            Some(collision) => Some((origin + collision.origin, origin + collision.origin + collision.size)),
            None => Some((origin, origin + Vec3::ONE)),
        }
    }

    fn friction(&self, block: IVec3) -> f32 {
        self.level
            .get_block(self.dimension, block.x, block.y, block.z, 0)
            .and_then(|id| self.blocks.get_components(id))
            .and_then(|components| components.get::<FrictionComponent>())
            .map_or(DEFAULT_FRICTION, |friction| friction.friction)
    }

    /// Moves a box along one axis as far as it can go and returns the distance travelled.
    fn sweep(&self, min: Vec3, max: Vec3, axis: usize, delta: f32) -> f32 {
        if delta == 0.0 {
            return 0.0;
        }
        let (mut swept_min, mut swept_max) = (min, max);
        if delta < 0.0 {
            swept_min[axis] += delta;
        } else {
            swept_max[axis] += delta;
        }
        let from = (swept_min - Vec3::splat(0.5)).floor().as_ivec3();
        let to = (swept_max + Vec3::splat(0.5)).floor().as_ivec3();
        let mut allowed = delta;
        for x in from.x..=to.x {
            for y in from.y..=to.y {
                for z in from.z..=to.z {
                    let Some((block_min, block_max)) = self.collision_box(IVec3::new(x, y, z)) else { continue };
                    let overlaps = (0..3).filter(|&other| other != axis).all(|other| min[other] < block_max[other] && max[other] > block_min[other]);
                    if !overlaps {
                        continue;
                    }
                    if delta > 0.0 && block_min[axis] >= max[axis] {
                        allowed = allowed.min(block_min[axis] - max[axis]);
                    } else if delta < 0.0 && block_max[axis] <= min[axis] {
                        allowed = allowed.max(block_max[axis] - min[axis]);
                    }
                }
            }
        }
        allowed
    }
    /// Advances an entity by one tick of gravity, movement and drag, and returns whether it is on the ground.
    pub fn step(&self, physics: &Physics, position: &mut Vec3, velocity: &mut Vec3) -> bool {
        velocity.y -= physics.gravity;

        let half = Vec3::new(physics.width / 2.0, 0.0, physics.width / 2.0);
        let mut on_ground = false;
        for axis in [1, 0, 2] {
            let (min, max) = (*position - half, *position + half + Vec3::new(0.0, physics.height, 0.0));
            let moved = self.sweep(min, max, axis, velocity[axis]);
            if moved != velocity[axis] {
                on_ground |= axis == 1 && velocity[axis] < 0.0;
                velocity[axis] = if axis == 1 && on_ground { velocity[axis] * -0.5 } else { 0.0 };
                if axis == 1 && velocity[axis].abs() < 0.05 {
                    velocity[axis] = 0.0;
                }
            }
            position[axis] += moved;
        }

        let friction = if on_ground {
            self.friction((*position - Vec3::new(0.0, 0.01, 0.0)).floor().as_ivec3()) * physics.drag
        } else {
            physics.drag
        };
        *velocity *= Vec3::new(friction, physics.drag, friction);
        if velocity.x.abs() < 1e-3 {
            velocity.x = 0.0;
        }
        if velocity.z.abs() < 1e-3 {
            velocity.z = 0.0;
        }
        on_ground
    }
}

pub fn apply_physics(mut entities: Query<(&mut Transform, &mut Physics, &DimensionId)>, level: Option<Res<Level>>, blocks: Res<BlockRegistry>) {
    let Some(level) = level else { return };
    let air = blocks.get_block_id("minecraft:air");
    for (mut transform, mut physics, dimension) in &mut entities {
        let collision = BlockCollision {
            level: &level,
            blocks: &blocks,
            dimension: dimension.0,
            air,
        };
        let below = (transform.position - Vec3::new(0.0, 0.01, 0.0)).floor().as_ivec3();
        if physics.on_ground && transform.velocity.length_squared() < 1e-6 && collision.collision_box(below).is_some() {
            continue;
        }

        let (mut position, mut velocity) = (transform.position, transform.velocity);
        let on_ground = collision.step(&physics, &mut position, &mut velocity);
        physics.on_ground = on_ground;
        if position == transform.position && velocity == transform.velocity {
            continue;
        }
        transform.position = position;
        transform.velocity = velocity;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::dimension_type::DimensionType;
    use crate::level::generator::r#impl::flat::{FlatGenerator, FlatLayer};

    fn flat_level(blocks: &BlockRegistry, top: &str) -> Level {
        bevy_tasks::AsyncComputeTaskPool::get_or_init(bevy_tasks::TaskPool::default);
        let id = |name: &str| blocks.get_block_id(name).expect(name);
        let mut level = Level::in_memory("test", 0);
        let dimension = level.insert_dimension(
            DimensionType::Overworld,
            FlatGenerator {
                layers: vec![
                    FlatLayer {
                        block_id: id("minecraft:stone"),
                        height: 3,
                    },
                    FlatLayer { block_id: id(top), height: 1 },
                ],
                biome: 1,
                air_id: id("minecraft:air"),
                min_sub_chunk_y: DimensionType::Overworld.min_sub_chunk_y(),
                sub_chunk_count: DimensionType::Overworld.sub_chunk_count(),
            },
        );
        dimension.request_chunk(0, 0);
        while dimension.tick().is_empty() {
            std::thread::yield_now();
        }
        level
    }

    fn drop_item(top: &str, start: Vec3, velocity: Vec3) -> (Vec3, Vec3, bool, usize) {
        let mut blocks = BlockRegistry::new();
        blocks.register_all(crate::block::r#impl::DEFINITIONS.iter().copied());
        let level = flat_level(&blocks, top);
        let collision = BlockCollision {
            level: &level,
            blocks: &blocks,
            dimension: 0,
            air: blocks.get_block_id("minecraft:air"),
        };
        let (mut position, mut velocity, mut on_ground) = (start, velocity, false);
        let mut ticks = 0;
        while ticks < 400 && !(on_ground && velocity == Vec3::ZERO) {
            on_ground = collision.step(&Physics::item(), &mut position, &mut velocity);
            ticks += 1;
        }
        (position, velocity, on_ground, ticks)
    }

    #[test]
    fn items_fall_and_settle_on_the_surface() {
        let (position, velocity, on_ground, ticks) = drop_item("minecraft:grass_block", Vec3::new(8.5, 10.0, 8.5), Vec3::new(0.05, 0.2, 0.0));
        assert!(on_ground, "settled after {ticks} ticks at {position}");
        assert_eq!(velocity, Vec3::ZERO);
        assert!((position.y - 4.0).abs() < 1e-4, "resting on top of the 4 block floor, got {position}");
        assert!(ticks < 100, "took {ticks} ticks to settle");
    }

    #[test]
    fn ice_lets_items_slide_further() {
        let start = Vec3::new(1.5, 4.0, 8.5);
        let (on_grass, ..) = drop_item("minecraft:grass_block", start, Vec3::new(0.2, 0.0, 0.0));
        let (on_ice, ..) = drop_item("minecraft:ice", start, Vec3::new(0.2, 0.0, 0.0));
        let (grass, ice) = (on_grass.x - start.x, on_ice.x - start.x);
        assert!(grass > 0.2 && grass < 1.0, "grass slid {grass}");
        assert!(ice > grass * 4.0, "grass slid {grass}, ice slid {ice}");
    }
}
