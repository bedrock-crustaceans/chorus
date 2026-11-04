use chorus_level::chunk::Chunk;
use chorus_level::generator::dimension::Generator;
use chorus_level::generator::error::PhaseError;
use chorus_level::generator::phase::{Phase, PhaseInputs};
use chorus_level::generator::pos::ChunkPos;

pub struct VoidGenerator {
    pub biome: i32,
    pub air_id: i32,
    pub min_sub_chunk_y: i8,
    pub sub_chunk_count: usize,
}

impl Generator for VoidGenerator {
    type Terminal = VoidPhase;
}

pub struct VoidPhase;

impl Phase<VoidGenerator> for VoidPhase {
    type Output = Chunk;

    fn run(generator: &VoidGenerator, cell: ChunkPos, _inputs: &mut PhaseInputs<VoidGenerator>) -> Result<Self::Output, PhaseError> {
        Ok(Chunk::new(cell.x, cell.z, generator.min_sub_chunk_y, generator.sub_chunk_count, generator.air_id, generator.biome))
    }
}

#[cfg(test)]
mod world_tests {
    use super::*;
    use chorus_level::dimension_type::DimensionType;
    use chorus_level::level::Level;

    fn void() -> VoidGenerator {
        VoidGenerator {
            biome: 1,
            air_id: 0,
            min_sub_chunk_y: DimensionType::Overworld.min_sub_chunk_y(),
            sub_chunk_count: DimensionType::Overworld.sub_chunk_count(),
        }
    }

    #[test]
    fn a_world_has_an_id_of_its_own_but_looks_like_its_dimension_type() {
        let mut level = Level::in_memory("test", 0);
        level.insert_world(1000, DimensionType::Overworld, void());
        let world = level.dimension(1000).expect("the world exists");
        assert_eq!(world.id(), 1000);
        assert_eq!(world.network_id(), DimensionType::Overworld.id());
        assert_eq!(level.network_id(1000), Some(0));
        assert!(!world.is_persistent());
    }

    #[test]
    fn several_worlds_of_one_type_exist_side_by_side_with_the_main_dimension() {
        let mut level = Level::in_memory("test", 0);
        level.insert_dimension(DimensionType::Overworld, void());
        level.insert_world(1000, DimensionType::Overworld, void());
        level.insert_world(1001, DimensionType::Overworld, void());
        assert!(level.has_dimension(0) && level.has_dimension(1000) && level.has_dimension(1001));
        assert!(level.remove_dimension(1000).is_some());
        assert!(!level.has_dimension(1000));
        assert!(level.has_dimension(0) && level.has_dimension(1001));
    }

    #[test]
    fn inserting_a_world_again_replaces_it() {
        let mut level = Level::in_memory("test", 0);
        level.insert_world(1000, DimensionType::Overworld, void());
        level.insert_world(1000, DimensionType::Overworld, void());
        assert_eq!(level.dimensions().filter(|dimension| dimension.id() == 1000).count(), 1);
    }

    #[test]
    fn worlds_that_only_exist_in_memory_never_unload_their_chunks() {
        let mut level = Level::in_memory("test", 0);
        let world = level.insert_world(1000, DimensionType::Overworld, void());
        assert_eq!(world.unload_chunks(|_, _| false), 0);
    }
}
