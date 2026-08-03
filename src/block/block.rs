use crate::block::block_permutation::BlockPermutation;
use crate::level::level::Level;
use glam::IVec3;

pub struct Block {
    permutation: BlockPermutation,
    position: IVec3,
    layer: i32,
    level: Level,
}
