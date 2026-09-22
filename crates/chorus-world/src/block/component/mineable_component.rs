use crate::block::component::block_component::BlockComponent;

#[derive(Clone, Debug)]
pub struct MineableComponent {
    pub hardness: f32,
    pub item_specific_hardness: &'static [(&'static str, f32)],
}

impl BlockComponent for MineableComponent {}

impl MineableComponent {
    pub const fn hardness(hardness: f32) -> Self {
        Self {
            hardness,
            item_specific_hardness: &[],
        }
    }
}
