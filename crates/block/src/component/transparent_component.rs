use crate::component::block_component::BlockComponent;

#[derive(Clone, Debug)]
pub struct TransparentComponent {
    pub transparent: bool,
}

impl BlockComponent for TransparentComponent {}
