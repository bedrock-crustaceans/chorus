use crate::block::component::block_component::BlockComponent;

#[derive(Clone, Debug)]
pub struct MapColorComponent {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl BlockComponent for MapColorComponent {}
