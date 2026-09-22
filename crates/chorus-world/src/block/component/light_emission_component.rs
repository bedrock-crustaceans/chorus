use crate::block::component::block_component::BlockComponent;

#[derive(Clone, Debug)]
pub struct LightEmissionComponent {
    pub emission: i32,
}

impl BlockComponent for LightEmissionComponent {}
