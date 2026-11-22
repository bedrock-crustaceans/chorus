use super::placed::PlacedFeature;
use super::registry::FeatureRegistry;
use super::rules::{self, Feature};
use super::tests::{REGISTRY, load};
use super::world::read;
use crate::shared::biome::Biome;
use crate::shared::blocks::{BlockId, BlockTable, Blocks};
use crate::shared::chunk::biome_volume::{BiomeVolume, SAMPLE_SPACING, sample_count, sample_levels, sample_origin};
use crate::shared::chunk::proto::ProtoChunk;
use crate::shared::chunk::region::Region;
use crate::shared::rng::mersenne::MersenneTwister;
use chorus_block::block_id::*;
use glam::IVec3;
use std::sync::Arc;

const MIN_Y: i32 = -64;
const HEIGHT: i32 = 384;

fn table() -> BlockTable<'static> {
    BlockTable::new(&REGISTRY)
}

fn volume_world(blocks: &Blocks, water: BlockId, lava: BlockId, biome_at: impl Fn(i32) -> Biome) -> Region<'_> {
    let levels = sample_levels(HEIGHT);
    let samples: Vec<i32> = (0..sample_count(HEIGHT))
        .map(|index| biome_at(sample_origin(0, 0, MIN_Y).1 + (index % levels) as i32 * SAMPLE_SPACING).bedrock_id())
        .collect();
    let chunks = std::array::from_fn(|slot| {
        let volume = BiomeVolume::resolve(&samples, 0, slot as i32 / 3 - 1, slot as i32 % 3 - 1, MIN_Y, HEIGHT);
        ProtoChunk::with_biome_volume(slot as i32 / 3 - 1, slot as i32 % 3 - 1, MIN_Y, HEIGHT, water, lava, volume)
    });
    Region::new(0, 0, blocks, chunks)
}

fn world(blocks: &Blocks, water: BlockId, lava: BlockId) -> Region<'_> {
    volume_world(blocks, water, lava, |_| Biome::PLAINS)
}

#[test]
fn rules_that_run_are_supported_end_to_end() {
    let mut table = table();
    let loaded = load(&mut table);
    let runnable: Vec<&str> = loaded
        .rules
        .values()
        .filter(|rule| rule.runs() && matches!(rule.feature, Feature::Placed(_)))
        .map(|rule| rule.identifier)
        .collect();
    for name in [
        "minecraft:desert_after_surface_desert_well_feature",
        "minecraft:overworld_amethyst_geode_feature",
        "minecraft:desert_after_surface_cactus_feature_rules",
        "minecraft:jungle_after_surface_bamboo_feature",
        "minecraft:swamp_after_surface_flowers_feature_rules",
        "minecraft:sulfur_caves_surface_sulfur_pool_with_potent_sulfur_feature_rules",
    ] {
        assert!(runnable.contains(&name), "{name}");
    }
    assert!(runnable.contains(&"minecraft:plains_after_surface_flowers_feature_rules"));
    assert!(runnable.contains(&"minecraft:flower_forest_after_surface_flowers_feature_rules"));
}

fn layered_world(blocks: &Blocks, water: BlockId, lava: BlockId, lower: Biome, upper: Biome) -> Region<'_> {
    volume_world(blocks, water, lava, |y| if y < 0 { lower.clone() } else { upper.clone() })
}

#[test]
fn biomes_are_listed_bottom_up_in_first_seen_order() {
    let mut table = table();
    let (water, lava) = (table.name(WATER), table.name(LAVA));
    let blocks = table.finish();
    let region = layered_world(&blocks, water, lava, Biome::DESERT, Biome::PLAINS);
    assert_eq!(super::biome_order(&region, 0, 0), [Biome::DESERT.bedrock_id(), Biome::PLAINS.bedrock_id()]);
    let region = layered_world(&blocks, water, lava, Biome::PLAINS, Biome::DESERT);
    assert_eq!(super::biome_order(&region, 0, 0), [Biome::PLAINS.bedrock_id(), Biome::DESERT.bedrock_id()]);
    let region = world(&blocks, water, lava);
    assert_eq!(super::biome_order(&region, 0, 0), [Biome::PLAINS.bedrock_id()]);
}

#[derive(Debug)]
struct FillFeature {
    block: BlockId,
}

impl PlacedFeature for FillFeature {
    fn run(&self, region: &mut Region, _random: &mut MersenneTwister, pos: IVec3) -> crate::shared::structure::Probed<bool> {
        Ok(region.set(pos, self.block))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

const FILL_DOCUMENTS: &str = r#"{
    "test:fill_stone": {
        "format_version": "1.0.0",
        "test:fill_feature": {"description": {"identifier": "test:fill_stone"}, "block": "minecraft:stone"}
    }
}"#;

#[test]
fn custom_feature_types_load_from_extra_documents() {
    let mut registry = FeatureRegistry::vanilla().with_documents(FILL_DOCUMENTS);
    registry.register("test:fill_feature", |loader, doc| {
        let name = doc.body["block"].as_str().ok_or(doc.blocked())?;
        Ok(super::rules::arc(FillFeature { block: loader.table().name(name) }))
    });
    let mut table = table();
    let stone = table.name(STONE);
    let water = table.name(WATER);
    let lava = table.name(LAVA);
    let loaded = rules::load_with_registry(&mut table, &[Arc::from("test:fill_stone")], &registry);
    let feature = loaded.features.get("test:fill_stone").expect("custom feature loads");
    assert!(feature.as_any().is::<FillFeature>());
    let blocks = table.finish();
    let mut region = world(&blocks, water, lava);
    let at = IVec3::new(16, 60, 16);
    assert_ne!(read(&region, at), stone);
    assert!(feature.place(&mut region, &mut MersenneTwister::new(1), at).unwrap());
    assert_eq!(read(&region, at), stone);
}

#[test]
fn unregistered_custom_feature_types_stay_blocked() {
    let registry = FeatureRegistry::vanilla().with_documents(FILL_DOCUMENTS);
    let mut table = table();
    let loaded = rules::load_with_registry(&mut table, &[Arc::from("test:fill_stone")], &registry);
    assert!(!loaded.features.contains_key("test:fill_stone"));
}
