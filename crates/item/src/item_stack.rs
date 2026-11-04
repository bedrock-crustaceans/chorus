use bedrock::protocol::ProtoVersionTypes;
use bedrock::protocol::v2168::types::{NetworkItemStackDescriptor, NetworkItemStackDescriptorV2};
use chorus_core::protocol::BedrockProtocol;

/// Item user data with nothing in it: no NBT (an i16 of 0), then empty can place on and can
/// destroy lists (two i32 counts). An empty buffer is not valid, clients run off its end.
const EMPTY_USER_DATA: [u8; 10] = [0; 10];

fn user_data_buffer(nbt: &[u8]) -> Vec<u8> {
    if nbt.is_empty() {
        return EMPTY_USER_DATA.to_vec();
    }
    let mut buffer = Vec::with_capacity(nbt.len() + 11);
    buffer.extend_from_slice(&(-1i16).to_le_bytes());
    buffer.push(1);
    buffer.extend_from_slice(nbt);
    buffer.extend_from_slice(&0u32.to_le_bytes());
    buffer.extend_from_slice(&0u32.to_le_bytes());
    buffer
}

pub fn user_data_of(buffer: &[u8]) -> Vec<u8> {
    const HEADER: [u8; 3] = [0xFF, 0xFF, 1];
    const FOOTER: usize = 8;
    if buffer.len() <= HEADER.len() + FOOTER || buffer[..HEADER.len()] != HEADER {
        return Vec::new();
    }
    buffer[HEADER.len()..buffer.len() - FOOTER].to_vec()
}

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

    pub fn to_descriptor_with_user_data(&self, net_id: Option<i32>, user_data: &[u8]) -> <BedrockProtocol as ProtoVersionTypes>::NetworkItemStackDescriptorV2 {
        if self.is_empty() {
            return self.to_descriptor(None);
        }

        NetworkItemStackDescriptorV2 {
            id: self.id,
            stack_size: self.count,
            aux_value: self.meta,
            net_id,
            block_runtime_id: self.block_runtime_id as u32,
            user_data_buffer: user_data_buffer(user_data),
        }
    }

    pub fn to_actor_descriptor(&self) -> <BedrockProtocol as ProtoVersionTypes>::NetworkItemStackDescriptor {
        self.to_actor_descriptor_with_data(&[])
    }

    pub fn to_actor_descriptor_with_data(&self, user_data: &[u8]) -> <BedrockProtocol as ProtoVersionTypes>::NetworkItemStackDescriptor {
        NetworkItemStackDescriptor {
            id: self.id,
            stack_size: self.count,
            aux_value: self.meta,
            net_id: None,
            block_runtime_id: self.block_runtime_id as u32,
            user_data_buffer: user_data_buffer(user_data),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack() -> ItemStack {
        ItemStack {
            id: 5,
            count: 1,
            meta: 0,
            block_runtime_id: 0,
        }
    }

    #[test]
    fn user_data_survives_the_envelope_round_trip() {
        let stack = ItemStack {
            id: 5,
            count: 1,
            meta: 0,
            block_runtime_id: 0,
        };
        let nbt = [10u8, 0, 0, 0];
        let descriptor = stack.to_descriptor_with_user_data(None, &nbt);
        assert_eq!(user_data_of(&descriptor.user_data_buffer), nbt.to_vec());
        assert!(user_data_of(&stack.to_descriptor(None).user_data_buffer).is_empty());
        assert!(user_data_of(&[]).is_empty());
    }

    #[test]
    fn user_data_is_wrapped_in_the_descriptor_envelope() {
        let nbt = [0x0A, 0x00, 0x00, 0x00];
        let descriptor = stack().to_descriptor_with_user_data(Some(7), &nbt);
        assert_eq!(&descriptor.user_data_buffer[..3], &[0xFF, 0xFF, 1]);
        assert_eq!(&descriptor.user_data_buffer[3..3 + nbt.len()], &nbt);
        assert_eq!(&descriptor.user_data_buffer[descriptor.user_data_buffer.len() - 8..], &[0u8; 8]);
        assert_eq!(descriptor.net_id, Some(7));
    }

    #[test]
    fn empty_stacks_never_carry_user_data() {
        let descriptor = ItemStack::air().to_descriptor_with_user_data(Some(1), &[0x0A, 0x00, 0x00, 0x00]);
        assert!(descriptor.user_data_buffer.is_empty());
        assert_eq!(descriptor.net_id, None);
    }
}
