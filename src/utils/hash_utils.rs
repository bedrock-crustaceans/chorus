#[allow(non_snake_case)]
pub mod HashUtils {
    use crate::block::state::block_state::BlockState;
    use atomicow::CowArc;
    use std::collections::HashMap;

    pub fn hash_nbt(mut compound: nbtx::Compound) -> i32 {
        compound.sort_keys();

        fnv1a_32::hash(nbtx::to_le_bytes(&nbtx::Value::Compound(compound)).unwrap().as_slice()) as i32
    }

    pub fn hash_block_permutation(identifier: &str, states: &HashMap<CowArc<'static, str>, BlockState>) -> i32 {
        if identifier == "minecraft:unknown" {
            return -2;
        }

        let mut states_tag = nbtx::Compound::new();
        for (id, val) in states {
            match val {
                BlockState::Bool(val) => {
                    states_tag.insert(id.as_ref().into(), nbtx::Value::Byte(if *val { 1 } else { 0 }));
                }
                BlockState::Int(val) => {
                    states_tag.insert(id.as_ref().into(), nbtx::Value::Int(*val));
                }
                BlockState::Enum(val) => {
                    states_tag.insert(id.as_ref().into(), nbtx::Value::String(val.as_ref().into()));
                }
            }
        }

        let mut tag = nbtx::Compound::new();

        tag.insert("name".into(), nbtx::Value::String(identifier.into()));
        tag.insert("states".into(), nbtx::Value::Compound(states_tag));

        hash_nbt(tag)
    }

    pub mod fnv1a_32 {
        const FNV1A_32_INIT: u32 = 0x811C9DC5;
        const FNV1A_32_PRIME: u32 = 0x01000193;

        pub const fn hash(data: &[u8]) -> u32 {
            let mut hash = FNV1A_32_INIT;
            let mut i = 0;
            while i < data.len() {
                hash ^= data[i] as u32;
                hash = hash.wrapping_mul(FNV1A_32_PRIME);
                i += 1;
            }
            hash
        }
    }
}
