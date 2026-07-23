use crate::block::component::block_component::BlockComponent;

#[derive(Clone, Debug)]
pub struct FrictionComponent {
    pub friction: f32,
}

impl BlockComponent for FrictionComponent {}
