mod freeze;
mod kinds;

use crate::shared::CHUNK_WIDTH;
use crate::shared::blocks::{AIR, BlockId, BlockTable, Blocks, flag, int, state};
use crate::shared::chunk::proto::RoutedTick;
use crate::shared::chunk::region::{Heightmap, Region};
use chorus_block::block_id::*;
use chorus_block::state::common::{DRAG_DOWN, LIQUID_DEPTH};
use chorus_level::fluid::{CellRules, FluidBlocks, FluidRules, FluidTicks, Grid, LiquidMotion};
use glam::IVec3;
use kinds::Kinds;
use std::sync::OnceLock;

const DEPTHS: usize = chorus_level::fluid::DEPTHS;
const BLOCK_SOURCE_CLOCK: u64 = 1;
const AQUIFER_TICK_DELAY: u64 = 1;
const BORDER_TICK_DELAY: u64 = 1;
const DECORATE_GENERATIONS: usize = 128;
const SNOWABLE_PLANTS: [&str; 14] = [
    SHORT_GRASS,
    FERN,
    POPPY,
    DANDELION,
    BLUE_ORCHID,
    AZURE_BLUET,
    RED_TULIP,
    ORANGE_TULIP,
    WHITE_TULIP,
    PINK_TULIP,
    OXEYE_DAISY,
    CORNFLOWER,
    LILY_OF_THE_VALLEY,
    ALLIUM,
];

pub struct Liquids {
    water: [BlockId; DEPTHS],
    lava: [BlockId; DEPTHS],
    ice: BlockId,
    snow_layer: BlockId,
    obsidian: BlockId,
    cobblestone: BlockId,
    stone: BlockId,
    bubble_column: BlockId,
    magma: BlockId,
    snowable: Vec<BlockId>,
    kinds: OnceLock<Kinds>,
    decorate_rules: OnceLock<FluidRules>,
}

fn depth_variants(table: &mut BlockTable, name: &'static str) -> [BlockId; DEPTHS] {
    let source = table.name(name);
    std::array::from_fn(|depth| if depth == 0 { source } else { table.get(name, &[state(&LIQUID_DEPTH, int(depth as i32))]) })
}

struct RegionGrid<'r, 'a> {
    region: &'r mut Region<'a>,
}

impl Grid for RegionGrid<'_, '_> {
    fn read(&self, pos: IVec3) -> Option<i32> {
        self.region.get_contained(pos).map(i32::from)
    }

    fn write(&mut self, pos: IVec3, block: i32) -> bool {
        self.region.set(pos, block as BlockId)
    }

    fn can_tick(&self, pos: IVec3) -> bool {
        [-1, 1].iter().all(|dx| [-1, 1].iter().all(|dz| self.region.has_column(pos.x + dx, pos.z + dz)))
    }

    fn route(&mut self, pos: IVec3, lava: bool, delay: u64) -> bool {
        if self.region.queue_reaches(pos) {
            return false;
        }
        self.region.route_tick(RoutedTick { pos, lava, delay });
        true
    }
}

impl Liquids {
    pub fn new(table: &mut BlockTable) -> Self {
        Self {
            water: depth_variants(table, WATER),
            lava: depth_variants(table, LAVA),
            ice: table.name(ICE),
            snow_layer: table.name(SNOW_LAYER),
            obsidian: table.name(OBSIDIAN),
            cobblestone: table.name(COBBLESTONE),
            stone: table.name(STONE),
            bubble_column: table.get_flooded(BUBBLE_COLUMN, &[state(&DRAG_DOWN, flag(true))]),
            magma: table.name(MAGMA),
            snowable: SNOWABLE_PLANTS.iter().map(|name| table.name(*name)).collect(),
            kinds: OnceLock::new(),
            decorate_rules: OnceLock::new(),
        }
    }

    fn is_snowable_plant(&self, block: BlockId) -> bool {
        self.snowable.contains(&block)
    }

    fn kinds(&self, blocks: &Blocks) -> &Kinds {
        self.kinds.get_or_init(|| Kinds::new(blocks))
    }

    fn decorate_rules(&self, blocks: &Blocks) -> &FluidRules {
        self.decorate_rules.get_or_init(|| {
            let kinds = self.kinds(blocks);
            let mut cells = rustc_hash::FxHashMap::default();
            for id in 0..blocks.entries().len() as BlockId {
                cells.insert(
                    i32::from(id),
                    CellRules {
                        liquid: kinds.liquid(id),
                        waterlogged: kinds.is_waterlogged(id),
                        stops_water: kinds.stops(id, false),
                        stops_lava: kinds.stops(id, true),
                        solid: kinds.is_solid(id),
                        water_source: kinds.is_water_source_block(id),
                        magma: id == self.magma,
                    },
                );
            }
            FluidRules::new(
                FluidBlocks {
                    air: i32::from(AIR),
                    stone: i32::from(self.stone),
                    obsidian: i32::from(self.obsidian),
                    cobblestone: i32::from(self.cobblestone),
                    bubble_column: i32::from(self.bubble_column),
                    water: self.water.map(i32::from),
                    lava: self.lava.map(i32::from),
                    water_motion: LiquidMotion::WATER,
                    lava_motion: LiquidMotion::LAVA,
                },
                cells,
            )
        })
    }

    pub fn finish_decoration(&self, region: &mut Region) -> Vec<RoutedTick> {
        let kinds = self.kinds(region.blocks);
        freeze::freeze_chunk(region, self, kinds);
        let rules = self.decorate_rules(region.blocks);
        let owner = (region.owner_x, region.owner_z);
        let (min_x, min_z) = (region.owner_x * CHUNK_WIDTH, region.owner_z * CHUNK_WIDTH);
        let mut sourced = Vec::new();
        let mut generated_ticks = Vec::new();
        for tick in region.take_fluid_schedule() {
            if !tick.on_chunk_queue {
                sourced.push(tick);
            } else if (tick.pos.x >> 4, tick.pos.z >> 4) == owner {
                generated_ticks.push(tick);
            } else {
                region.route_tick(RoutedTick {
                    pos: tick.pos,
                    lava: tick.lava,
                    delay: tick.delay,
                });
            }
        }
        let mut border = Vec::new();
        self.reset_border_water(region, kinds, &mut border);
        let mut ticks = FluidTicks::generation_clock(BLOCK_SOURCE_CLOCK);
        for tick in sourced {
            ticks.add(tick.pos, tick.lava, tick.delay);
        }
        for tick in border {
            ticks.add(tick.pos, tick.lava, tick.delay);
        }
        let mut generated = FluidTicks::generation_clock(0);
        for tick in region.chunk_fluid_ticks() {
            generated.add(IVec3::new(min_x + tick.x as i32, tick.y as i32, min_z + tick.z as i32), tick.lava, AQUIFER_TICK_DELAY);
        }
        for tick in region.inherited_ticks() {
            generated.add(tick.pos, tick.lava, tick.delay);
        }
        for tick in generated_ticks {
            generated.add(tick.pos, tick.lava, tick.delay);
        }
        ticks.absorb(generated);
        {
            let mut grid = RegionGrid { region };
            for _ in 0..DECORATE_GENERATIONS {
                if ticks.is_empty() {
                    break;
                }
                ticks.advance_generation(&mut grid, rules);
            }
        }
        ticks
            .take_pending()
            .into_iter()
            .filter(|update| (update.pos.x >> 4, update.pos.z >> 4) == owner)
            .map(|update| RoutedTick {
                pos: update.pos,
                lava: update.lava,
                delay: update.due,
            })
            .collect()
    }

    pub fn rules(&self, blocks: &Blocks) -> FluidRules {
        let kinds = self.kinds(blocks);
        let mut cells = rustc_hash::FxHashMap::default();
        for (runtime, id) in blocks.canonical_runtime_ids() {
            cells.insert(
                runtime,
                CellRules {
                    liquid: kinds.liquid(id),
                    waterlogged: kinds.is_waterlogged(id),
                    stops_water: kinds.stops(id, false),
                    stops_lava: kinds.stops(id, true),
                    solid: kinds.is_solid(id),
                    water_source: kinds.is_water_source_block(id),
                    magma: id == self.magma,
                },
            );
        }
        FluidRules::new(
            FluidBlocks {
                air: blocks.runtime(AIR),
                stone: blocks.runtime(self.stone),
                obsidian: blocks.runtime(self.obsidian),
                cobblestone: blocks.runtime(self.cobblestone),
                bubble_column: blocks.runtime(self.bubble_column),
                water: self.water.map(|block| blocks.runtime(block)),
                lava: self.lava.map(|block| blocks.runtime(block)),
                water_motion: LiquidMotion::WATER,
                lava_motion: LiquidMotion::LAVA,
            },
            cells,
        )
    }

    fn reset_border_water(&self, region: &mut Region, kinds: &Kinds, pending: &mut Vec<RoutedTick>) {
        let (min_x, min_z) = (region.owner_x * CHUNK_WIDTH, region.owner_z * CHUNK_WIDTH);
        let last = CHUNK_WIDTH - 1;
        for dx in 0..CHUNK_WIDTH {
            for dz in 0..CHUNK_WIDTH {
                if dx != 0 && dx != last && dz != 0 && dz != last {
                    continue;
                }
                let (x, z) = (min_x + dx, min_z + dz);
                let top = region.height(Heightmap::WorldSurfaceWg, x, z);
                let mut dry_below = true;
                for y in region.min_y()..top {
                    let pos = IVec3::new(x, y, z);
                    let block = region.get(pos);
                    let water = kinds.is_named_water(block);
                    if water && dry_below {
                        if !kinds.is_water_source_block(block) {
                            region.set(pos, self.water[0]);
                        }
                        pending.push(RoutedTick {
                            pos,
                            lava: false,
                            delay: BORDER_TICK_DELAY,
                        });
                    }
                    dry_below = !water;
                }
            }
        }
    }
}
