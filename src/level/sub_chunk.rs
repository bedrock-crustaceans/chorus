use std::collections::HashMap;

const VALID_BITS: [u8; 8] = [1, 2, 3, 4, 5, 6, 8, 16];

pub(crate) fn write_zigzag_var_i32(buf: &mut Vec<u8>, value: i32) {
    let mut v = ((value << 1) ^ (value >> 31)) as u32;
    loop {
        if v & !0x7F == 0 {
            buf.push(v as u8);
            return;
        }
        buf.push(((v & 0x7F) | 0x80) as u8);
        v >>= 7;
    }
}

fn sub_chunk_index(x: u8, y: u8, z: u8) -> usize {
    ((x as usize) << 8) | ((z as usize) << 4) | (y as usize)
}

fn encode_layer(blocks: &[u32; 4096]) -> Vec<u8> {
    let mut palette: Vec<u32> = Vec::new();
    let mut index_map: HashMap<u32, u16> = HashMap::new();
    let mut indices = [0u16; 4096];

    for (i, &block) in blocks.iter().enumerate() {
        let idx = if let Some(&idx) = index_map.get(&block) {
            idx
        } else {
            let idx = palette.len() as u16;
            index_map.insert(block, idx);
            palette.push(block);
            idx
        };
        indices[i] = idx;
    }

    let mut buf = Vec::new();

    if palette.len() == 1 {
        buf.push(0x01u8);
        write_zigzag_var_i32(&mut buf, palette[0] as i32);
        return buf;
    }

    let bits = VALID_BITS
        .iter()
        .copied()
        .find(|&b| (1usize << b) >= palette.len())
        .unwrap_or(16);

    buf.push((bits << 1) | 1);

    let epw = (32 / bits) as usize;
    let num_words = (4096 + epw - 1) / epw;
    let mut words = vec![0u32; num_words];
    for (i, &idx) in indices.iter().enumerate() {
        let wi = i / epw;
        let bit_off = (i % epw) * bits as usize;
        words[wi] |= (idx as u32) << bit_off;
    }
    for word in &words {
        buf.extend_from_slice(&word.to_le_bytes());
    }

    write_zigzag_var_i32(&mut buf, palette.len() as i32);
    for &id in &palette {
        write_zigzag_var_i32(&mut buf, id as i32);
    }

    buf
}

pub struct SubChunk {
    // blocks[layer][index], index = (x<<8)|(z<<4)|y
    blocks: [[u32; 4096]; 2],
    pub biome: u32,
    air_id: u32,
}

impl SubChunk {
    pub fn new(air_id: u32, biome: u32) -> Self {
        Self {
            blocks: [[air_id; 4096]; 2],
            biome,
            air_id,
        }
    }

    pub fn get(&self, x: u8, y: u8, z: u8, layer: usize) -> u32 {
        debug_assert!(x < 16 && y < 16 && z < 16 && layer < 2);
        self.blocks[layer][sub_chunk_index(x, y, z)]
    }

    pub fn set(&mut self, x: u8, y: u8, z: u8, layer: usize, block_id: u32) {
        debug_assert!(x < 16 && y < 16 && z < 16 && layer < 2);
        self.blocks[layer][sub_chunk_index(x, y, z)] = block_id;
    }

    pub fn is_all_air(&self) -> bool {
        self.blocks[0].iter().all(|&b| b == self.air_id)
            && self.blocks[1].iter().all(|&b| b == self.air_id)
    }

    pub fn serialize_network(&self, sub_chunk_y: i8) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.push(9u8);  // version = Limitless
        buf.push(2u8);  // layer count
        buf.push(sub_chunk_y as u8);
        buf.extend(encode_layer(&self.blocks[0]));
        buf.extend(encode_layer(&self.blocks[1]));
        buf
    }
}
