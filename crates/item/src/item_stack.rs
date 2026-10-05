use bedrock::protocol::ProtoVersionTypes;
use bedrock::protocol::v2168::types::{NetworkItemStackDescriptor, NetworkItemStackDescriptorV2};
use chorus_core::protocol::BedrockProtocol;

/// Item user data with nothing in it: no NBT (an i16 of 0), then empty can place on and can
/// destroy lists (two i32 counts). An empty buffer is not valid, clients run off its end.
const EMPTY_USER_DATA: [u8; 10] = [0; 10];

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

    /// Compares the item type only, so stacks of different sizes still match.
    pub fn is_same(&self, other: &Self) -> bool {
        self.id == other.id && self.meta == other.meta && self.block_runtime_id == other.block_runtime_id
    }

    /// `net_id` identifies the stack across item stack requests. Empty slots must never carry one.
    pub fn to_descriptor(&self, net_id: Option<i32>) -> <BedrockProtocol as ProtoVersionTypes>::NetworkItemStackDescriptorV2 {
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
            net_id,
            block_runtime_id: self.block_runtime_id as u32,
            user_data_buffer: EMPTY_USER_DATA.to_vec(),
        }
    }

    pub fn to_actor_descriptor(&self) -> <BedrockProtocol as ProtoVersionTypes>::NetworkItemStackDescriptor {
        NetworkItemStackDescriptor {
            id: self.id,
            stack_size: self.count,
            aux_value: self.meta,
            net_id: None,
            block_runtime_id: self.block_runtime_id as u32,
            user_data_buffer: EMPTY_USER_DATA.to_vec(),
        }
    }
}
