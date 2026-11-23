use std::sync::Arc;

use chorus_block::block_id::{
    ANCIENT_DEBRIS, BASALT, BEDROCK, BLACKSTONE, BROWN_MUSHROOM, CHEST, COARSE_DIRT, CRIMSON_FUNGUS, CRIMSON_NYLIUM, CRIMSON_ROOTS, CRIMSON_STEM, DIRT, DRIED_GHAST, FIRE, GLOWSTONE, GRASS_BLOCK,
    LAVA, MAGMA, MOB_SPAWNER, MYCELIUM, NETHER_BRICK, NETHER_BRICK_FENCE, NETHER_BRICK_STAIRS, NETHER_SPROUTS, NETHER_WART, NETHER_WART_BLOCK, NETHERRACK, PODZOL, RED_MUSHROOM, SHROOMLIGHT,
    SOUL_FIRE, SOUL_SAND, SOUL_SOIL, STONE, TWISTING_VINES, WARPED_FUNGUS, WARPED_NYLIUM, WARPED_ROOTS, WARPED_STEM, WARPED_WART_BLOCK, WATER, WEEPING_VINES,
};
use chorus_block::block_registry::BlockRegistry;
use chorus_block::state::block_state::{BlockState, BlockStateDefinition};
use chorus_block::state::common::{
    DRAG_DOWN, LIQUID_DEPTH, MINECRAFT_CARDINAL_DIRECTION, MINECRAFT_CONNECTION_EAST, MINECRAFT_CONNECTION_NORTH, MINECRAFT_CONNECTION_SOUTH, MINECRAFT_CONNECTION_WEST, MINECRAFT_CORNER,
    TWISTING_VINES_AGE, UPSIDE_DOWN_BIT, WEEPING_VINES_AGE, WEIRDO_DIRECTION,
};

use crate::shared::blocks::{self, AIR, BlockId, BlockTable, Blocks};
use crate::shared::structure::{PaletteBlocks, StructureData};
use chorus_level::chunk::Chunk;
use chorus_level::fluid::{CellRules, DEPTHS, FluidBlocks, FluidRules, Liquid, LiquidMotion};
use chorus_level::generator::dimension::Generator;
use chorus_level::generator::engine::LocateJob;

use super::phases::ChunkPhase;
use super::{NetherChunk, biome, carver, decorate, feature, rules, structure, surface, terrain};
use crate::shared::chunk::dense;
use chorus_level::dimension_definition::{DefinitionError, DimensionDefinition};

const VINE_AGES: usize = 26;
const GHAST_FACINGS: usize = 4;
const LAVA_LEVELS: usize = 16;
const GHAST_DIRECTIONS: [&str; GHAST_FACINGS] = chorus_block::state::common::CARDINAL_DIRECTIONS;
const CORNER_NAMES: [&str; 5] = ["none", "inner_left", "inner_right", "outer_left", "outer_right"];

pub struct NetherBlocks {
    pub stone: BlockId,
    pub water: BlockId,
    pub bedrock: BlockId,
    pub netherrack: BlockId,
    pub soul_sand: BlockId,
    pub soul_soil: BlockId,
    pub crimson_nylium: BlockId,
    pub warped_nylium: BlockId,
    pub nether_wart: BlockId,
    pub nether_wart_plant: BlockId,
    pub basalt: BlockId,
    pub blackstone: BlockId,
    pub nether_brick: BlockId,
    pub nether_brick_fence: BlockId,
    pub nether_brick_fence_sides: [BlockId; 16],
    pub nether_brick_stairs: [BlockId; 4],
    pub nether_brick_stairs_corners: [[BlockId; 5]; 4],
    pub chest: BlockId,
    pub mob_spawner: BlockId,
    pub lava: BlockId,
    pub dirt: BlockId,
    pub coarse_dirt: BlockId,
    pub grass_block: BlockId,
    pub air: BlockId,
    pub fire: BlockId,
    pub soul_fire: BlockId,
    pub glowstone: BlockId,
    pub ancient_debris: BlockId,
    pub brown_mushroom: BlockId,
    pub red_mushroom: BlockId,
    pub magma: BlockId,
    pub warped_wart: BlockId,
    pub shroomlight: BlockId,
    pub crimson_stem: BlockId,
    pub warped_stem: BlockId,
    pub mycelium: BlockId,
    pub podzol: BlockId,
    pub crimson_roots: BlockId,
    pub warped_roots: BlockId,
    pub nether_sprouts: BlockId,
    pub crimson_fungus: BlockId,
    pub warped_fungus: BlockId,
    pub weeping_vines: [BlockId; VINE_AGES],
    pub twisting_vines: [BlockId; VINE_AGES],
    pub dried_ghasts: [BlockId; GHAST_FACINGS],
    pub lava_levels: [BlockId; LAVA_LEVELS],
    pub obsidian: BlockId,
    pub cobblestone: BlockId,
    pub bubble_column: BlockId,
}

fn vine_ages(table: &mut BlockTable, name: &'static str, age: &'static BlockStateDefinition) -> [BlockId; VINE_AGES] {
    std::array::from_fn(|index| table.get(name, &[blocks::state(age, BlockState::Int(index as i32))]))
}

fn liquid_levels(table: &mut BlockTable, source: BlockId) -> [BlockId; LAVA_LEVELS] {
    std::array::from_fn(|depth| {
        if depth == 0 {
            source
        } else {
            table.get(LAVA, &[blocks::state(&LIQUID_DEPTH, BlockState::Int(depth as i32))])
        }
    })
}

pub struct NetherGenerator {
    pub(super) seed: u32,
    pub(super) noises: terrain::NetherNoises,
    pub(super) blocks: Blocks,
    pub ids: NetherBlocks,
    pub(super) biomes: biome::NetherBiomes,
    pub(super) surface: surface::NetherSurface,
    pub(super) carver: carver::NetherCarver,
    pub(super) ores: decorate::LegacyOres,
    pub(super) rules: rules::NetherRules,
    pub(super) structure: structure::NetherStructures,
    pub(super) data: Option<Arc<StructureData>>,
    pub palette: Option<PaletteBlocks>,
    pub definition: DimensionDefinition,
    fluid_rules: std::sync::OnceLock<FluidRules>,
}

impl NetherGenerator {
    pub fn new(seed: i64, registry: &BlockRegistry) -> Self {
        Self::with_definition(seed, registry, DimensionDefinition::NETHER).expect("the vanilla nether definition is valid")
    }

    pub fn with_definition(seed: i64, registry: &BlockRegistry, definition: DimensionDefinition) -> Result<Self, DefinitionError> {
        let definition = definition.validate()?;
        let data = StructureData::shared();
        let mut table = BlockTable::new(registry);
        let palette = data.as_ref().and_then(|data| data.library.as_ref()).map(|library| PaletteBlocks::register(&mut table, library));
        let lava = table.name(LAVA);
        let ids = NetherBlocks {
            stone: table.name(STONE),
            water: table.name(WATER),
            bedrock: table.name(BEDROCK),
            netherrack: table.name(NETHERRACK),
            soul_sand: table.name(SOUL_SAND),
            soul_soil: table.name(SOUL_SOIL),
            crimson_nylium: table.name(CRIMSON_NYLIUM),
            warped_nylium: table.name(WARPED_NYLIUM),
            nether_wart: table.name(NETHER_WART_BLOCK),
            nether_wart_plant: table.name(NETHER_WART),
            basalt: table.name(BASALT),
            blackstone: table.name(BLACKSTONE),
            nether_brick: table.name(NETHER_BRICK),
            nether_brick_fence: table.name(NETHER_BRICK_FENCE),
            nether_brick_fence_sides: std::array::from_fn(|code| {
                let code = code as u16;
                let side = |direction: &BlockStateDefinition, bit: u16| blocks::state(direction, blocks::flag(code & bit != 0));
                table.get(
                    NETHER_BRICK_FENCE,
                    &[
                        side(&MINECRAFT_CONNECTION_NORTH, 1),
                        side(&MINECRAFT_CONNECTION_EAST, 2),
                        side(&MINECRAFT_CONNECTION_SOUTH, 4),
                        side(&MINECRAFT_CONNECTION_WEST, 8),
                    ],
                )
            }),
            nether_brick_stairs: std::array::from_fn(|facing| table.get(NETHER_BRICK_STAIRS, &[blocks::state(&WEIRDO_DIRECTION, BlockState::Int(facing as i32))])),
            nether_brick_stairs_corners: std::array::from_fn(|facing| {
                std::array::from_fn(|corner| {
                    table.get(
                        NETHER_BRICK_STAIRS,
                        &[
                            blocks::state(&WEIRDO_DIRECTION, blocks::int(facing as i32)),
                            blocks::state(&UPSIDE_DOWN_BIT, blocks::flag(false)),
                            blocks::state(&MINECRAFT_CORNER, blocks::text(CORNER_NAMES[corner])),
                        ],
                    )
                })
            }),
            chest: table.get(CHEST, &[blocks::state(&MINECRAFT_CARDINAL_DIRECTION, blocks::text("west"))]),
            mob_spawner: table.name(MOB_SPAWNER),
            lava,
            dirt: table.name(DIRT),
            coarse_dirt: table.name(COARSE_DIRT),
            grass_block: table.name(GRASS_BLOCK),
            air: AIR,
            fire: table.name(FIRE),
            soul_fire: table.name(SOUL_FIRE),
            glowstone: table.name(GLOWSTONE),
            ancient_debris: table.name(ANCIENT_DEBRIS),
            brown_mushroom: table.name(BROWN_MUSHROOM),
            red_mushroom: table.name(RED_MUSHROOM),
            magma: table.name(MAGMA),
            warped_wart: table.name(WARPED_WART_BLOCK),
            shroomlight: table.name(SHROOMLIGHT),
            crimson_stem: table.name(CRIMSON_STEM),
            warped_stem: table.name(WARPED_STEM),
            mycelium: table.name(MYCELIUM),
            podzol: table.name(PODZOL),
            crimson_roots: table.name(CRIMSON_ROOTS),
            warped_roots: table.name(WARPED_ROOTS),
            nether_sprouts: table.name(NETHER_SPROUTS),
            crimson_fungus: table.name(CRIMSON_FUNGUS),
            warped_fungus: table.name(WARPED_FUNGUS),
            weeping_vines: vine_ages(&mut table, WEEPING_VINES, &WEEPING_VINES_AGE),
            twisting_vines: vine_ages(&mut table, TWISTING_VINES, &TWISTING_VINES_AGE),
            dried_ghasts: GHAST_DIRECTIONS.map(|direction| table.get(DRIED_GHAST, &[blocks::state(&MINECRAFT_CARDINAL_DIRECTION, blocks::text(direction))])),
            lava_levels: liquid_levels(&mut table, lava),
            obsidian: table.name(chorus_block::block_id::OBSIDIAN),
            cobblestone: table.name(chorus_block::block_id::COBBLESTONE),
            bubble_column: table.get_flooded(chorus_block::block_id::BUBBLE_COLUMN, &[blocks::state(&DRAG_DOWN, blocks::flag(true))]),
        };
        let ores = decorate::LegacyOres::new(&mut table);
        let rules = rules::NetherRules::new(&mut table);
        let surface = surface::NetherSurface::new(seed, &mut table);
        Ok(Self {
            seed: seed as u32,
            noises: terrain::NetherNoises::new(seed as u32),
            blocks: table.finish(),
            ids,
            biomes: biome::NetherBiomes::new(seed),
            surface,
            carver: carver::NetherCarver::new(seed),
            ores,
            rules,
            structure: structure::NetherStructures::new(seed, data.clone(), definition),
            data,
            palette,
            definition,
            fluid_rules: std::sync::OnceLock::new(),
        })
    }

    pub fn blocks(&self) -> &Blocks {
        &self.blocks
    }

    pub(crate) fn active_fluid_ticks(&self, chunk: &mut chorus_level::chunk::Chunk) {
        let rules = self.fluid_rules.get_or_init(|| self.fluid_rules());
        let updates = chunk.take_fluid_updates();
        chunk.set_fluid_updates(chorus_level::fluid::retain_active(chunk, rules, updates));
    }

    pub fn fluid_rules(&self) -> FluidRules {
        let blocks = &self.blocks;
        let mut cells = rustc_hash::FxHashMap::default();
        for (runtime, id) in blocks.canonical_runtime_ids() {
            let name = blocks.name_of(id);
            let lava = name == LAVA;
            let water = name == WATER;
            let solid = decorate::solid(blocks, id);
            let depth = blocks.int(id, &LIQUID_DEPTH).unwrap_or(0).clamp(0, 15) as u8;
            cells.insert(
                runtime,
                CellRules {
                    liquid: (lava || water).then_some(Liquid { lava, depth }),
                    waterlogged: false,
                    stops_water: solid,
                    stops_lava: solid,
                    solid,
                    water_source: water && depth == 0,
                    magma: false,
                },
            );
        }
        FluidRules::new(
            FluidBlocks {
                air: blocks.runtime(AIR),
                stone: blocks.runtime(self.ids.stone),
                obsidian: blocks.runtime(self.ids.obsidian),
                cobblestone: blocks.runtime(self.ids.cobblestone),
                bubble_column: blocks.runtime(self.ids.bubble_column),
                water: [blocks.runtime(self.ids.water); DEPTHS],
                lava: self.ids.lava_levels.map(|block| blocks.runtime(block)),
                water_motion: LiquidMotion::WATER,
                lava_motion: LiquidMotion::FAST_LAVA,
            },
            cells,
        )
    }

    pub fn seed(&self) -> u32 {
        self.seed
    }

    pub fn biome_at(&self, x: i32, z: i32) -> i32 {
        self.biomes.biome(x, z)
    }

    pub fn quart_biome(&self, x: i32, z: i32) -> i32 {
        self.biomes.quart_biome(x, z)
    }

    pub(super) fn materials(&self) -> Option<feature::Materials<'_>> {
        let data = self.data.as_ref()?;
        let palette = self.palette.as_ref()?;
        Some(feature::Materials {
            templates: data.library.as_ref()?,
            palette,
            air: self.ids.air,
            ground: self.ids.soul_sand,
            ghast_ground: self.ids.soul_soil,
        })
    }
}

impl NetherGenerator {
    pub(super) fn dense_chunk(&self, chunk_x: i32, chunk_z: i32, dense: &NetherChunk, biomes: &[i32; 256]) -> Chunk {
        let cells = dense.cells_vec();
        dense::from_dense((chunk_x, chunk_z), self.definition, &cells, self.blocks(), |x, z| biomes[(x * 16 + z) as usize])
    }
}

impl Generator for NetherGenerator {
    type Terminal = ChunkPhase;

    fn fluid_rules(&self) -> Option<chorus_level::fluid::FluidRules> {
        Some(NetherGenerator::fluid_rules(self))
    }

    fn locate_job(self: &Arc<Self>, name: &str, from: (i32, i32), radius: i32) -> Option<Box<dyn LocateJob<Self>>> {
        super::locate::job(self.clone(), name, from, radius)
    }
}
