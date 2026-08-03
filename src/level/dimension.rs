use crate::level::chunk::Chunk;
use crate::level::generator::WorldGenerator;
use crate::registry::block_registry::BlockRegistry;
use bevy_tasks::ComputeTaskPool;
use std::collections::HashMap;

pub struct Dimension {
    pub id: i32,
    pub min_sub_chunk_y: i8,
    pub max_sub_chunk_y: i8,
    chunks: HashMap<(i32, i32), Chunk>,
    generator: Box<dyn WorldGenerator + Send + Sync>,
}

impl Dimension {
    pub fn new<G>(id: i32, min_sub_chunk_y: i8, max_sub_chunk_y: i8, generator: G) -> Self
    where
        G: WorldGenerator + Send + Sync + 'static,
    {
        Self {
            id,
            min_sub_chunk_y,
            max_sub_chunk_y,
            chunks: HashMap::new(),
            generator: Box::new(generator),
        }
    }

    pub fn sub_chunk_count(&self) -> usize {
        (self.max_sub_chunk_y as i32 - self.min_sub_chunk_y as i32 + 1) as usize
    }

    pub fn get_or_generate_chunk(&mut self, registry: &BlockRegistry, x: i32, z: i32) -> &Chunk {
        if !self.chunks.contains_key(&(x, z)) {
            let chunk = self.generate_chunk(registry, x, z);

            self.chunks.insert((x, z), chunk);
        }
        self.chunks.get(&(x, z)).unwrap()
    }

    /// Generates every position that is not loaded yet, spreading the work over the compute task
    /// pool instead of doing it one by one on the caller's thread.
    pub fn generate_chunks(&mut self, registry: &BlockRegistry, positions: &[(i32, i32)]) {
        let missing: Vec<(i32, i32)> = positions.iter().copied().filter(|position| !self.chunks.contains_key(position)).collect();
        if missing.is_empty() {
            return;
        }

        let dimension = &*self;
        let generated = ComputeTaskPool::get().scope(|scope| {
            for (x, z) in missing {
                scope.spawn(async move { ((x, z), dimension.generate_chunk(registry, x, z)) });
            }
        });

        self.chunks.extend(generated);
    }

    fn generate_chunk(&self, registry: &BlockRegistry, x: i32, z: i32) -> Chunk {
        let mut chunk = Self::empty_chunk(registry, x, z, self.min_sub_chunk_y, self.max_sub_chunk_y, 0);

        self.generator.generate(registry, x, z, &mut chunk);

        chunk
    }

    pub fn get_chunk(&self, x: i32, z: i32) -> Option<&Chunk> {
        self.chunks.get(&(x, z))
    }

    pub fn get_chunk_mut(&mut self, x: i32, z: i32) -> Option<&mut Chunk> {
        self.chunks.get_mut(&(x, z))
    }

    pub fn get_block(&self, x: i32, y: i32, z: i32, layer: usize) -> Option<i32> {
        self.chunks.get(&(x >> 4, z >> 4)).and_then(|c| c.get_block((x & 0xF) as u8, y, (z & 0xF) as u8, layer))
    }

    pub fn set_block(&mut self, x: i32, y: i32, z: i32, layer: usize, block_id: i32) -> bool {
        match self.chunks.get_mut(&(x >> 4, z >> 4)) {
            Some(chunk) => chunk.set_block((x & 0xF) as u8, y, (z & 0xF) as u8, layer, block_id),
            None => false,
        }
    }

    fn empty_chunk(registry: &BlockRegistry, x: i32, z: i32, min_sub_chunk_y: i8, max_sub_chunk_y: i8, biome: i32) -> Chunk {
        let air_id = registry.get_block_id("minecraft:air").unwrap_or(0);
        let count = (max_sub_chunk_y as i32 - min_sub_chunk_y as i32 + 1) as usize;

        Chunk::new(x, z, min_sub_chunk_y, count, air_id, biome)
    }
}
