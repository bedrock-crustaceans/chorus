#[allow(non_snake_case)]
pub mod HashUtils {
    use crate::state::block_state::BlockState;
    use atomicow::CowArc;
    use std::collections::HashMap;

    pub fn hash_block_permutation(identifier: &str, states: &HashMap<CowArc<'static, str>, BlockState>) -> i32 {
        let states = nbtx::Compound::from_iter(states.iter().map(|(id, val)| {
            (
                id.as_ref().into(),
                match val {
                    BlockState::Bool(val) => nbtx::Value::Byte(if *val { 1 } else { 0 }),
                    BlockState::Int(val) => nbtx::Value::Int(*val),
                    BlockState::Enum(val) => nbtx::Value::String(val.as_ref().into()),
                },
            )
        }));
        hash_block_nbt(identifier, states)
    }

    pub fn hash_block_nbt(identifier: &str, mut states: nbtx::Compound) -> i32 {
        if identifier == "minecraft:unknown" {
            return -2;
        }

        states.sort_unstable_keys();
        let mut tag = nbtx::Compound::new();
        tag.insert("name".into(), nbtx::Value::String(identifier.into()));
        tag.insert("states".into(), nbtx::Value::Compound(states));
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

#[cfg(test)]
mod tests {
    use super::HashUtils::{self, fnv1a_32};
    use crate::state::block_state::BlockState;
    use atomicow::CowArc;
    use std::collections::HashMap;

    fn states(entries: &[(&'static str, BlockState)]) -> HashMap<CowArc<'static, str>, BlockState> {
        entries.iter().map(|(name, value)| (CowArc::Borrowed(*name), value.clone())).collect()
    }

    #[test]
    fn fnv1a_matches_the_published_test_vectors() {
        assert_eq!(fnv1a_32::hash(b""), 0x811C9DC5);
        assert_eq!(fnv1a_32::hash(b"a"), 0xE40C292C);
        assert_eq!(fnv1a_32::hash(b"foobar"), 0xBF9CF968);
    }

    #[test]
    fn a_stateless_block_has_a_fixed_hash() {
        assert_eq!(HashUtils::hash_block_permutation("minecraft:air", &HashMap::new()), -604749536);
    }

    #[test]
    fn hashing_is_deterministic() {
        let states = states(&[("pillar_axis", "x".into())]);
        let first = HashUtils::hash_block_permutation("minecraft:acacia_log", &states);
        for _ in 0..10 {
            assert_eq!(HashUtils::hash_block_permutation("minecraft:acacia_log", &states), first);
        }
    }

    #[test]
    fn state_order_does_not_change_the_hash() {
        let a = states(&[("open_bit", true.into()), ("facing_direction", 3.into()), ("kind", "oak".into())]);
        let b = states(&[("kind", "oak".into()), ("facing_direction", 3.into()), ("open_bit", true.into())]);
        assert_eq!(HashUtils::hash_block_permutation("minecraft:test", &a), HashUtils::hash_block_permutation("minecraft:test", &b));
    }

    #[test]
    fn different_identifiers_and_states_hash_differently() {
        let x = states(&[("pillar_axis", "x".into())]);
        let z = states(&[("pillar_axis", "z".into())]);
        let hashes = [
            HashUtils::hash_block_permutation("minecraft:acacia_log", &x),
            HashUtils::hash_block_permutation("minecraft:acacia_log", &z),
            HashUtils::hash_block_permutation("minecraft:birch_log", &x),
            HashUtils::hash_block_permutation("minecraft:acacia_log", &HashMap::new()),
        ];
        for (i, a) in hashes.iter().enumerate() {
            for b in &hashes[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }

    #[test]
    fn a_bool_state_hashes_as_a_byte_and_an_int_state_as_an_int() {
        let as_bool = HashUtils::hash_block_permutation("minecraft:test", &states(&[("value", true.into())]));
        let as_int = HashUtils::hash_block_permutation("minecraft:test", &states(&[("value", 1.into())]));
        assert_ne!(as_bool, as_int);

        let mut compound = nbtx::Compound::new();
        compound.insert("value".into(), nbtx::Value::Byte(1));
        assert_eq!(HashUtils::hash_block_nbt("minecraft:test", compound), as_bool);
    }

    #[test]
    fn nbt_key_order_does_not_change_the_hash() {
        let mut forward = nbtx::Compound::new();
        forward.insert("a".into(), nbtx::Value::Int(1));
        forward.insert("b".into(), nbtx::Value::Int(2));
        let mut backward = nbtx::Compound::new();
        backward.insert("b".into(), nbtx::Value::Int(2));
        backward.insert("a".into(), nbtx::Value::Int(1));
        assert_eq!(HashUtils::hash_block_nbt("minecraft:test", forward), HashUtils::hash_block_nbt("minecraft:test", backward));
    }

    #[test]
    fn the_unknown_block_is_minus_two() {
        assert_eq!(HashUtils::hash_block_nbt("minecraft:unknown", nbtx::Compound::new()), -2);
    }
}
