use crate::block::component::block_component::BlockComponent;

#[derive(Clone, Debug)]
pub struct ReplaceableComponent {
    pub replaceable: bool,
}

impl BlockComponent for ReplaceableComponent {}
