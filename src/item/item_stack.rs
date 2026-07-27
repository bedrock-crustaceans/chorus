use bedrock::protocol::v662::types::NetworkItemStackDescriptor;
use bedrock::protocol::v975::types::NetworkItemStackDescriptorV2;

pub const MAX_STACK_SIZE: u16 = 64;

#[derive(Clone, Copy, Debug)]
pub struct ItemStack {
    block_id: i32,
    count: u16,
}

impl ItemStack {
    pub fn new(block_id: i32, count: u16) -> Self {
        Self { block_id, count }
    }

    pub fn block_id(&self) -> i32 {
        self.block_id
    }

    pub fn count(&self) -> u16 {
        self.count
    }

    pub fn set_count(&mut self, count: u16) {
        self.count = count;
    }

    pub fn is_same_kind(&self, other: &ItemStack) -> bool {
        self.block_id == other.block_id
    }

    pub fn to_descriptor(&self) -> NetworkItemStackDescriptor {
        NetworkItemStackDescriptor {
            id: self.block_id,
            stack_size: Some(self.count),
            aux_value: Some(0),
            net_id_variant: Some(None),
            block_runtime_id: Some(self.block_id),
            user_data_buffer: Some(vec![]),
        }
    }

    pub fn to_descriptor_v2(&self) -> NetworkItemStackDescriptorV2 {
        NetworkItemStackDescriptorV2 {
            id: self.block_id as i16,
            stack_size: self.count,
            aux_value: 0,
            net_id: None,
            block_runtime_id: self.block_id as u32,
            user_data_buffer: vec![],
        }
    }
}
