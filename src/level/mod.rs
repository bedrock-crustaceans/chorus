pub mod biome;
mod bit_array;
pub mod chunk;
pub mod chunk_state;
pub mod dimension;
pub mod generator;
pub mod level;
mod palette;
pub mod sub_chunk;

use bevy_ecs::prelude::{Component, Message};

#[derive(Component, Clone, Copy, Debug)]
pub struct DimensionId(pub i32);

#[derive(Message, Clone)]
pub struct BlockUpdatedMessage {
    pub dimension_id: i32,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub layer: usize,
    pub block_id: i32,
}

#[derive(Message, Clone)]
pub struct BlockDestroyedMessage {
    pub dimension_id: i32,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub block_id: i32,
}
