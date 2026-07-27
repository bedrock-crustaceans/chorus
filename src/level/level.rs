use crate::level::BlockUpdatedMessage;
use crate::level::dimension::Dimension;
// use crate::level::generator::flat::{FlatGenerator, FlatLayer};
use crate::level::generator::random::RandomGenerator;
use crate::registry::block_registry::BlockRegistry;
use bevy_ecs::message::MessageWriter;
use bevy_ecs::prelude::{Commands, Resource};
use bevy_ecs::system::Res;
use std::collections::HashMap;

#[derive(Resource)]
pub struct Level {
    pub dimensions: HashMap<i32, Dimension>,
}

impl Level {
    pub fn init(mut commands: Commands, _registry: Res<BlockRegistry>) {
        // let bedrock_id = registry.get_block_id("minecraft:bedrock").unwrap_or(0);
        // let dirt_id = registry.get_block_id("minecraft:dirt").unwrap_or(0);
        // let grass_id = registry.get_block_id("minecraft:grass_block").unwrap_or(0);
        //
        // let generator = FlatGenerator {
        //     layers: vec![
        //         FlatLayer { block_id: bedrock_id, height: 1 },
        //         FlatLayer { block_id: dirt_id, height: 2 },
        //         FlatLayer { block_id: grass_id, height: 1 },
        //     ],
        //     biome: 1,
        // };

        let generator = RandomGenerator { biome: 1 };

        let overworld = Dimension::new(0, -4, 19, Box::new(generator));

        let mut level = Level { dimensions: HashMap::new() };
        level.dimensions.insert(0, overworld);

        commands.insert_resource(level);
    }

    pub fn dimension(&self, id: i32) -> Option<&Dimension> {
        self.dimensions.get(&id)
    }

    pub fn dimension_mut(&mut self, id: i32) -> Option<&mut Dimension> {
        self.dimensions.get_mut(&id)
    }

    pub fn overworld(&self) -> &Dimension {
        self.dimensions.get(&0).expect("overworld dimension not initialised")
    }

    pub fn overworld_mut(&mut self) -> &mut Dimension {
        self.dimensions.get_mut(&0).expect("overworld dimension not initialised")
    }

    pub fn get_block(&self, dim: i32, x: i32, y: i32, z: i32, layer: usize) -> Option<i32> {
        self.dimension(dim)?.get_block(x, y, z, layer)
    }

    pub fn set_block(&mut self, dim: i32, x: i32, y: i32, z: i32, layer: usize, block_id: i32, writer: &mut MessageWriter<BlockUpdatedMessage>) -> bool {
        let changed = match self.dimension_mut(dim) {
            Some(d) => d.set_block(x, y, z, layer, block_id),
            None => return false,
        };
        if changed {
            writer.write(BlockUpdatedMessage {
                dimension_id: dim,
                x,
                y,
                z,
                layer,
                block_id,
            });
        }
        changed
    }

    pub fn break_block(&mut self, dim: i32, x: i32, y: i32, z: i32, air_id: i32, writer: &mut MessageWriter<BlockUpdatedMessage>) -> Option<i32> {
        let previous = self.get_block(dim, x, y, z, 0)?;
        if previous == air_id {
            return None;
        }
        if self.set_block(dim, x, y, z, 0, air_id, writer) { Some(previous) } else { None }
    }

    pub fn place_block(&mut self, dim: i32, x: i32, y: i32, z: i32, block_id: i32, air_id: i32, writer: &mut MessageWriter<BlockUpdatedMessage>) -> bool {
        match self.get_block(dim, x, y, z, 0) {
            Some(current) if current == air_id => self.set_block(dim, x, y, z, 0, block_id, writer),
            _ => false,
        }
    }
}
