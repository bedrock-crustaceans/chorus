use crate::block_id;
use crate::component::placement_filter_component::{PlacementFace, PlacementRule};

pub const AZALEA: &[PlacementRule] = &[PlacementRule {
    allowed_faces: &[PlacementFace::Up],
    block_filter: &[
        block_id::CLAY,
        block_id::COARSE_DIRT,
        block_id::DIRT,
        block_id::DIRT_WITH_ROOTS,
        block_id::FARMLAND,
        block_id::GRASS_BLOCK,
        block_id::MOSS_BLOCK,
        block_id::MUD,
        block_id::MUDDY_MANGROVE_ROOTS,
        block_id::MYCELIUM,
        block_id::PALE_MOSS_BLOCK,
        block_id::PODZOL,
    ],
}];

pub const DRY_VEGETATION: &[PlacementRule] = &[PlacementRule {
    allowed_faces: &[PlacementFace::Up],
    block_filter: &[
        block_id::BLACK_TERRACOTTA,
        block_id::BLUE_TERRACOTTA,
        block_id::BROWN_TERRACOTTA,
        block_id::COARSE_DIRT,
        block_id::CYAN_TERRACOTTA,
        block_id::DIRT,
        block_id::DIRT_WITH_ROOTS,
        block_id::FARMLAND,
        block_id::GRASS_BLOCK,
        block_id::GRAY_TERRACOTTA,
        block_id::GREEN_TERRACOTTA,
        block_id::HARDENED_CLAY,
        block_id::LIGHT_BLUE_TERRACOTTA,
        block_id::LIGHT_GRAY_TERRACOTTA,
        block_id::LIME_TERRACOTTA,
        block_id::MAGENTA_TERRACOTTA,
        block_id::MOSS_BLOCK,
        block_id::MUD,
        block_id::MUDDY_MANGROVE_ROOTS,
        block_id::MYCELIUM,
        block_id::ORANGE_TERRACOTTA,
        block_id::PALE_MOSS_BLOCK,
        block_id::PINK_TERRACOTTA,
        block_id::PODZOL,
        block_id::PURPLE_TERRACOTTA,
        block_id::RED_SAND,
        block_id::RED_TERRACOTTA,
        block_id::SAND,
        block_id::SUSPICIOUS_SAND,
        block_id::WHITE_TERRACOTTA,
        block_id::YELLOW_TERRACOTTA,
    ],
}];

pub const BAMBOO: &[PlacementRule] = &[PlacementRule {
    allowed_faces: &[PlacementFace::Up],
    block_filter: &[
        block_id::BAMBOO,
        block_id::BAMBOO_SAPLING,
        block_id::COARSE_DIRT,
        block_id::DIRT,
        block_id::DIRT_WITH_ROOTS,
        block_id::GRASS_BLOCK,
        block_id::GRAVEL,
        block_id::MOSS_BLOCK,
        block_id::MUD,
        block_id::MUDDY_MANGROVE_ROOTS,
        block_id::MYCELIUM,
        block_id::PALE_MOSS_BLOCK,
        block_id::PODZOL,
        block_id::RED_SAND,
        block_id::SAND,
        block_id::SUSPICIOUS_GRAVEL,
        block_id::SUSPICIOUS_SAND,
    ],
}];

pub const SMALL_DRIPLEAF: &[PlacementRule] = &[PlacementRule {
    allowed_faces: &[PlacementFace::Up],
    block_filter: &[block_id::CLAY, block_id::MOSS_BLOCK],
}];

pub const BIG_DRIPLEAF: &[PlacementRule] = &[PlacementRule {
    allowed_faces: &[PlacementFace::Up],
    block_filter: &[
        block_id::CLAY,
        block_id::COARSE_DIRT,
        block_id::DIRT,
        block_id::DIRT_WITH_ROOTS,
        block_id::FARMLAND,
        block_id::GRASS_BLOCK,
        block_id::MOSS_BLOCK,
        block_id::MUD,
        block_id::MUDDY_MANGROVE_ROOTS,
        block_id::MYCELIUM,
        block_id::PODZOL,
    ],
}];

pub const MANGROVE_PROPAGULE: &[PlacementRule] = &[
    PlacementRule {
        allowed_faces: &[PlacementFace::Up],
        block_filter: &[
            block_id::CLAY,
            block_id::COARSE_DIRT,
            block_id::DIRT,
            block_id::DIRT_WITH_ROOTS,
            block_id::FARMLAND,
            block_id::GRASS_BLOCK,
            block_id::MOSS_BLOCK,
            block_id::MUD,
            block_id::MUDDY_MANGROVE_ROOTS,
            block_id::MYCELIUM,
            block_id::PALE_MOSS_BLOCK,
            block_id::PODZOL,
        ],
    },
    PlacementRule {
        allowed_faces: &[PlacementFace::Down],
        block_filter: &[block_id::MANGROVE_LEAVES],
    },
];

pub const CACTUS: &[PlacementRule] = &[PlacementRule {
    allowed_faces: &[PlacementFace::Up],
    block_filter: &[block_id::RED_SAND, block_id::SAND, block_id::SUSPICIOUS_SAND],
}];

pub const SUGAR_CANE: &[PlacementRule] = &[PlacementRule {
    allowed_faces: &[PlacementFace::Up],
    block_filter: &[
        block_id::COARSE_DIRT,
        block_id::DIRT,
        block_id::DIRT_WITH_ROOTS,
        block_id::GRASS_BLOCK,
        block_id::MOSS_BLOCK,
        block_id::MUD,
        block_id::MUDDY_MANGROVE_ROOTS,
        block_id::MYCELIUM,
        block_id::PALE_MOSS_BLOCK,
        block_id::PODZOL,
        block_id::RED_SAND,
        block_id::SAND,
        block_id::SUSPICIOUS_SAND,
    ],
}];

pub const VEGETATION: &[PlacementRule] = &[PlacementRule {
    allowed_faces: &[PlacementFace::Up],
    block_filter: &[
        block_id::COARSE_DIRT,
        block_id::DIRT,
        block_id::DIRT_WITH_ROOTS,
        block_id::FARMLAND,
        block_id::GRASS_BLOCK,
        block_id::MOSS_BLOCK,
        block_id::MUD,
        block_id::MUDDY_MANGROVE_ROOTS,
        block_id::MYCELIUM,
        block_id::PALE_MOSS_BLOCK,
        block_id::PODZOL,
    ],
}];

pub const WATERLILY: &[PlacementRule] = &[PlacementRule {
    allowed_faces: &[PlacementFace::Up],
    block_filter: &[block_id::FROSTED_ICE, block_id::ICE, block_id::WATER],
}];

pub const KELP_DENIED: &[&str] = &[block_id::MAGMA];

pub const SEAGRASS_DENIED: &[&str] = &[block_id::MAGMA];

pub const SNOW_LAYER: &[PlacementRule] = &[PlacementRule {
    allowed_faces: &[PlacementFace::Up],
    block_filter: &[block_id::HONEY_BLOCK, block_id::MUD, block_id::SOUL_SAND],
}];

pub const SNOW_LAYER_DENIED: &[&str] = &[block_id::BARRIER, block_id::ICE, block_id::PACKED_ICE];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_registry::BlockRegistry;
    use crate::r#impl::DEFINITIONS;

    const RULES: &[&[PlacementRule]] = &[
        AZALEA,
        DRY_VEGETATION,
        BAMBOO,
        SMALL_DRIPLEAF,
        BIG_DRIPLEAF,
        MANGROVE_PROPAGULE,
        CACTUS,
        SUGAR_CANE,
        VEGETATION,
        WATERLILY,
        SNOW_LAYER,
    ];
    const DENIED: &[&[&str]] = &[KELP_DENIED, SEAGRASS_DENIED, SNOW_LAYER_DENIED];

    fn lists() -> Vec<&'static [&'static str]> {
        let filters = RULES.iter().flat_map(|rules| rules.iter()).map(|rule| rule.block_filter);
        filters.chain(DENIED.iter().copied()).collect()
    }

    #[test]
    fn every_listed_block_is_registered() {
        let mut registry = BlockRegistry::new();
        registry.register_all(DEFINITIONS.iter().copied());
        for identifier in lists().into_iter().flatten() {
            assert!(registry.get_block_id(identifier).is_some(), "{identifier}");
        }
    }

    #[test]
    fn no_list_repeats_a_block() {
        for list in lists() {
            let mut sorted = list.to_vec();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(sorted.len(), list.len(), "{list:?}");
        }
    }

    #[test]
    fn every_rule_names_a_face() {
        for rule in RULES.iter().flat_map(|rules| rules.iter()) {
            assert!(!rule.allowed_faces.is_empty());
            assert!(!rule.block_filter.is_empty());
        }
    }

    #[test]
    fn vegetation_grows_on_dirt_but_not_stone() {
        let rule = &VEGETATION[0];
        assert!(rule.allowed_faces.contains(&PlacementFace::Up));
        assert!(rule.block_filter.contains(&block_id::DIRT));
        assert!(!rule.block_filter.contains(&block_id::STONE));
    }

    #[test]
    fn a_propagule_also_hangs_from_leaves() {
        let hanging = MANGROVE_PROPAGULE.iter().find(|rule| rule.allowed_faces.contains(&PlacementFace::Down)).expect("a hanging rule");
        assert_eq!(hanging.block_filter, &[block_id::MANGROVE_LEAVES]);
    }
}
