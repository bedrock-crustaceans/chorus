use crate::shared::blocks::{BlockId, BlockTable};
use crate::shared::structure::palette::register_named;

macro_rules! materials {
    ($($variant:ident => $name:literal $states:literal,)*) => {
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        #[repr(usize)]
        pub enum Material {
            $($variant,)*
        }

        const TABLE: &[(&str, &str)] = &[$(($name, $states),)*];
    };
}

materials! {
    Air => "air" "",
    Sandstone => "sandstone" "",
    CutSandstone => "cut_sandstone" "",
    ChiseledSandstone => "chiseled_sandstone" "",
    OrangeTerracotta => "orange_terracotta" "",
    BlueTerracotta => "blue_terracotta" "",
    SandstoneStairsEast => "sandstone_stairs" "minecraft:corner=none,upside_down_bit=0,weirdo_direction=0",
    SandstoneStairsWest => "sandstone_stairs" "minecraft:corner=none,upside_down_bit=0,weirdo_direction=1",
    SandstoneStairsSouth => "sandstone_stairs" "minecraft:corner=none,upside_down_bit=0,weirdo_direction=2",
    SandstoneStairsNorth => "sandstone_stairs" "minecraft:corner=none,upside_down_bit=0,weirdo_direction=3",
    SandstoneSlab => "sandstone_slab" "minecraft:vertical_half=bottom",
    Sand => "sand" "",
    SuspiciousSand => "suspicious_sand" "brushed_progress=0,hanging=1",
    StonePressurePlate => "stone_pressure_plate" "redstone_signal=0",
    Tnt => "tnt" "explode_bit=0",
    ChestSouth => "chest" "minecraft:cardinal_direction=south",
    ChestWest => "chest" "minecraft:cardinal_direction=west",
    ChestNorth => "chest" "minecraft:cardinal_direction=north",
    ChestEast => "chest" "minecraft:cardinal_direction=east",
    SprucePlanks => "spruce_planks" "",
    OakLog => "oak_log" "pillar_axis=y",
    OakFence => "oak_fence" "minecraft:connection_east=0,minecraft:connection_north=0,minecraft:connection_south=0,minecraft:connection_west=0",
    FlowerPot => "flower_pot" "update_bit=1",
    CraftingTable => "crafting_table" "",
    Cauldron2 => "cauldron" "cauldron_liquid=water,fill_level=2",
    Cauldron4 => "cauldron" "cauldron_liquid=water,fill_level=4",
    Cauldron6 => "cauldron" "cauldron_liquid=water,fill_level=6",
    SpruceStairsEast => "spruce_stairs" "minecraft:corner=none,upside_down_bit=0,weirdo_direction=0",
    SpruceStairsWest => "spruce_stairs" "minecraft:corner=none,upside_down_bit=0,weirdo_direction=1",
    SpruceStairsSouth => "spruce_stairs" "minecraft:corner=none,upside_down_bit=0,weirdo_direction=2",
    SpruceStairsNorth => "spruce_stairs" "minecraft:corner=none,upside_down_bit=0,weirdo_direction=3",
    Cobblestone => "cobblestone" "",
    MossyCobblestone => "mossy_cobblestone" "",
    StoneStairsEast => "stone_stairs" "minecraft:corner=none,upside_down_bit=0,weirdo_direction=0",
    StoneStairsWest => "stone_stairs" "minecraft:corner=none,upside_down_bit=0,weirdo_direction=1",
    StoneStairsSouth => "stone_stairs" "minecraft:corner=none,upside_down_bit=0,weirdo_direction=2",
    StoneStairsNorth => "stone_stairs" "minecraft:corner=none,upside_down_bit=0,weirdo_direction=3",
    TripwireHookEast => "tripwire_hook" "attached_bit=1,direction=3,powered_bit=0",
    TripwireHookWest => "tripwire_hook" "attached_bit=1,direction=1,powered_bit=0",
    TripwireHookSouth => "tripwire_hook" "attached_bit=0,direction=0,powered_bit=0",
    TripwireHookNorth => "tripwire_hook" "attached_bit=0,direction=2,powered_bit=0",
    TripWireEastWest => "trip_wire" "attached_bit=1,disarmed_bit=0,minecraft:connection_east=1,minecraft:connection_north=0,minecraft:connection_south=0,minecraft:connection_west=1,powered_bit=0,suspended_bit=0",
    TripWireNorthSouth => "trip_wire" "attached_bit=1,disarmed_bit=0,minecraft:connection_east=0,minecraft:connection_north=1,minecraft:connection_south=1,minecraft:connection_west=0,powered_bit=0,suspended_bit=0",
    RedstoneWire => "redstone_wire" "redstone_signal=0",
    DispenserSouth => "dispenser" "facing_direction=3,triggered_bit=0",
    DispenserWest => "dispenser" "facing_direction=4,triggered_bit=0",
    VineAllSides => "vine" "vine_direction_bits=15",
    ChiseledStoneBricks => "chiseled_stone_bricks" "",
    LeverSouth => "lever" "lever_direction=south,open_bit=0",
    StickyPistonUp => "sticky_piston" "facing_direction=1",
    StickyPistonWest => "sticky_piston" "facing_direction=5",
    RepeaterSouth => "unpowered_repeater" "minecraft:cardinal_direction=south,repeater_delay=0",
    GrassBlock => "grass_block" "",
    Dirt => "dirt" "",
    Gravel => "gravel" "",
    StoneBricks => "stone_bricks" "",
    SuspiciousGravel => "suspicious_gravel" "brushed_progress=0,hanging=1",
}

impl Material {
    pub const CHESTS: [Material; 4] = [Material::ChestSouth, Material::ChestWest, Material::ChestNorth, Material::ChestEast];
    pub const CAULDRONS: [Material; 3] = [Material::Cauldron2, Material::Cauldron4, Material::Cauldron6];
    pub const DISPENSERS: [Material; 2] = [Material::DispenserSouth, Material::DispenserWest];
}

#[derive(Clone, Default)]
pub struct ScatteredBlocks {
    ids: Vec<BlockId>,
    pub flooded_chest_east: BlockId,
}

impl ScatteredBlocks {
    pub fn register(table: &mut BlockTable) -> Self {
        let ids: Vec<BlockId> = TABLE.iter().map(|&(name, states)| register_named(table, name, states)).collect();
        let flooded_chest_east = table.flooded(ids[Material::ChestEast as usize]);
        Self { ids, flooded_chest_east }
    }

    pub fn id(&self, material: Material) -> BlockId {
        self.ids[material as usize]
    }

    pub fn is_any(&self, block: BlockId, materials: &[Material]) -> bool {
        materials.iter().any(|&material| self.id(material) == block)
    }
}
