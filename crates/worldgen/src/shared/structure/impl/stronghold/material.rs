use crate::shared::blocks::{BlockId, BlockTable};
use crate::shared::structure::palette::register_named;

const BARS_STATES: &str = "minecraft:connection_east=0,minecraft:connection_north=0,minecraft:connection_south=0,minecraft:connection_west=0";

macro_rules! materials {
    ($($variant:ident => $name:literal $states:expr,)*) => {
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
    Water => "water" "liquid_depth=0",
    Lava => "lava" "liquid_depth=0",
    StoneBricks => "stone_bricks" "",
    MossyStoneBricks => "mossy_stone_bricks" "",
    CrackedStoneBricks => "cracked_stone_bricks" "",
    InfestedStoneBricks => "infested_stone_bricks" "",
    StoneBrickSlab => "stone_brick_slab" "minecraft:vertical_half=bottom",
    SmoothStoneSlab => "smooth_stone_slab" "minecraft:vertical_half=bottom",
    SmoothStoneDoubleSlab => "smooth_stone_double_slab" "minecraft:vertical_half=bottom",
    Cobblestone => "cobblestone" "",
    OakPlanks => "oak_planks" "",
    OakFence => "oak_fence" BARS_STATES,
    Bookshelf => "bookshelf" "",
    Web => "web" "",
    MobSpawner => "mob_spawner" "",
    IronBars => "iron_bars" BARS_STATES,
    TorchTop => "torch" "torch_facing_direction=top",
    TorchUnknown => "torch" "torch_facing_direction=unknown",
    TorchWest => "torch" "torch_facing_direction=west",
    TorchEast => "torch" "torch_facing_direction=east",
    TorchNorth => "torch" "torch_facing_direction=north",
    TorchSouth => "torch" "torch_facing_direction=south",
    StoneStairsEast => "stone_stairs" "minecraft:corner=none,upside_down_bit=0,weirdo_direction=0",
    StoneStairsWest => "stone_stairs" "minecraft:corner=none,upside_down_bit=0,weirdo_direction=1",
    StoneStairsSouth => "stone_stairs" "minecraft:corner=none,upside_down_bit=0,weirdo_direction=2",
    StoneStairsNorth => "stone_stairs" "minecraft:corner=none,upside_down_bit=0,weirdo_direction=3",
    StoneBrickStairsEast => "stone_brick_stairs" "minecraft:corner=none,upside_down_bit=0,weirdo_direction=0",
    StoneBrickStairsWest => "stone_brick_stairs" "minecraft:corner=none,upside_down_bit=0,weirdo_direction=1",
    StoneBrickStairsSouth => "stone_brick_stairs" "minecraft:corner=none,upside_down_bit=0,weirdo_direction=2",
    StoneBrickStairsNorth => "stone_brick_stairs" "minecraft:corner=none,upside_down_bit=0,weirdo_direction=3",
    LadderNorth => "ladder" "facing_direction=2",
    LadderSouth => "ladder" "facing_direction=3",
    LadderWest => "ladder" "facing_direction=4",
    LadderEast => "ladder" "facing_direction=5",
    ButtonNorth => "stone_button" "button_pressed_bit=0,facing_direction=2",
    ButtonSouth => "stone_button" "button_pressed_bit=0,facing_direction=3",
    ButtonWest => "stone_button" "button_pressed_bit=0,facing_direction=4",
    ButtonEast => "stone_button" "button_pressed_bit=0,facing_direction=5",
    ChestSouth => "chest" "minecraft:cardinal_direction=south",
    ChestWest => "chest" "minecraft:cardinal_direction=west",
    ChestNorth => "chest" "minecraft:cardinal_direction=north",
    ChestEast => "chest" "minecraft:cardinal_direction=east",
    WoodenDoorEast => "wooden_door" "door_hinge_bit=0,minecraft:cardinal_direction=east,open_bit=0,upper_block_bit=0",
    WoodenDoorEastTop => "wooden_door" "door_hinge_bit=0,minecraft:cardinal_direction=east,open_bit=0,upper_block_bit=1",
    WoodenDoorSouth => "wooden_door" "door_hinge_bit=0,minecraft:cardinal_direction=south,open_bit=0,upper_block_bit=0",
    WoodenDoorSouthTop => "wooden_door" "door_hinge_bit=0,minecraft:cardinal_direction=south,open_bit=0,upper_block_bit=1",
    WoodenDoorWest => "wooden_door" "door_hinge_bit=0,minecraft:cardinal_direction=west,open_bit=0,upper_block_bit=0",
    WoodenDoorWestTop => "wooden_door" "door_hinge_bit=0,minecraft:cardinal_direction=west,open_bit=0,upper_block_bit=1",
    WoodenDoorNorth => "wooden_door" "door_hinge_bit=0,minecraft:cardinal_direction=north,open_bit=0,upper_block_bit=0",
    WoodenDoorNorthTop => "wooden_door" "door_hinge_bit=0,minecraft:cardinal_direction=north,open_bit=0,upper_block_bit=1",
    IronDoorEast => "iron_door" "door_hinge_bit=0,minecraft:cardinal_direction=east,open_bit=0,upper_block_bit=0",
    IronDoorEastTop => "iron_door" "door_hinge_bit=0,minecraft:cardinal_direction=east,open_bit=0,upper_block_bit=1",
    IronDoorSouth => "iron_door" "door_hinge_bit=0,minecraft:cardinal_direction=south,open_bit=0,upper_block_bit=0",
    IronDoorSouthTop => "iron_door" "door_hinge_bit=0,minecraft:cardinal_direction=south,open_bit=0,upper_block_bit=1",
    IronDoorWest => "iron_door" "door_hinge_bit=0,minecraft:cardinal_direction=west,open_bit=0,upper_block_bit=0",
    IronDoorWestTop => "iron_door" "door_hinge_bit=0,minecraft:cardinal_direction=west,open_bit=0,upper_block_bit=1",
    IronDoorNorth => "iron_door" "door_hinge_bit=0,minecraft:cardinal_direction=north,open_bit=0,upper_block_bit=0",
    IronDoorNorthTop => "iron_door" "door_hinge_bit=0,minecraft:cardinal_direction=north,open_bit=0,upper_block_bit=1",
    PortalFrameSouth => "end_portal_frame" "end_portal_eye_bit=0,minecraft:cardinal_direction=south",
    PortalFrameWest => "end_portal_frame" "end_portal_eye_bit=0,minecraft:cardinal_direction=west",
    PortalFrameNorth => "end_portal_frame" "end_portal_eye_bit=0,minecraft:cardinal_direction=north",
    PortalFrameEast => "end_portal_frame" "end_portal_eye_bit=0,minecraft:cardinal_direction=east",
    PortalFrameSouthEye => "end_portal_frame" "end_portal_eye_bit=1,minecraft:cardinal_direction=south",
    PortalFrameWestEye => "end_portal_frame" "end_portal_eye_bit=1,minecraft:cardinal_direction=west",
    PortalFrameNorthEye => "end_portal_frame" "end_portal_eye_bit=1,minecraft:cardinal_direction=north",
    PortalFrameEastEye => "end_portal_frame" "end_portal_eye_bit=1,minecraft:cardinal_direction=east",
    EndPortal => "end_portal" "",
}

impl Material {
    pub const CHESTS: [Material; 4] = [Material::ChestSouth, Material::ChestWest, Material::ChestNorth, Material::ChestEast];
    pub const TORCHES: [Material; 5] = [Material::TorchUnknown, Material::TorchWest, Material::TorchEast, Material::TorchNorth, Material::TorchSouth];
    pub const LADDERS: [Material; 4] = [Material::LadderNorth, Material::LadderSouth, Material::LadderWest, Material::LadderEast];
    pub const BUTTONS: [Material; 4] = [Material::ButtonNorth, Material::ButtonSouth, Material::ButtonWest, Material::ButtonEast];
    pub const STONE_STAIRS: [Material; 4] = [Material::StoneStairsEast, Material::StoneStairsWest, Material::StoneStairsSouth, Material::StoneStairsNorth];
    pub const BRICK_STAIRS: [Material; 4] = [
        Material::StoneBrickStairsEast,
        Material::StoneBrickStairsWest,
        Material::StoneBrickStairsSouth,
        Material::StoneBrickStairsNorth,
    ];
    pub const WOODEN_DOORS: [(Material, Material); 4] = [
        (Material::WoodenDoorEast, Material::WoodenDoorEastTop),
        (Material::WoodenDoorSouth, Material::WoodenDoorSouthTop),
        (Material::WoodenDoorWest, Material::WoodenDoorWestTop),
        (Material::WoodenDoorNorth, Material::WoodenDoorNorthTop),
    ];
    pub const IRON_DOORS: [(Material, Material); 4] = [
        (Material::IronDoorEast, Material::IronDoorEastTop),
        (Material::IronDoorSouth, Material::IronDoorSouthTop),
        (Material::IronDoorWest, Material::IronDoorWestTop),
        (Material::IronDoorNorth, Material::IronDoorNorthTop),
    ];
    pub const PORTAL_FRAMES: [Material; 4] = [Material::PortalFrameSouth, Material::PortalFrameWest, Material::PortalFrameNorth, Material::PortalFrameEast];
    pub const PORTAL_FRAMES_WITH_EYE: [Material; 4] = [Material::PortalFrameSouthEye, Material::PortalFrameWestEye, Material::PortalFrameNorthEye, Material::PortalFrameEastEye];
}

#[derive(Clone, Default)]
pub struct StrongholdBlocks {
    ids: Vec<BlockId>,
}

impl StrongholdBlocks {
    pub fn register(table: &mut BlockTable) -> Self {
        Self {
            ids: TABLE.iter().map(|&(name, states)| register_named(table, name, states)).collect(),
        }
    }

    pub fn id(&self, material: Material) -> BlockId {
        self.ids[material as usize]
    }

    pub fn is_chest(&self, block: BlockId) -> bool {
        Material::CHESTS.iter().any(|&chest| self.id(chest) == block)
    }
}
