use glam::IVec3;
use crate::block::block_permutation::BlockPermutation;
use crate::level::level::Level;

pub struct Block {
    permutation: BlockPermutation,
    position: IVec3,
    layer: i32,
    level: Level,
}
