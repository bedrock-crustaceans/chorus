pub fn max_stack_size(identifier: &str) -> u16 {
    let name = identifier.strip_prefix("minecraft:").unwrap_or(identifier);

    const SINGLE_SUFFIXES: &[&str] = &[
        "_sword",
        "_pickaxe",
        "_axe",
        "_shovel",
        "_hoe",
        "_helmet",
        "_chestplate",
        "_leggings",
        "_boots",
        "_horse_armor",
        "_boat",
        "_chest_boat",
        "_raft",
        "_chest_raft",
        "_minecart",
        "_bed",
        "_shulker_box",
        "_bundle",
        "_banner_pattern",
    ];
    const SINGLE_NAMES: &[&str] = &[
        "bow",
        "crossbow",
        "trident",
        "fishing_rod",
        "shield",
        "elytra",
        "shears",
        "flint_and_steel",
        "carrot_on_a_stick",
        "warped_fungus_on_a_stick",
        "brush",
        "mace",
        "spyglass",
        "saddle",
        "totem_of_undying",
        "potion",
        "splash_potion",
        "lingering_potion",
        "written_book",
        "writable_book",
        "enchanted_book",
        "filled_map",
        "bundle",
        "mushroom_stew",
        "rabbit_stew",
        "beetroot_soup",
        "suspicious_stew",
        "cake",
        "minecart",
        "shulker_box",
        "goat_horn",
    ];
    const SIXTEEN_NAMES: &[&str] = &["bucket", "ender_pearl", "snowball", "egg", "honey_bottle", "armor_stand"];
    const SIXTEEN_SUFFIXES: &[&str] = &["_sign", "_hanging_sign", "_banner"];

    if SINGLE_NAMES.contains(&name)
        || name.starts_with("music_disc_")
        || SINGLE_SUFFIXES.iter().any(|suffix| name.ends_with(suffix))
        || (name.ends_with("_bucket") && name != "bucket")
    {
        1
    } else if SIXTEEN_NAMES.contains(&name) || SIXTEEN_SUFFIXES.iter().any(|suffix| name.ends_with(suffix)) {
        16
    } else {
        64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_weapons_and_armour_do_not_stack() {
        for item in ["minecraft:diamond_sword", "minecraft:iron_pickaxe", "minecraft:netherite_helmet", "minecraft:bow", "minecraft:shield", "minecraft:elytra"] {
            assert_eq!(max_stack_size(item), 1, "{item}");
        }
    }

    #[test]
    fn unique_state_items_do_not_stack() {
        for item in ["minecraft:potion", "minecraft:splash_potion", "minecraft:water_bucket", "minecraft:red_bed", "minecraft:oak_boat", "minecraft:music_disc_13"] {
            assert_eq!(max_stack_size(item), 1, "{item}");
        }
    }

    #[test]
    fn some_items_stack_to_sixteen() {
        for item in ["minecraft:bucket", "minecraft:ender_pearl", "minecraft:snowball", "minecraft:egg", "minecraft:oak_sign", "minecraft:white_banner"] {
            assert_eq!(max_stack_size(item), 16, "{item}");
        }
    }

    #[test]
    fn everything_else_stacks_to_sixty_four() {
        for item in ["minecraft:golden_apple", "minecraft:arrow", "minecraft:cobblestone", "minecraft:diamond", "minecraft:enchanted_golden_apple", "minecraft:shulker_shell"] {
            assert_eq!(max_stack_size(item), 64, "{item}");
        }
    }
}
