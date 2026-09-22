use crate::block::component::block_component::BlockComponent;

#[derive(Clone, Debug)]
pub struct LightDampeningComponent {
    pub dampening: i32,
}

impl BlockComponent for LightDampeningComponent {}
