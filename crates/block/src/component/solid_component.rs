use crate::component::block_component::BlockComponent;

#[derive(Debug, Clone)]
pub struct SolidComponent {
    pub solid: bool,
}

impl BlockComponent for SolidComponent {}
