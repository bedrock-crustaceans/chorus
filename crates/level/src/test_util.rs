pub struct Lcg(u64);

impl Lcg {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }

    pub fn below(&mut self, bound: u32) -> u32 {
        self.next() % bound
    }
}

fn zigzag(value: u32) -> i32 {
    ((value >> 1) as i32) ^ -((value & 1) as i32)
}

fn read_varint(reader: &mut &[u8]) -> u32 {
    let mut value = 0u32;
    let mut shift = 0;
    loop {
        let byte = reader[0];
        *reader = &reader[1..];
        value |= ((byte & 0x7f) as u32) << shift;
        if byte & 0x80 == 0 {
            return value;
        }
        shift += 7;
    }
}

fn read_u32_le(reader: &mut &[u8]) -> u32 {
    let value = u32::from_le_bytes(reader[..4].try_into().unwrap());
    *reader = &reader[4..];
    value
}

fn read_words(reader: &mut &[u8], bits: usize) -> Vec<u16> {
    let mut indices = vec![0u16; 4096];
    if bits == 0 {
        return indices;
    }
    let per_word = 32 / bits;
    let mask = (1u32 << bits) - 1;
    for word in 0..4096usize.div_ceil(per_word) {
        let packed = read_u32_le(reader);
        for slot in 0..per_word {
            let index = word * per_word + slot;
            if index < 4096 {
                indices[index] = ((packed >> (slot * bits)) & mask) as u16;
            }
        }
    }
    indices
}

pub fn decode_disk(reader: &mut &[u8]) -> Vec<i32> {
    let header = reader[0];
    *reader = &reader[1..];
    assert_eq!(header & 1, 0, "disk palettes are not runtime ids");
    let indices = read_words(reader, (header >> 1) as usize);
    let len = read_u32_le(reader) as usize;
    let palette: Vec<i32> = (0..len).map(|_| read_u32_le(reader) as i32).collect();
    indices.iter().map(|&i| palette[i as usize]).collect()
}

pub fn decode_network(reader: &mut &[u8]) -> Vec<i32> {
    let header = reader[0];
    *reader = &reader[1..];
    assert_eq!(header & 1, 1, "network palettes use runtime ids");
    if header >> 1 == 0 {
        return vec![zigzag(read_varint(reader)); 4096];
    }
    let indices = read_words(reader, (header >> 1) as usize);
    let len = zigzag(read_varint(reader)) as usize;
    let palette: Vec<i32> = (0..len).map(|_| zigzag(read_varint(reader))).collect();
    indices.iter().map(|&i| palette[i as usize]).collect()
}

pub fn decode_disk_biomes(reader: &mut &[u8]) -> Vec<i32> {
    if reader[0] == 0 {
        *reader = &reader[1..];
        return vec![read_u32_le(reader) as i32; 4096];
    }
    decode_disk(reader)
}
