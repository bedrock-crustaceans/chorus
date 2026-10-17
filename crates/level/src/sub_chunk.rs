use crate::palette::Palette;
use bedrock::protocol::ProtoCodec;

#[derive(Clone)]
pub struct SubChunk {
    blocks: Vec<Palette>,
    biomes: Palette,
    air_id: i32,
}

impl SubChunk {
    pub fn new(air_id: i32, biome: i32) -> Self {
        Self {
            blocks: vec![Palette::new(air_id), Palette::new(air_id)],
            biomes: Palette::new(biome),
            air_id,
        }
    }

    pub fn from_blocks(blocks: &[i32; 4096], air_id: i32, biome: i32) -> Self {
        Self {
            blocks: vec![Palette::from_blocks(blocks), Palette::new(air_id)],
            biomes: Palette::new(biome),
            air_id,
        }
    }

    pub fn from_layers(blocks: &[i32; 4096], extra: Option<&[i32; 4096]>, biomes: &[i32; 4096], air_id: i32) -> Self {
        Self {
            blocks: vec![Palette::from_blocks(blocks), extra.map_or_else(|| Palette::new(air_id), Palette::from_blocks)],
            biomes: Palette::from_blocks(biomes),
            air_id,
        }
    }

    pub fn set_biome(&mut self, x: u8, y: u8, z: u8, biome: i32) {
        self.biomes.set(Self::index(x, y, z), biome);
    }

    pub fn get_biome(&self, x: u8, y: u8, z: u8) -> i32 {
        self.biomes.get(Self::index(x, y, z))
    }

    pub fn get(&self, x: u8, y: u8, z: u8, layer: usize) -> i32 {
        debug_assert!(x < 16 && y < 16 && z < 16 && layer < 2);
        self.blocks[layer].get(Self::index(x, y, z))
    }

    pub fn set(&mut self, x: u8, y: u8, z: u8, layer: usize, block_id: i32) -> bool {
        debug_assert!(x < 16 && y < 16 && z < 16 && layer < 2);
        let index = Self::index(x, y, z);

        let old = self.blocks[layer].get(index);
        if old == block_id {
            return false;
        }

        self.blocks[layer].set(index, block_id);
        true
    }

    pub(crate) fn layers(&self) -> &[Palette] {
        &self.blocks
    }

    pub(crate) fn biomes(&self) -> &Palette {
        &self.biomes
    }

    pub fn is_all_air(&self) -> bool {
        self.blocks.iter().all(|p| p.is_all(self.air_id))
    }

    pub fn serialize_network(&self, sub_chunk_y: i8) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.push(9u8); // version = Limitless
        buf.push(self.blocks.len() as u8);
        buf.push(sub_chunk_y as u8);
        for palette in &self.blocks {
            palette.serialize(&mut buf).unwrap();
        }
        buf
    }

    pub fn serialize_biomes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.biomes.serialize(&mut buf).unwrap();
        buf
    }

    pub fn index(x: u8, y: u8, z: u8) -> usize {
        ((x as usize) << 8) | ((z as usize) << 4) | (y as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{Lcg, decode_network};

    const AIR: i32 = -7;
    const PLAINS: i32 = 1;

    fn random_blocks(seed: u64, distinct: u32) -> [i32; 4096] {
        let mut rng = Lcg::new(seed);
        let mut blocks = [AIR; 4096];
        for block in blocks.iter_mut() {
            *block = rng.below(distinct) as i32 + 10;
        }
        blocks
    }

    #[test]
    fn index_packs_x_then_z_then_y() {
        assert_eq!(SubChunk::index(0, 0, 0), 0);
        assert_eq!(SubChunk::index(0, 1, 0), 1);
        assert_eq!(SubChunk::index(0, 0, 1), 16);
        assert_eq!(SubChunk::index(1, 0, 0), 256);
        assert_eq!(SubChunk::index(15, 15, 15), 4095);
    }

    #[test]
    fn a_new_sub_chunk_is_all_air_with_the_default_biome() {
        let sub_chunk = SubChunk::new(AIR, PLAINS);
        assert!(sub_chunk.is_all_air());
        assert_eq!(sub_chunk.get(3, 4, 5, 0), AIR);
        assert_eq!(sub_chunk.get(3, 4, 5, 1), AIR);
        assert_eq!(sub_chunk.get_biome(15, 15, 15), PLAINS);
    }

    #[test]
    fn set_reports_whether_anything_changed() {
        let mut sub_chunk = SubChunk::new(AIR, PLAINS);
        assert!(!sub_chunk.set(1, 2, 3, 0, AIR));
        assert!(sub_chunk.set(1, 2, 3, 0, 5));
        assert!(!sub_chunk.set(1, 2, 3, 0, 5));
        assert_eq!(sub_chunk.get(1, 2, 3, 0), 5);
    }

    #[test]
    fn layers_are_independent() {
        let mut sub_chunk = SubChunk::new(AIR, PLAINS);
        sub_chunk.set(0, 0, 0, 1, 8);
        assert_eq!(sub_chunk.get(0, 0, 0, 0), AIR);
        assert_eq!(sub_chunk.get(0, 0, 0, 1), 8);
    }

    #[test]
    fn is_all_air_follows_edits_and_restores_on_a_built_sub_chunk() {
        let mut blocks = [AIR; 4096];
        blocks[0] = 3;
        let mut sub_chunk = SubChunk::from_blocks(&blocks, AIR, PLAINS);
        assert!(!sub_chunk.is_all_air());
        sub_chunk.set(0, 0, 0, 0, AIR);
        assert!(sub_chunk.is_all_air());
        sub_chunk.set(15, 15, 15, 0, 3);
        assert!(!sub_chunk.is_all_air());
    }

    #[test]
    fn placing_a_block_in_an_empty_sub_chunk_makes_it_not_all_air() {
        let mut sub_chunk = SubChunk::new(AIR, PLAINS);
        sub_chunk.set(15, 15, 15, 0, 3);
        assert!(!sub_chunk.is_all_air());

        let mut layered = SubChunk::new(AIR, PLAINS);
        layered.set(0, 0, 0, 1, 8);
        assert!(!layered.is_all_air());
    }

    #[test]
    fn a_sub_chunk_built_from_blocks_of_one_value_is_air_only_when_that_value_is_air() {
        assert!(SubChunk::from_blocks(&[AIR; 4096], AIR, PLAINS).is_all_air());
        assert!(!SubChunk::from_blocks(&[3; 4096], AIR, PLAINS).is_all_air());
    }

    #[test]
    fn from_blocks_and_from_layers_expose_every_cell() {
        let blocks = random_blocks(1, 12);
        let extra = random_blocks(2, 3);
        let biomes = random_blocks(3, 5);
        let sub_chunk = SubChunk::from_layers(&blocks, Some(&extra), &biomes, AIR);
        let single = SubChunk::from_blocks(&blocks, AIR, PLAINS);

        for x in 0..16u8 {
            for y in 0..16u8 {
                for z in 0..16u8 {
                    let index = SubChunk::index(x, y, z);
                    assert_eq!(sub_chunk.get(x, y, z, 0), blocks[index]);
                    assert_eq!(sub_chunk.get(x, y, z, 1), extra[index]);
                    assert_eq!(sub_chunk.get_biome(x, y, z), biomes[index]);
                    assert_eq!(single.get(x, y, z, 0), blocks[index]);
                    assert_eq!(single.get(x, y, z, 1), AIR);
                }
            }
        }
    }

    #[test]
    fn from_layers_without_an_extra_layer_leaves_it_as_air() {
        let sub_chunk = SubChunk::from_layers(&random_blocks(4, 4), None, &[PLAINS; 4096], AIR);
        assert_eq!(sub_chunk.get(0, 0, 0, 1), AIR);
        assert_eq!(sub_chunk.layers()[1].count(AIR), 4096);
    }

    #[test]
    fn biomes_are_set_per_cell() {
        let mut sub_chunk = SubChunk::new(AIR, PLAINS);
        sub_chunk.set_biome(2, 9, 14, 40);
        assert_eq!(sub_chunk.get_biome(2, 9, 14), 40);
        assert_eq!(sub_chunk.get_biome(2, 9, 13), PLAINS);
        assert_eq!(sub_chunk.get_biome(3, 9, 14), PLAINS);
    }

    #[test]
    fn network_serialisation_starts_with_version_layer_count_and_y() {
        let sub_chunk = SubChunk::new(AIR, PLAINS);
        let bytes = sub_chunk.serialize_network(-4);
        assert_eq!(&bytes[..3], &[9, 2, 252]);
        assert_eq!(bytes[3], 0x01);
    }

    #[test]
    fn network_serialisation_decodes_to_the_same_blocks() {
        let blocks = random_blocks(5, 30);
        let sub_chunk = SubChunk::from_blocks(&blocks, AIR, PLAINS);
        let bytes = sub_chunk.serialize_network(2);

        let mut reader = &bytes[3..];
        let first = decode_network(&mut reader);
        assert_eq!(first.as_slice(), blocks.as_slice());
        let second = decode_network(&mut reader);
        assert_eq!(second, vec![AIR; 4096]);
        assert!(reader.is_empty());
    }

    #[test]
    fn biome_serialisation_matches_the_biome_palette() {
        let biomes = random_blocks(6, 6);
        let sub_chunk = SubChunk::from_layers(&[AIR; 4096], None, &biomes, AIR);
        let bytes = sub_chunk.serialize_biomes();
        let mut reader = bytes.as_slice();
        let decoded = decode_network(&mut reader);
        assert_eq!(decoded.as_slice(), biomes.as_slice());
        assert!(reader.is_empty());
    }
}
