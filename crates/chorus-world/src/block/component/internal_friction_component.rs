use crate::block::component::block_component::BlockComponent;

#[derive(Clone, Debug)]
pub struct InternalFrictionComponent {
    pub internal_friction: f32,
}

impl BlockComponent for InternalFrictionComponent {}
