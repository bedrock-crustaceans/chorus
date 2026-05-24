use std::collections::HashMap;
use varint_rs::VarintWriter;

const VALID_BITS: [u8; 8] = [1, 2, 3, 4, 5, 6, 8, 16];

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
        buf.write_u32_varint(palette[0]).unwrap();
        return buf;
    }

    let bits = VALID_BITS.iter().copied().find(|&b| (1usize << b) >= palette.len()).unwrap_or(16);

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

    buf.write_u32_varint(palette.len() as u32).unwrap();
    for &id in &palette {
        buf.write_u32_varint(id).unwrap();
    }

    buf
}

pub struct SubChunk {
    // blocks[layer][index], index = (x<<8)|(z<<4)|y
    blocks: [[u32; 4096]; 2],
    biomes: [u32; 4096],
    air_id: u32,
}

impl SubChunk {
    pub fn new(air_id: u32, biome: u32) -> Self {
        Self {
            blocks: [[air_id; 4096]; 2],
            biomes: [biome; 4096],
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
        self.blocks[0].iter().all(|&b| b == self.air_id) && self.blocks[1].iter().all(|&b| b == self.air_id)
    }

    pub fn serialize_network(&self, sub_chunk_y: i8) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.push(9u8); // version = Limitless
        buf.push(2u8); // layer count
        buf.push(sub_chunk_y as u8);
        buf.extend(encode_layer(&self.blocks[0]));
        buf.extend(encode_layer(&self.blocks[1]));
        buf
    }

    pub fn serialize_biomes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend(encode_layer(&self.biomes));
        buf
    }
}
