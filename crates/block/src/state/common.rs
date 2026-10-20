use crate::state::block_state::BlockStateDefinition;
use crate::{const_bool, const_enum, const_int};

pub static ACTIVE: BlockStateDefinition = const_bool!("active", false);
pub static AGE_16: BlockStateDefinition = const_int!("age", 0, 15);
pub static AGE_3: BlockStateDefinition = const_int!("age", 0, 2);
pub static AGE_4: BlockStateDefinition = const_int!("age", 0, 3);
pub static AGE_6: BlockStateDefinition = const_int!("age", 0, 5);
pub static AGE_BIT: BlockStateDefinition = const_bool!("age_bit", false);
pub static ALLOW_UNDERWATER_BIT: BlockStateDefinition = const_bool!("allow_underwater_bit", false);
pub static ATTACHED_BIT: BlockStateDefinition = const_bool!("attached_bit", false);
pub static ATTACHMENT: BlockStateDefinition = const_enum!("attachment", ["hanging", "multiple", "side", "standing"]);
pub static BAMBOO_LEAF_SIZE: BlockStateDefinition = const_enum!("bamboo_leaf_size", ["no_leaves", "small_leaves", "large_leaves"]);
pub static BAMBOO_STALK_THICKNESS: BlockStateDefinition = const_enum!("bamboo_stalk_thickness", ["thick", "thin"]);
pub static BIG_DRIPLEAF_HEAD: BlockStateDefinition = const_bool!("big_dripleaf_head", false);
pub static BIG_DRIPLEAF_TILT: BlockStateDefinition = const_enum!("big_dripleaf_tilt", ["full_tilt", "none", "partial_tilt", "unstable"]);
pub static BITE_COUNTER: BlockStateDefinition = const_int!("bite_counter", 0, 6);
pub static BLOCK_LIGHT_LEVEL: BlockStateDefinition = const_int!("block_light_level", 0, 15);
pub static BLOOM: BlockStateDefinition = const_bool!("bloom", false);
pub static BOOKS_STORED: BlockStateDefinition = const_int!("books_stored", 0, 63);
pub static BREWING_STAND_SLOT_A_BIT: BlockStateDefinition = const_bool!("brewing_stand_slot_a_bit", false);
pub static BREWING_STAND_SLOT_B_BIT: BlockStateDefinition = const_bool!("brewing_stand_slot_b_bit", false);
pub static BREWING_STAND_SLOT_C_BIT: BlockStateDefinition = const_bool!("brewing_stand_slot_c_bit", false);
pub static BRUSHED_PROGRESS: BlockStateDefinition = const_int!("brushed_progress", 0, 3);
pub static BUTTON_PRESSED_BIT: BlockStateDefinition = const_bool!("button_pressed_bit", false);
pub static CANDLES: BlockStateDefinition = const_int!("candles", 0, 3);
pub static CAN_SUMMON: BlockStateDefinition = const_bool!("can_summon", false);
pub static CAULDRON_LIQUID: BlockStateDefinition = const_enum!("cauldron_liquid", ["water", "lava", "powder_snow"]);
pub static CHEMISTRY_TABLE_TYPE: BlockStateDefinition = const_enum!("chemistry_table_type", ["compound_creator", "element_constructor", "lab_table", "material_reducer"]);
pub static CHISEL_TYPE: BlockStateDefinition = const_enum!("chisel_type", ["default", "chiseled", "lines", "smooth"]);
pub static CLUSTER_COUNT: BlockStateDefinition = const_int!("cluster_count", 0, 3);
pub static COLOR: BlockStateDefinition = const_enum!(
    "color",
    [
        "black",
        "blue",
        "brown",
        "cyan",
        "gray",
        "green",
        "light_blue",
        "lime",
        "magenta",
        "orange",
        "pink",
        "purple",
        "red",
        "silver",
        "white",
        "yellow"
    ]
);
pub static COLOR_BIT: BlockStateDefinition = const_bool!("color_bit", false);
pub static COMPOSTER_FILL_LEVEL: BlockStateDefinition = const_int!("composter_fill_level", 0, 8);
pub static CONDITIONAL_BIT: BlockStateDefinition = const_bool!("conditional_bit", false);
pub static CORAL_COLOR: BlockStateDefinition = const_enum!("coral_color", ["blue", "pink", "purple", "red", "yellow"]);
pub static CORAL_DIRECTION: BlockStateDefinition = const_int!("coral_direction", 0, 3);
pub static CORAL_FAN_DIRECTION: BlockStateDefinition = const_int!("coral_fan_direction", 0, 1);
pub static CORAL_HANG_TYPE_BIT: BlockStateDefinition = const_bool!("coral_hang_type_bit", false);
pub static COVERED_BIT: BlockStateDefinition = const_bool!("covered_bit", false);
pub static CRACKED_STATE: BlockStateDefinition = const_enum!("cracked_state", ["no_cracks", "cracked", "max_cracked"]);
pub static CRAFTING: BlockStateDefinition = const_bool!("crafting", false);
pub static CREAKING_HEART_STATE: BlockStateDefinition = const_enum!("creaking_heart_state", ["uprooted", "dormant", "awake"]);
pub static DAMAGE: BlockStateDefinition = const_enum!("damage", ["undamaged", "slightly_damaged", "very_damaged", "broken"]);
pub static DEAD_BIT: BlockStateDefinition = const_bool!("dead_bit", false);
pub static DEPRECATED: BlockStateDefinition = const_int!("deprecated", 0, 3);
pub static DIRECTION: BlockStateDefinition = const_int!("direction", 0, 3);
pub static DIRT_TYPE: BlockStateDefinition = const_enum!("dirt_type", ["normal", "coarse"]);
pub static DISARMED_BIT: BlockStateDefinition = const_bool!("disarmed_bit", false);
pub static DOOR_HINGE_BIT: BlockStateDefinition = const_bool!("door_hinge_bit", false);
pub static DOUBLE_PLANT_TYPE: BlockStateDefinition = const_enum!("double_plant_type", ["sunflower", "syringa", "grass", "fern", "rose", "paeonia", "pitcher_plant"]);
pub static DRAG_DOWN: BlockStateDefinition = const_bool!("drag_down", false);
pub static DRIPSTONE_THICKNESS: BlockStateDefinition = const_enum!("dripstone_thickness", ["base", "frustum", "merge", "middle", "tip"]);
pub static END_PORTAL_EYE_BIT: BlockStateDefinition = const_bool!("end_portal_eye_bit", false);
pub static EXPLODE_BIT: BlockStateDefinition = const_bool!("explode_bit", false);
pub static EXTINGUISHED: BlockStateDefinition = const_bool!("extinguished", false);
pub static FACING_DIRECTION: BlockStateDefinition = const_int!("facing_direction", 0, 5);
pub static FILL_LEVEL: BlockStateDefinition = const_int!("fill_level", 0, 6);
pub static FLOWER_TYPE: BlockStateDefinition = const_enum!(
    "flower_type",
    [
        "poppy",
        "orchid",
        "allium",
        "houstonia",
        "tulip_red",
        "tulip_orange",
        "tulip_white",
        "tulip_pink",
        "oxeye",
        "cornflower",
        "lily_of_the_valley"
    ]
);
pub static GROUND_SIGN_DIRECTION: BlockStateDefinition = const_int!("ground_sign_direction", 0, 15);
pub static GROWING_PLANT_AGE: BlockStateDefinition = const_int!("growing_plant_age", 0, 25);
pub static GROWTH: BlockStateDefinition = const_int!("growth", 0, 7);
pub static HANGING: BlockStateDefinition = const_bool!("hanging", false);
pub static HEAD_PIECE_BIT: BlockStateDefinition = const_bool!("head_piece_bit", false);
pub static HEIGHT: BlockStateDefinition = const_int!("height", 0, 7);
pub static HONEY_LEVEL: BlockStateDefinition = const_int!("honey_level", 0, 5);
pub static HUGE_MUSHROOM_BITS: BlockStateDefinition = const_int!("huge_mushroom_bits", 0, 15);
pub static INFINIBURN_BIT: BlockStateDefinition = const_bool!("infiniburn_bit", false);
pub static IN_WALL_BIT: BlockStateDefinition = const_bool!("in_wall_bit", false);
pub static ITEM_FRAME_MAP_BIT: BlockStateDefinition = const_bool!("item_frame_map_bit", false);
pub static ITEM_FRAME_PHOTO_BIT: BlockStateDefinition = const_bool!("item_frame_photo_bit", false);
pub static KELP_AGE: BlockStateDefinition = const_int!("kelp_age", 0, 25);
pub static LEVER_DIRECTION: BlockStateDefinition = const_enum!("lever_direction", ["down_x", "down_z", "east", "north", "south", "up_x", "up_z", "west"]);
pub static LIQUID_DEPTH: BlockStateDefinition = const_int!("liquid_depth", 0, 15);
pub static LIT: BlockStateDefinition = const_bool!("lit", false);
pub static MINECRAFT_BLOCK_FACE: BlockStateDefinition = const_enum!("minecraft:block_face", ["down", "up", "north", "south", "west", "east"]);
pub static MINECRAFT_CARDINAL_DIRECTION: BlockStateDefinition = const_enum!("minecraft:cardinal_direction", ["south", "west", "north", "east"]);
pub static MINECRAFT_FACING_DIRECTION: BlockStateDefinition = const_enum!("minecraft:facing_direction", ["down", "up", "north", "south", "west", "east"]);
pub static MINECRAFT_VERTICAL_HALF: BlockStateDefinition = const_enum!("minecraft:vertical_half", ["bottom", "top"]);
pub static MOISTURIZED_AMOUNT: BlockStateDefinition = const_int!("moisturized_amount", 0, 7);
pub static MONSTER_EGG_STONE_TYPE: BlockStateDefinition = const_enum!(
    "monster_egg_stone_type",
    ["chiseled_stone_brick", "cobblestone", "cracked_stone_brick", "mossy_stone_brick", "stone", "stone_brick"]
);
pub static MULTI_FACE_DIRECTION_BITS: BlockStateDefinition = const_int!("multi_face_direction_bits", 0, 63);
pub static NATURAL: BlockStateDefinition = const_bool!("natural", false);
pub static NETHER_REACTOR_STATE: BlockStateDefinition = const_enum!("nether_reactor_state", ["ready", "initialized", "finished"]);
pub static NEW_LEAF_TYPE: BlockStateDefinition = const_enum!("new_leaf_type", ["acacia", "dark_oak"]);
pub static NEW_LOG_TYPE: BlockStateDefinition = const_enum!("new_log_type", ["acacia", "dark_oak"]);
pub static OCCUPIED_BIT: BlockStateDefinition = const_bool!("occupied_bit", false);
pub static OLD_LEAF_TYPE: BlockStateDefinition = const_enum!("old_leaf_type", ["oak", "spruce", "birch", "jungle"]);
pub static OLD_LOG_TYPE: BlockStateDefinition = const_enum!("old_log_type", ["oak", "spruce", "birch", "jungle"]);
pub static OMINOUS: BlockStateDefinition = const_bool!("ominous", false);
pub static OPEN_BIT: BlockStateDefinition = const_bool!("open_bit", false);
pub static ORIENTATION: BlockStateDefinition = const_enum!(
    "orientation",
    [
        "down_east",
        "down_north",
        "down_south",
        "down_west",
        "east_up",
        "north_up",
        "south_up",
        "up_east",
        "up_north",
        "up_south",
        "up_west",
        "west_up"
    ]
);
pub static OUTPUT_LIT_BIT: BlockStateDefinition = const_bool!("output_lit_bit", false);
pub static OUTPUT_SUBTRACT_BIT: BlockStateDefinition = const_bool!("output_subtract_bit", false);
pub static OXIDIZATION_LEVEL: BlockStateDefinition = const_enum!("oxidization_level", ["unaffected", "exposed", "weathered", "oxidized"]);
pub static PALE_MOSS_CARPET_SIDE_EAST: BlockStateDefinition = const_enum!("pale_moss_carpet_side_east", ["none", "short", "tall"]);
pub static PALE_MOSS_CARPET_SIDE_NORTH: BlockStateDefinition = const_enum!("pale_moss_carpet_side_north", ["none", "short", "tall"]);
pub static PALE_MOSS_CARPET_SIDE_SOUTH: BlockStateDefinition = const_enum!("pale_moss_carpet_side_south", ["none", "short", "tall"]);
pub static PALE_MOSS_CARPET_SIDE_WEST: BlockStateDefinition = const_enum!("pale_moss_carpet_side_west", ["none", "short", "tall"]);
pub static PERSISTENT_BIT: BlockStateDefinition = const_bool!("persistent_bit", false);
pub static PILLAR_AXIS: BlockStateDefinition = const_enum!("pillar_axis", ["y", "z", "x"]);
pub static PORTAL_AXIS: BlockStateDefinition = const_enum!("portal_axis", ["unknown", "x", "z"]);
pub static POTENT_SULFUR_STATE: BlockStateDefinition = const_enum!("potent_sulfur_state", ["dry", "wet", "dormant", "erupting", "continuous"]);
pub static POWERED_BIT: BlockStateDefinition = const_bool!("powered_bit", false);
pub static PRISMARINE_BLOCK_TYPE: BlockStateDefinition = const_enum!("prismarine_block_type", ["bricks", "dark", "default"]);
pub static PROPAGULE_STAGE: BlockStateDefinition = const_int!("propagule_stage", 0, 4);
pub static RAIL_DATA_BIT: BlockStateDefinition = const_bool!("rail_data_bit", false);
pub static RAIL_DIRECTION_10: BlockStateDefinition = const_int!("rail_direction", 0, 9);
pub static RAIL_DIRECTION_6: BlockStateDefinition = const_int!("rail_direction", 0, 5);
pub static REDSTONE_SIGNAL: BlockStateDefinition = const_int!("redstone_signal", 0, 15);
pub static REHYDRATION_LEVEL: BlockStateDefinition = const_int!("rehydration_level", 0, 3);
pub static REPEATER_DELAY: BlockStateDefinition = const_int!("repeater_delay", 0, 3);
pub static RESPAWN_ANCHOR_CHARGE: BlockStateDefinition = const_int!("respawn_anchor_charge", 0, 4);
pub static ROTATION: BlockStateDefinition = const_int!("rotation", 0, 3);
pub static SAND_STONE_TYPE: BlockStateDefinition = const_enum!("sand_stone_type", ["default", "heiroglyphs", "cut", "smooth"]);
pub static SAND_TYPE: BlockStateDefinition = const_enum!("sand_type", ["normal", "red"]);
pub static SCULK_SENSOR_PHASE: BlockStateDefinition = const_int!("sculk_sensor_phase", 0, 2);
pub static SEA_GRASS_TYPE: BlockStateDefinition = const_enum!("sea_grass_type", ["default", "double_top", "double_bot"]);
pub static SPONGE_TYPE: BlockStateDefinition = const_enum!("sponge_type", ["dry", "wet"]);
pub static STABILITY: BlockStateDefinition = const_int!("stability", 0, 7);
pub static STABILITY_CHECK: BlockStateDefinition = const_bool!("stability_check", false);
pub static STONE_BRICK_TYPE: BlockStateDefinition = const_enum!("stone_brick_type", ["chiseled", "cracked", "default", "mossy", "smooth"]);
pub static STONE_SLAB_TYPE: BlockStateDefinition = const_enum!(
    "stone_slab_type",
    ["smooth_stone", "sandstone", "wood", "cobblestone", "brick", "stone_brick", "quarts", "nether_brick"]
);
pub static STONE_SLAB_TYPE_2: BlockStateDefinition = const_enum!(
    "stone_slab_type_2",
    [
        "red_sandstone",
        "purpur",
        "prismarine_rough",
        "prismarine_dark",
        "prismarine_brick",
        "mossy_cobblestone",
        "smooth_sandstone",
        "red_nether_brick"
    ]
);
pub static STONE_SLAB_TYPE_3: BlockStateDefinition = const_enum!(
    "stone_slab_type_3",
    [
        "end_stone_brick",
        "smooth_red_sandstone",
        "polished_andesite",
        "andesite",
        "diorite",
        "polished_diorite",
        "granite",
        "polished_granite"
    ]
);
pub static STONE_SLAB_TYPE_4: BlockStateDefinition = const_enum!("stone_slab_type_4", ["mossy_stone_brick", "smooth_quartz", "stone", "cut_sandstone", "cut_red_sandstone"]);
pub static STONE_TYPE: BlockStateDefinition = const_enum!("stone_type", ["andesite", "andesite_smooth", "diorite", "diorite_smooth", "granite", "granite_smooth", "stone"]);
pub static STRIPPED_BIT: BlockStateDefinition = const_bool!("stripped_bit", false);
pub static STRUCTURE_BLOCK_TYPE: BlockStateDefinition = const_enum!("structure_block_type", ["invalid", "data", "save", "load", "corner", "export"]);
pub static STRUCTURE_VOID_TYPE: BlockStateDefinition = const_enum!("structure_void_type", ["air", "void"]);
pub static SUSPENDED_BIT: BlockStateDefinition = const_bool!("suspended_bit", false);
pub static TALL_GRASS_TYPE: BlockStateDefinition = const_enum!("tall_grass_type", ["default", "tall", "fern", "snow"]);
pub static TIP: BlockStateDefinition = const_bool!("tip", false);
pub static TOGGLE_BIT: BlockStateDefinition = const_bool!("toggle_bit", false);
pub static TORCH_FACING_DIRECTION: BlockStateDefinition = const_enum!("torch_facing_direction", ["unknown", "west", "east", "north", "south", "top"]);
pub static TRIAL_SPAWNER_STATE: BlockStateDefinition = const_int!("trial_spawner_state", 0, 5);
pub static TRIGGERED_BIT: BlockStateDefinition = const_bool!("triggered_bit", false);
pub static TURTLE_EGG_COUNT: BlockStateDefinition = const_enum!("turtle_egg_count", ["one_egg", "two_egg", "three_egg", "four_egg"]);
pub static TWISTING_VINES_AGE: BlockStateDefinition = const_int!("twisting_vines_age", 0, 25);
pub static UPDATE_BIT: BlockStateDefinition = const_bool!("update_bit", false);
pub static UPPER_BLOCK_BIT: BlockStateDefinition = const_bool!("upper_block_bit", false);
pub static UPSIDE_DOWN_BIT: BlockStateDefinition = const_bool!("upside_down_bit", false);
pub static VAULT_STATE: BlockStateDefinition = const_enum!("vault_state", ["inactive", "active", "unlocking", "ejecting"]);
pub static VINE_DIRECTION_BITS: BlockStateDefinition = const_int!("vine_direction_bits", 0, 15);
pub static WALL_BLOCK_TYPE: BlockStateDefinition = const_enum!(
    "wall_block_type",
    [
        "andesite",
        "brick",
        "cobblestone",
        "diorite",
        "end_brick",
        "granite",
        "mossy_cobblestone",
        "mossy_stone_brick",
        "nether_brick",
        "prismarine",
        "red_nether_brick",
        "red_sandstone",
        "sandstone",
        "stone_brick"
    ]
);
pub static WALL_CONNECTION_TYPE_EAST: BlockStateDefinition = const_enum!("wall_connection_type_east", ["none", "short", "tall"]);
pub static WALL_CONNECTION_TYPE_NORTH: BlockStateDefinition = const_enum!("wall_connection_type_north", ["none", "short", "tall"]);
pub static WALL_CONNECTION_TYPE_SOUTH: BlockStateDefinition = const_enum!("wall_connection_type_south", ["none", "short", "tall"]);
pub static WALL_CONNECTION_TYPE_WEST: BlockStateDefinition = const_enum!("wall_connection_type_west", ["none", "short", "tall"]);
pub static WALL_POST_BIT: BlockStateDefinition = const_bool!("wall_post_bit", false);
pub static WEEPING_VINES_AGE: BlockStateDefinition = const_int!("weeping_vines_age", 0, 25);
pub static WEIRDO_DIRECTION: BlockStateDefinition = const_int!("weirdo_direction", 0, 3);
pub static WOOD_TYPE: BlockStateDefinition = const_enum!("wood_type", ["oak", "spruce", "birch", "jungle", "acacia", "dark_oak", "cherry", "pale_oak", "mangrove"]);
pub static POWERED_SHELF_TYPE: BlockStateDefinition = const_int!("powered_shelf_type", 0, 3);
pub static DIRECTION_16: BlockStateDefinition = const_int!("direction", 0, 15);
pub static MINECRAFT_CORNER: BlockStateDefinition = const_enum!("minecraft:corner", ["none", "inner_left", "inner_right", "outer_left", "outer_right"]);
pub static MINECRAFT_CONNECTION_DOWN: BlockStateDefinition = const_bool!("minecraft:connection_down", false);
pub static MINECRAFT_CONNECTION_EAST: BlockStateDefinition = const_bool!("minecraft:connection_east", false);
pub static MINECRAFT_CONNECTION_NORTH: BlockStateDefinition = const_bool!("minecraft:connection_north", false);
pub static MINECRAFT_CONNECTION_SOUTH: BlockStateDefinition = const_bool!("minecraft:connection_south", false);
pub static MINECRAFT_CONNECTION_UP: BlockStateDefinition = const_bool!("minecraft:connection_up", false);
pub static MINECRAFT_CONNECTION_WEST: BlockStateDefinition = const_bool!("minecraft:connection_west", false);
