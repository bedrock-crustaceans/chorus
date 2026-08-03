use bedrock::protocol::v975::types::{ItemStackNetIdVariant, NetworkItemStackDescriptorV2};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ItemStack {
    pub id: i16,
    pub count: u16,
    pub meta: u32,
    /// Only set for items that place a block, 0 means the item is not a block.
    pub block_runtime_id: i32,
}

impl ItemStack {
    pub const fn air() -> Self {
        Self {
            id: 0,
            count: 0,
            meta: 0,
            block_runtime_id: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.id == 0 || self.count == 0
    }

    /// `net_id` identifies the stack across item stack requests. Empty slots must never carry one.
    pub fn to_descriptor(&self, net_id: Option<i32>) -> NetworkItemStackDescriptorV2 {
        if self.is_empty() {
            return NetworkItemStackDescriptorV2 {
                id: 0,
                stack_size: 0,
                aux_value: 0,
                net_id: None,
                block_runtime_id: 0,
                user_data_buffer: vec![],
            };
        }

        NetworkItemStackDescriptorV2 {
            id: self.id,
            stack_size: self.count,
            aux_value: self.meta,
            net_id: net_id.map(ItemStackNetIdVariant::ItemStackNetId),
            block_runtime_id: self.block_runtime_id as u32,
            user_data_buffer: vec![],
        }
    }
}
