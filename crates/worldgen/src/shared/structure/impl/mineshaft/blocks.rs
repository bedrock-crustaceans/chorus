use crate::shared::blocks::{BlockId, BlockTable};
use crate::shared::structure::palette::register_named;
use chorus_block::block_id::BEDROCK;

const FENCE_SIDES: [(&str, u8); 4] = [("east", 1), ("north", 2), ("south", 4), ("west", 8)];
const FENCE_VARIANTS: usize = 16;
pub const TORCH_FACINGS: [&str; 4] = chorus_block::state::common::CARDINAL_DIRECTIONS;

#[derive(Clone, Default)]
pub struct MineshaftBlocks {
    pub oak_planks: BlockId,
    pub dark_oak_planks: BlockId,
    pub oak_log: BlockId,
    pub dark_oak_log: BlockId,
    pub chain: BlockId,
    pub web: BlockId,
    pub spawner: BlockId,
    pub rail_north_south: BlockId,
    pub rail_east_west: BlockId,
    pub bedrock: BlockId,
    pub torches: [BlockId; 4],
    pub oak_fences: [BlockId; FENCE_VARIANTS],
    pub dark_oak_fences: [BlockId; FENCE_VARIANTS],
}

fn fences(table: &mut BlockTable, name: &str) -> [BlockId; FENCE_VARIANTS] {
    std::array::from_fn(|bits| {
        let states: Vec<String> = FENCE_SIDES
            .iter()
            .map(|&(side, bit)| format!("minecraft:connection_{side}={}", u8::from(bits as u8 & bit != 0)))
            .collect();
        register_named(table, name, &states.join(","))
    })
}

impl MineshaftBlocks {
    pub fn register(table: &mut BlockTable) -> Self {
        Self {
            oak_planks: register_named(table, "oak_planks", ""),
            dark_oak_planks: register_named(table, "dark_oak_planks", ""),
            oak_log: register_named(table, "oak_log", "pillar_axis=y"),
            dark_oak_log: register_named(table, "dark_oak_log", "pillar_axis=y"),
            chain: register_named(table, "iron_chain", "pillar_axis=y"),
            web: register_named(table, "web", ""),
            spawner: register_named(table, "mob_spawner", ""),
            rail_north_south: register_named(table, "rail", "rail_direction=0"),
            rail_east_west: register_named(table, "rail", "rail_direction=1"),
            bedrock: table.name(BEDROCK),
            torches: TORCH_FACINGS.map(|facing| register_named(table, "torch", &format!("torch_facing_direction={facing}"))),
            oak_fences: fences(table, "oak_fence"),
            dark_oak_fences: fences(table, "dark_oak_fence"),
        }
    }

    pub fn is_kept(&self, block: BlockId) -> bool {
        [self.oak_planks, self.dark_oak_planks, self.oak_log, self.dark_oak_log, self.chain, self.bedrock].contains(&block)
    }

    pub fn is_rail(&self, block: BlockId) -> bool {
        block == self.rail_north_south || block == self.rail_east_west
    }
}
