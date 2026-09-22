use crate::block::component::block_component::BlockComponent;

#[derive(Clone, Debug)]
pub struct MoveableComponent {
    pub movement: Movement,
    pub sticky: bool,
}

#[derive(Clone, Debug)]
pub enum Movement {
    Both,
    Push,
    Break,
    None,
}

impl BlockComponent for MoveableComponent {}
