pub const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
pub const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

pub fn fnv1a_mix(hash: u64, value: u64) -> u64 {
    (hash ^ value).wrapping_mul(FNV_PRIME)
}

pub fn fnv1a_64(bytes: impl IntoIterator<Item = u8>) -> u64 {
    bytes.into_iter().fold(FNV_OFFSET_BASIS, |hash, byte| fnv1a_mix(hash, u64::from(byte)))
}

pub fn fnv1_64(text: &str) -> u64 {
    text.bytes().fold(FNV_OFFSET_BASIS, |hash, byte| hash.wrapping_mul(FNV_PRIME) ^ u64::from(byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a_matches_the_reference_vectors() {
        assert_eq!(fnv1a_64([]), FNV_OFFSET_BASIS);
        assert_eq!(fnv1a_64(*b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a_64(*b"foobar"), 0x8594_4171_f739_67e8);
    }
}
