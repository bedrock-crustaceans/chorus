#[allow(non_snake_case)]
pub mod HashUtils {
    use crate::block::state::block_state::BlockState;
    use atomicow::CowArc;
    use std::collections::HashMap;

    pub fn hash_block_permutation(identifier: &str, states: &HashMap<CowArc<'static, str>, BlockState>) -> i32 {
        if identifier == "minecraft:unknown" {
            return -2;
        }

        let mut tag = nbtx::Compound::new();
        tag.insert("name".into(), nbtx::Value::String(identifier.into()));
        tag.insert(
            "states".into(),
            nbtx::Value::Compound({
                let mut map = nbtx::Compound::from_iter(states.iter().map(|(id, val)| {
                    (
                        id.as_ref().into(),
                        match val {
                            BlockState::Bool(val) => nbtx::Value::Byte(if *val { 1 } else { 0 }),
                            BlockState::Int(val) => nbtx::Value::Int(*val),
                            BlockState::Enum(val) => nbtx::Value::String(val.as_ref().into()),
                        },
                    )
                }));
                map.sort_unstable_keys();
                map
            }),
        );
        tag.sort_unstable_keys();

        // TODO: return error here instead of unwrap
        fnv1a_32::hash(nbtx::to_le_bytes(&nbtx::Value::Compound(tag)).unwrap().as_slice()) as i32
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
