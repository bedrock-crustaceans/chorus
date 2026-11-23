use std::sync::Arc;

use chorus_block::block_id::{BEDROCK, CHORUS_FLOWER, CHORUS_PLANT, END_GATEWAY, END_STONE, FRAME, IRON_BARS, OBSIDIAN};
use chorus_block::block_registry::BlockRegistry;
use chorus_block::state::block_state::BlockState;
use chorus_block::state::common::{AGE_6, FACING_DIRECTION, INFINIBURN_BIT, ITEM_FRAME_MAP_BIT, ITEM_FRAME_PHOTO_BIT};
use chorus_level::chunk::Chunk;
use chorus_level::generator::dimension::Generator;
use chorus_level::generator::engine::LocateJob;

use super::phases::ChunkPhase;
use super::terrain::{EndNoises, IslandSource};
use super::{AIR, EndChunk, Spike, cities, decorate, spikes, terrain};
use crate::bedrock::overworld::decoration_seed;
use crate::shared::biome::pack_ids::THE_END;
use crate::shared::blocks::{self, BlockId, BlockTable, Blocks};
use crate::shared::chunk::dense;
use crate::shared::decoration_window::{SLOTS, SourceChanges, assemble, window_chunks};
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::{PaletteBlocks, StructureData};
use chorus_level::dimension_definition::{DefinitionError, DimensionDefinition};

const FLOWER_AGE: i32 = 5;
const RULE_DRAWS: usize = 1;
const FRAME_FACINGS: [i32; 4] = [3, 4, 2, 5];

pub struct EndBlocks {
    pub end_stone: BlockId,
    pub obsidian: BlockId,
    pub iron_bars: BlockId,
    pub chorus_plant: BlockId,
    pub chorus_flower: BlockId,
    pub end_gateway: BlockId,
    pub bedrock: BlockId,
    pub frames: [BlockId; 4],
}

pub type EndWrites = SourceChanges;

pub struct EndGenerator {
    pub(super) seed: u32,
    pub(super) noises: terrain::EndNoises,
    pub(super) blocks: Blocks,
    pub(super) ids: EndBlocks,
    pub(super) spikes: [Spike; spikes::COUNT],
    pub data: Option<Arc<StructureData>>,
    pub palette: Option<PaletteBlocks>,
    pub definition: DimensionDefinition,
}

impl EndGenerator {
    pub fn new(seed: i64, registry: &BlockRegistry) -> Self {
        Self::with_definition(seed, registry, DimensionDefinition::END).expect("the vanilla end definition is valid")
    }

    pub fn with_definition(seed: i64, registry: &BlockRegistry, definition: DimensionDefinition) -> Result<Self, DefinitionError> {
        let definition = definition.validate()?;
        let level_seed = seed as u32;
        let data = StructureData::shared();
        let mut table = BlockTable::new(registry);
        let palette = data.as_ref().and_then(|data| data.library.as_ref()).map(|library| PaletteBlocks::register(&mut table, library));
        let ids = EndBlocks {
            end_stone: table.name(END_STONE),
            obsidian: table.name(OBSIDIAN),
            iron_bars: table.name(IRON_BARS),
            chorus_plant: table.name(CHORUS_PLANT),
            chorus_flower: table.get(CHORUS_FLOWER, &[blocks::state(&AGE_6, BlockState::Int(FLOWER_AGE))]),
            end_gateway: table.name(END_GATEWAY),
            bedrock: table.get(BEDROCK, &[blocks::state(&INFINIBURN_BIT, BlockState::Bool(true))]),
            frames: FRAME_FACINGS.map(|facing| {
                table.get(
                    FRAME,
                    &[
                        blocks::state(&FACING_DIRECTION, BlockState::Int(facing)),
                        blocks::state(&ITEM_FRAME_MAP_BIT, BlockState::Bool(false)),
                        blocks::state(&ITEM_FRAME_PHOTO_BIT, BlockState::Bool(false)),
                    ],
                )
            }),
        };
        Ok(Self {
            seed: level_seed,
            noises: terrain::EndNoises::new(level_seed),
            blocks: table.finish(),
            ids,
            spikes: spikes::spikes(level_seed),
            data,
            palette,
            definition,
        })
    }

    pub fn blocks(&self) -> &Blocks {
        &self.blocks
    }

    pub fn seed(&self) -> u32 {
        self.seed
    }

    pub fn spikes(&self) -> [Spike; spikes::COUNT] {
        self.spikes
    }

    pub fn island_height(&self, chunk_x: i32, chunk_z: i32) -> f32 {
        self.noises.island_height(chunk_x, chunk_z, 1, 1)
    }

    pub(super) fn island_sources(&self, cell: (i32, i32)) -> Vec<IslandSource> {
        self.noises.island_sources(cell.0, cell.1)
    }

    pub(super) fn fill_terrain(&self, chunk_x: i32, chunk_z: i32, sources: &[IslandSource]) -> EndChunk {
        let mut chunk = EndChunk::empty(self.definition);
        let end_stone = self.ids.end_stone;
        let cells = self.noises.cells(chunk_x, chunk_z, sources);
        terrain::fill_stone(&cells, |x: usize, y: i32, z: usize| chunk.set(x as i32, y, z as i32, end_stone));
        chunk
    }

    pub fn city_start_cells(&self, chunk: (i32, i32)) -> Vec<(i32, i32)> {
        let spacing = cities::GRID.spacing;
        let regions = |value: i32| (value - cities::REACH).div_euclid(spacing)..=(value + cities::REACH).div_euclid(spacing);
        let mut starts = Vec::new();
        for region_x in regions(chunk.0) {
            for region_z in regions(chunk.1) {
                let start = cities::GRID.candidate(self.seed, (region_x * spacing, region_z * spacing));
                if (start.0 - chunk.0).abs() > cities::REACH || (start.1 - chunk.1).abs() > cities::REACH {
                    continue;
                }
                if cities::beyond_origin(start) {
                    starts.push(start);
                }
            }
        }
        starts
    }

    pub(super) fn city_at(&self, start: (i32, i32)) -> Option<Arc<cities::City>> {
        let (candidate, mut random) = cities::GRID.candidate_random(self.seed, start);
        if candidate != start || !cities::beyond_origin(start) {
            return None;
        }
        let (data, _) = (self.data.as_ref()?, self.palette.as_ref()?);
        let library = data.library.as_ref()?;
        let sources = self.noises.island_sources(start.0, start.1);
        let island = EndNoises::island_height_from(&sources, start.0, start.1, 1, 1);
        if (island.floor() as i16) < 0 {
            return None;
        }
        let rotation = cities::rotation(start);
        let cells = self.noises.cells(start.0, start.1, &sources);
        let height = cities::start_height(&|x, z| terrain::column_top(&cells, x, z), rotation);
        cities::is_start(start, island, height).then(|| Arc::new(cities::generate(&mut random, start, height, library)))
    }

    pub(super) fn loaded_chunk(&self, cell: (i32, i32), terrain: &EndChunk, cities: &[Arc<cities::City>]) -> EndChunk {
        let mut chunk = terrain.clone();
        if let (Some(data), Some(palette)) = (&self.data, &self.palette)
            && let Some(library) = &data.library
        {
            let materials = cities::Materials {
                templates: library,
                palette,
                frames: &self.ids.frames,
                air: AIR,
            };
            for city in cities {
                cities::place(city, cell, &mut chunk, &materials);
            }
        }
        chunk
    }

    pub(super) fn decorate_source(
        &self,
        source: (i32, i32),
        loaded: &[Arc<EndChunk>; SLOTS],
        terrain: &[Arc<EndChunk>; SLOTS],
        islands: &[IslandSource],
        earlier: &[((i32, i32), &EndWrites)],
    ) -> EndWrites {
        let mut chunks = window_chunks(source, earlier.iter().copied(), |slot| (*loaded[slot]).clone());
        let tops: [[i32; 256]; SLOTS] = std::array::from_fn(|slot| terrain[slot].column_tops());
        let mut random = MersenneTwister::new(decoration_seed(self.seed, source.0, source.1));
        random.skip(RULE_DRAWS);
        let mut view = decorate::View::new(&mut chunks, &tops, source);
        let island = |x: i32, z: i32| EndNoises::island_height_from(islands, x, z, 1, 1);
        decorate::decorate(&mut random, source, &mut view, &self.spikes, &self.ids, island);
        view.take_changes()
    }

    pub(super) fn assemble(&self, cell: (i32, i32), loaded: &EndChunk, around: &[Arc<EndWrites>; SLOTS]) -> EndChunk {
        assemble(cell, loaded.clone(), |index| &around[index])
    }
}

pub(super) fn may_start_city(x: i32, z: i32) -> bool {
    let spacing = cities::GRID.spacing;
    let range = spacing - cities::GRID.separation;
    cities::beyond_origin((x, z)) && x.rem_euclid(spacing) < range && z.rem_euclid(spacing) < range
}

impl EndGenerator {
    pub(super) fn dense_chunk(&self, chunk_x: i32, chunk_z: i32, dense: &EndChunk) -> Chunk {
        let cells = dense.cells_vec();
        dense::from_dense((chunk_x, chunk_z), self.definition, &cells, self.blocks(), |_, _| THE_END)
    }
}

impl Generator for EndGenerator {
    type Terminal = ChunkPhase;

    fn locate_job(self: &Arc<Self>, name: &str, from: (i32, i32), radius: i32) -> Option<Box<dyn LocateJob<Self>>> {
        super::locate::job(self.clone(), name, from, radius)
    }
}
