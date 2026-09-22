#[allow(non_snake_case)]
pub mod HashUtils {
    use serde::ser::SerializeMap;
    use serde::{Serialize, Serializer};
    use std::collections::{BTreeMap, HashMap};

    pub struct SortedCompound<'a> {
        compound: &'a HashMap<String, nbtx::Value>,
    }

    impl<'a> SortedCompound<'a> {
        pub fn new(compound: &'a HashMap<String, nbtx::Value>) -> Self {
            Self { compound }
        }
    }

    impl<'a> Serialize for SortedCompound<'a> {
        fn serialize<S>(&self, ser: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let map: BTreeMap<&String, &nbtx::Value> = self.compound.iter().collect();

            let mut map_ser = ser.serialize_map(Some(map.len()))?;
            for (k, v) in &map {
                match v {
                    nbtx::Value::Compound(map) => {
                        let v = &SortedCompound::new(map);
                        map_ser.serialize_entry(k, v)?
                    }
                    _ => {
                        map_ser.serialize_entry(k, v)?;
                    }
                }
            }
            map_ser.end()
        }
    }

    pub fn hash_nbt(compound: &HashMap<String, nbtx::Value>) -> i32 {
        let sorted = SortedCompound::new(compound);

        fnv1a_32::hash(nbtx::to_le_bytes(&sorted).unwrap().as_slice()) as i32
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
