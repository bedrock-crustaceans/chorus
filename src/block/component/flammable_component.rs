use crate::block::component::block_component::BlockComponent;

#[derive(Clone, Debug)]
pub struct FlammableComponent {
    pub catch_chance: i32,
    pub destroy_chance: i32,
}

impl BlockComponent for FlammableComponent {}
