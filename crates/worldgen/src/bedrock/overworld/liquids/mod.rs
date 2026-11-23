mod freeze;
mod kinds;

use crate::shared::CHUNK_WIDTH;
use crate::shared::blocks::{AIR, BlockId, BlockTable, Blocks, flag, int, state};
use crate::shared::chunk::proto::RoutedTick;
use crate::shared::chunk::region::{Heightmap, Region};
use chorus_block::block_id::*;
use chorus_block::state::common::{DRAG_DOWN, LIQUID_DEPTH};
use chorus_level::fluid::{CellRules, FluidBlocks, FluidRules, LiquidMotion};
use glam::IVec3;
use kinds::Kinds;
use std::sync::OnceLock;

const DEPTHS: usize = chorus_level::fluid::DEPTHS;
const BLOCK_SOURCE_CLOCK: u64 = 1;
const AQUIFER_TICK_DELAY: u64 = 1;
const BORDER_TICK_DELAY: u64 = 1;
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
}

fn depth_variants(table: &mut BlockTable, name: &'static str) -> [BlockId; DEPTHS] {
    let source = table.name(name);
    std::array::from_fn(|depth| if depth == 0 { source } else { table.get(name, &[state(&LIQUID_DEPTH, int(depth as i32))]) })
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
        }
    }

    fn is_snowable_plant(&self, block: BlockId) -> bool {
        self.snowable.contains(&block)
    }

    fn kinds(&self, blocks: &Blocks) -> &Kinds {
        self.kinds.get_or_init(|| Kinds::new(blocks))
    }

    pub fn finish_decoration(&self, region: &mut Region) -> Vec<RoutedTick> {
        let kinds = self.kinds(region.blocks);
        freeze::freeze_chunk(region, self, kinds);
        let owner = (region.owner_x, region.owner_z);
        let (min_x, min_z) = (region.owner_x * CHUNK_WIDTH, region.owner_z * CHUNK_WIDTH);
        let mut pending = Vec::new();
        let mut sourced = Vec::new();
        for tick in region.take_fluid_schedule() {
            let routed = RoutedTick {
                pos: tick.pos,
                lava: tick.lava,
                delay: tick.delay,
            };
            if !tick.on_chunk_queue {
                sourced.push(RoutedTick {
                    delay: BLOCK_SOURCE_CLOCK + tick.delay,
                    ..routed
                });
            } else {
                Self::keep(region, owner, &mut pending, routed);
            }
        }
        let mut border = Vec::new();
        self.reset_border_water(region, kinds, &mut border);
        for tick in sourced {
            Self::keep(region, owner, &mut pending, tick);
        }
        for tick in border {
            Self::keep(
                region,
                owner,
                &mut pending,
                RoutedTick {
                    delay: BLOCK_SOURCE_CLOCK + tick.delay,
                    ..tick
                },
            );
        }
        pending.extend(region.chunk_fluid_ticks().iter().map(|tick| RoutedTick {
            pos: IVec3::new(min_x + tick.x as i32, tick.y as i32, min_z + tick.z as i32),
            lava: tick.lava,
            delay: AQUIFER_TICK_DELAY,
        }));
        pending.extend(region.inherited_ticks().iter().filter(|tick| (tick.pos.x >> 4, tick.pos.z >> 4) == owner).copied());
        pending.sort_unstable_by_key(|tick| (tick.delay, tick.pos.x, tick.pos.y, tick.pos.z, tick.lava));
        pending
    }

    fn keep(region: &mut Region, owner: (i32, i32), pending: &mut Vec<RoutedTick>, tick: RoutedTick) {
        if (tick.pos.x >> 4, tick.pos.z >> 4) == owner {
            pending.push(tick);
        } else {
            region.route_tick(tick);
        }
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
