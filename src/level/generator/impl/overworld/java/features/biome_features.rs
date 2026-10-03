use super::super::biome::Biome;
use super::key::Key::{self, *};

pub const STEP_COUNT: usize = 11;

// variant order feeds the decoration seed, so unused steps keep their slot
#[derive(Clone, Copy)]
#[allow(dead_code)]
pub enum Step {
    RawGeneration,
    Lakes,
    LocalModifications,
    UndergroundStructures,
    SurfaceStructures,
    Strongholds,
    UndergroundOres,
    UndergroundDecoration,
    FluidSprings,
    VegetalDecoration,
    TopLayerModification,
}

use Step::*;

#[derive(Default)]
struct Builder {
    steps: [Vec<Key>; STEP_COUNT],
}

impl Builder {
    fn add(&mut self, step: Step, key: Key) -> &mut Self {
        self.steps[step as usize].push(key);
        self
    }

    fn default_carvers_and_lakes(&mut self) {
        self.add(Lakes, LakeLavaUnderground).add(Lakes, LakeLavaSurface);
    }

    fn default_monster_room(&mut self) {
        self.add(UndergroundStructures, MonsterRoom).add(UndergroundStructures, MonsterRoomDeep);
    }

    fn default_underground_variety(&mut self) {
        for key in [
            OreDirt,
            OreGravel,
            OreGraniteUpper,
            OreGraniteLower,
            OreDioriteUpper,
            OreDioriteLower,
            OreAndesiteUpper,
            OreAndesiteLower,
            OreTuff,
        ] {
            self.add(UndergroundOres, key);
        }
        self.add(VegetalDecoration, GlowLichen);
    }

    fn dripstone(&mut self) {
        self.add(LocalModifications, LargeDripstone)
            .add(UndergroundDecoration, DripstoneCluster)
            .add(UndergroundDecoration, PointedDripstone);
    }

    fn sculk(&mut self) {
        self.add(UndergroundDecoration, SculkVein).add(UndergroundDecoration, SculkPatchDeepDark);
    }

    fn default_ores(&mut self, large_copper_blobs: bool) {
        for key in [
            OreCoalUpper,
            OreCoalLower,
            OreIronUpper,
            OreIronMiddle,
            OreIronSmall,
            OreGold,
            OreGoldLower,
            OreRedstone,
            OreRedstoneLower,
            OreDiamond,
            OreDiamondMedium,
            OreDiamondLarge,
            OreDiamondBuried,
            OreLapis,
            OreLapisBuried,
        ] {
            self.add(UndergroundOres, key);
        }
        self.add(UndergroundOres, if large_copper_blobs { OreCopperLarge } else { OreCopper });
        self.add(UndergroundOres, UnderwaterMagma);
    }

    fn extra_gold(&mut self) {
        self.add(UndergroundOres, OreGoldExtra);
    }

    fn extra_emeralds(&mut self) {
        self.add(UndergroundOres, OreEmerald);
    }

    fn infested_stone(&mut self) {
        self.add(UndergroundDecoration, OreInfested);
    }

    fn default_soft_disks(&mut self) {
        self.add(UndergroundOres, DiskSand).add(UndergroundOres, DiskClay).add(UndergroundOres, DiskGravel);
    }

    fn swamp_clay_disk(&mut self) {
        self.add(UndergroundOres, DiskClay);
    }

    fn mangrove_swamp_disks(&mut self) {
        self.add(UndergroundOres, DiskGrass).add(UndergroundOres, DiskClay);
    }

    fn vegetal(&mut self, keys: &[Key]) {
        for &key in keys {
            self.add(VegetalDecoration, key);
        }
    }

    fn default_flowers(&mut self) {
        self.vegetal(&[FlowerDefault]);
    }

    fn default_grass(&mut self) {
        self.vegetal(&[PatchGrassBadlands]);
    }

    fn default_mushrooms(&mut self) {
        self.vegetal(&[BrownMushroomNormal, RedMushroomNormal]);
    }

    fn plain_grass(&mut self) {
        self.vegetal(&[PatchTallGrass2]);
    }

    fn plain_vegetation(&mut self) {
        self.vegetal(&[TreesPlains, FlowerPlains, PatchGrassPlain]);
    }

    fn near_water_vegetation(&mut self) {
        self.vegetal(&[PatchSugarCane, PatchFireflyBushNearWater]);
    }

    fn default_extra_vegetation(&mut self, near_water: bool) {
        self.vegetal(&[PatchPumpkin]);
        if near_water {
            self.near_water_vegetation();
        }
    }

    fn default_springs(&mut self) {
        self.add(FluidSprings, SpringWater).add(FluidSprings, SpringLava);
    }

    fn global_overworld_generation(&mut self) {
        self.default_carvers_and_lakes();
        self.add(LocalModifications, AmethystGeode);
        self.default_monster_room();
        self.default_underground_variety();
        self.default_springs();
        self.add(TopLayerModification, FreezeTopLayer);
    }

    fn fossils(&mut self) {
        self.add(UndergroundStructures, FossilUpper).add(UndergroundStructures, FossilLower);
    }

    fn base_ocean(&mut self) {
        self.global_overworld_generation();
        self.default_ores(false);
        self.default_soft_disks();
        self.vegetal(&[TreesWater]);
        self.default_flowers();
        self.default_grass();
        self.default_mushrooms();
        self.default_extra_vegetation(true);
    }

    fn peaks(&mut self) {
        self.global_overworld_generation();
        self.add(FluidSprings, SpringLavaFrozen);
        self.default_ores(false);
        self.default_soft_disks();
        self.extra_emeralds();
        self.infested_stone();
    }
}

fn plains(sunflower: bool, snowy: bool, spikes: bool) -> Builder {
    let mut b = Builder::default();
    b.global_overworld_generation();
    if snowy {
        if spikes {
            b.add(SurfaceStructures, IceSpike).add(SurfaceStructures, IcePatch);
        }
    } else {
        b.plain_grass();
        b.vegetal(&[if sunflower { PatchSunflower } else { PatchBush }]);
    }
    b.default_ores(false);
    b.default_soft_disks();
    if snowy {
        b.vegetal(&[TreesSnowy]);
        b.default_flowers();
        b.default_grass();
    } else {
        b.plain_vegetation();
    }
    b.default_mushrooms();
    b.default_extra_vegetation(true);
    b
}

fn forest(birch: bool, tall: bool, flower: bool) -> Builder {
    let mut b = Builder::default();
    b.global_overworld_generation();
    b.vegetal(&[if flower { FlowerForestFlowers } else { ForestFlowers }]);
    b.default_ores(false);
    b.default_soft_disks();
    if flower {
        b.vegetal(&[TreesFlowerForest, FlowerFlowerForest]);
        b.default_grass();
    } else {
        if birch {
            b.vegetal(&[WildflowersBirchForest]);
            b.vegetal(&[if tall { BirchTall } else { TreesBirch }]);
        } else {
            b.vegetal(&[TreesBirchAndOakLeafLitter]);
        }
        b.vegetal(&[PatchBush]);
        b.default_flowers();
        b.vegetal(&[PatchGrassForest]);
    }
    b.default_mushrooms();
    b.default_extra_vegetation(true);
    b
}

fn old_growth_taiga(spruce: bool) -> Builder {
    let mut b = Builder::default();
    b.global_overworld_generation();
    b.add(LocalModifications, ForestRock);
    b.vegetal(&[PatchLargeFern]);
    b.default_ores(false);
    b.default_soft_disks();
    b.vegetal(&[if spruce { TreesOldGrowthSpruceTaiga } else { TreesOldGrowthPineTaiga }]);
    b.default_flowers();
    b.vegetal(&[PatchGrassTaiga, PatchDeadBush, BrownMushroomOldGrowth, RedMushroomOldGrowth]);
    b.default_mushrooms();
    b.default_extra_vegetation(true);
    b.vegetal(&[PatchBerryCommon]);
    b
}

fn jungle(bamboo: bool, sparse: bool, core: bool) -> Builder {
    let mut b = Builder::default();
    b.global_overworld_generation();
    b.default_ores(false);
    b.default_soft_disks();
    if bamboo {
        b.vegetal(&[Bamboo, BambooVegetation]);
    } else {
        if core {
            b.vegetal(&[BambooLight]);
        }
        b.vegetal(&[if sparse { TreesSparseJungle } else { TreesJungle }]);
    }
    b.vegetal(&[FlowerWarm, PatchGrassJungle]);
    b.default_mushrooms();
    b.default_extra_vegetation(true);
    b.vegetal(&[Vines, if sparse { PatchMelonSparse } else { PatchMelon }]);
    b
}

fn windswept_hills(more_trees: bool) -> Builder {
    let mut b = Builder::default();
    b.global_overworld_generation();
    b.default_ores(false);
    b.default_soft_disks();
    b.vegetal(&[if more_trees { TreesWindsweptForest } else { TreesWindsweptHills }, PatchBush]);
    b.default_flowers();
    b.default_grass();
    b.default_mushrooms();
    b.default_extra_vegetation(true);
    b.extra_emeralds();
    b.infested_stone();
    b
}

fn desert() -> Builder {
    let mut b = Builder::default();
    b.fossils();
    b.global_overworld_generation();
    b.default_ores(false);
    b.default_soft_disks();
    b.default_flowers();
    b.default_grass();
    b.vegetal(&[PatchDryGrassDesert, PatchDeadBush2]);
    b.default_mushrooms();
    b.vegetal(&[PatchSugarCaneDesert, PatchPumpkin, PatchCactusDesert]);
    b.add(SurfaceStructures, DesertWell);
    b
}

fn savanna(shattered: bool) -> Builder {
    let mut b = Builder::default();
    b.global_overworld_generation();
    if !shattered {
        b.vegetal(&[PatchTallGrass]);
    }
    b.default_ores(false);
    b.default_soft_disks();
    if shattered {
        b.vegetal(&[TreesWindsweptSavanna]);
        b.default_flowers();
        b.vegetal(&[PatchGrassNormal]);
    } else {
        b.vegetal(&[TreesSavanna, FlowerWarm, PatchGrassSavanna]);
    }
    b.default_mushrooms();
    b.default_extra_vegetation(true);
    b
}

fn badlands(wooded: bool) -> Builder {
    let mut b = Builder::default();
    b.global_overworld_generation();
    b.default_ores(false);
    b.extra_gold();
    b.default_soft_disks();
    if wooded {
        b.vegetal(&[TreesBadlands]);
    }
    b.vegetal(&[PatchGrassBadlands, PatchDryGrassBadlands, PatchDeadBushBadlands]);
    b.default_mushrooms();
    b.vegetal(&[PatchSugarCaneBadlands, PatchPumpkin, PatchCactusDecorated, PatchFireflyBushNearWater]);
    b
}

fn ocean(seagrass: Key, extra: &[Key]) -> Builder {
    let mut b = Builder::default();
    b.base_ocean();
    b.vegetal(&[seagrass]);
    b.vegetal(extra);
    b
}

fn frozen_ocean() -> Builder {
    let mut b = Builder::default();
    b.add(LocalModifications, IcebergPacked).add(LocalModifications, IcebergBlue);
    b.global_overworld_generation();
    b.add(SurfaceStructures, BlueIce);
    b.default_ores(false);
    b.default_soft_disks();
    b.vegetal(&[TreesWater]);
    b.default_flowers();
    b.default_grass();
    b.default_mushrooms();
    b.default_extra_vegetation(true);
    b
}

fn taiga(snowy: bool) -> Builder {
    let mut b = Builder::default();
    b.global_overworld_generation();
    b.vegetal(&[PatchLargeFern]);
    b.default_ores(false);
    b.default_soft_disks();
    b.vegetal(&[TreesTaiga]);
    b.default_flowers();
    b.vegetal(&[PatchGrassTaiga2, BrownMushroomTaiga, RedMushroomTaiga]);
    b.default_extra_vegetation(true);
    b.vegetal(&[if snowy { PatchBerryRare } else { PatchBerryCommon }]);
    b
}

fn dark_forest(pale_garden: bool) -> Builder {
    let mut b = Builder::default();
    b.global_overworld_generation();
    b.vegetal(&[if pale_garden { PaleGardenVegetation } else { DarkForestVegetation }]);
    if pale_garden {
        b.vegetal(&[PaleMossPatch, PaleGardenFlowers]);
    } else {
        b.vegetal(&[ForestFlowers]);
    }
    b.default_ores(false);
    b.default_soft_disks();
    if pale_garden {
        b.vegetal(&[FlowerPaleGarden]);
    } else {
        b.default_flowers();
    }
    b.vegetal(&[PatchGrassForest]);
    if !pale_garden {
        b.default_mushrooms();
        b.vegetal(&[PatchLeafLitter]);
    }
    b.default_extra_vegetation(true);
    b
}

fn swamp() -> Builder {
    let mut b = Builder::default();
    b.fossils();
    b.global_overworld_generation();
    b.default_ores(false);
    b.swamp_clay_disk();
    b.vegetal(&[TreesSwamp, FlowerSwamp, PatchGrassNormal, PatchDeadBush, PatchWaterlily, BrownMushroomSwamp, RedMushroomSwamp]);
    b.default_mushrooms();
    b.vegetal(&[PatchSugarCaneSwamp, PatchPumpkin, PatchFireflyBushSwamp, PatchFireflyBushNearWaterSwamp, SeagrassSwamp]);
    b
}

fn mangrove_swamp() -> Builder {
    let mut b = Builder::default();
    b.fossils();
    b.global_overworld_generation();
    b.default_ores(false);
    b.mangrove_swamp_disks();
    b.vegetal(&[TreesMangrove, PatchGrassNormal, PatchDeadBush, PatchWaterlily, SeagrassSwamp, PatchFireflyBushNearWater]);
    b
}

fn river(frozen: bool) -> Builder {
    let mut b = Builder::default();
    b.global_overworld_generation();
    b.default_ores(false);
    b.default_soft_disks();
    b.vegetal(&[TreesWater, PatchBush]);
    b.default_flowers();
    b.default_grass();
    b.default_mushrooms();
    b.default_extra_vegetation(true);
    if !frozen {
        b.vegetal(&[SeagrassRiver]);
    }
    b
}

fn beach() -> Builder {
    let mut b = Builder::default();
    b.global_overworld_generation();
    b.default_ores(false);
    b.default_soft_disks();
    b.default_flowers();
    b.default_grass();
    b.default_mushrooms();
    b.default_extra_vegetation(true);
    b
}

fn meadow_or_cherry_grove(cherry_grove: bool) -> Builder {
    let mut b = Builder::default();
    b.global_overworld_generation();
    b.plain_grass();
    b.default_ores(false);
    b.default_soft_disks();
    if cherry_grove {
        b.vegetal(&[PatchGrassPlain, FlowerCherry, TreesCherry]);
    } else {
        b.vegetal(&[PatchGrassMeadow, FlowerMeadow, TreesMeadow, WildflowersMeadow]);
    }
    b.extra_emeralds();
    b.infested_stone();
    b
}

fn dappled_forest() -> Builder {
    let mut b = Builder::default();
    b.global_overworld_generation();
    b.default_ores(false);
    b.default_soft_disks();
    b.vegetal(&[TreesDappledForest, BrownMushroomDappledForest, PatchRedShrub, PatchGrassForest]);
    b
}

fn stony_peaks() -> Builder {
    let mut b = Builder::default();
    b.global_overworld_generation();
    b.default_ores(false);
    b.default_soft_disks();
    b.extra_emeralds();
    b.infested_stone();
    b
}

fn snowy_slopes_or_grove(grove: bool) -> Builder {
    let mut b = Builder::default();
    b.global_overworld_generation();
    b.add(FluidSprings, SpringLavaFrozen);
    b.default_ores(false);
    b.default_soft_disks();
    if grove {
        b.vegetal(&[TreesGrove]);
    }
    b.default_extra_vegetation(false);
    b.extra_emeralds();
    b.infested_stone();
    b
}

fn sulfur_caves() -> Builder {
    let mut b = Builder::default();
    b.global_overworld_generation();
    b.plain_grass();
    b.default_ores(false);
    b.default_soft_disks();
    b.add(Lakes, RootedSulfurSpring)
        .add(Lakes, SulfurPool)
        .add(UndergroundDecoration, SulfurSpikeCluster)
        .add(UndergroundDecoration, SulfurSpike);
    b
}

fn lush_caves() -> Builder {
    let mut b = Builder::default();
    b.global_overworld_generation();
    b.plain_grass();
    b.default_ores(false);
    b.add(UndergroundOres, OreClay);
    b.default_soft_disks();
    b.vegetal(&[LushCavesCeilingVegetation, CaveVines, LushCavesClay, LushCavesVegetation, RootedAzaleaTree, SporeBlossom, ClassicVines]);
    b
}

fn dripstone_caves() -> Builder {
    let mut b = Builder::default();
    b.global_overworld_generation();
    b.plain_grass();
    b.default_ores(true);
    b.default_soft_disks();
    b.plain_vegetation();
    b.default_mushrooms();
    b.default_extra_vegetation(false);
    b.dripstone();
    b
}

fn deep_dark() -> Builder {
    let mut b = Builder::default();
    b.add(LocalModifications, AmethystGeode);
    b.default_monster_room();
    b.default_underground_variety();
    b.add(TopLayerModification, FreezeTopLayer);
    b.plain_grass();
    b.default_ores(false);
    b.default_soft_disks();
    b.plain_vegetation();
    b.default_mushrooms();
    b.default_extra_vegetation(false);
    b.sculk();
    b
}

fn mushroom_fields() -> Builder {
    let mut b = Builder::default();
    b.global_overworld_generation();
    b.default_ores(false);
    b.default_soft_disks();
    b.vegetal(&[MushroomIslandVegetation, BrownMushroomTaiga, RedMushroomTaiga]);
    b.near_water_vegetation();
    b
}

pub fn features(biome: Biome) -> [Vec<Key>; STEP_COUNT] {
    use Biome as B;
    let builder = match biome {
        B::Plains => plains(false, false, false),
        B::SunflowerPlains => plains(true, false, false),
        B::SnowyPlains => plains(false, true, false),
        B::IceSpikes => plains(false, true, true),
        B::Desert => desert(),
        B::Swamp => swamp(),
        B::MangroveSwamp => mangrove_swamp(),
        B::Forest => forest(false, false, false),
        B::FlowerForest => forest(false, false, true),
        B::BirchForest => forest(true, false, false),
        B::OldGrowthBirchForest => forest(true, true, false),
        B::DappledForest => dappled_forest(),
        B::DarkForest => dark_forest(false),
        B::PaleGarden => dark_forest(true),
        B::OldGrowthPineTaiga => old_growth_taiga(false),
        B::OldGrowthSpruceTaiga => old_growth_taiga(true),
        B::Taiga => taiga(false),
        B::SnowyTaiga => taiga(true),
        B::Savanna | B::SavannaPlateau => savanna(false),
        B::WindsweptSavanna => savanna(true),
        B::WindsweptHills | B::WindsweptGravellyHills => windswept_hills(false),
        B::WindsweptForest => windswept_hills(true),
        B::Jungle => jungle(false, false, true),
        B::SparseJungle => jungle(false, true, false),
        B::BambooJungle => jungle(true, false, true),
        B::Badlands | B::ErodedBadlands => badlands(false),
        B::WoodedBadlands => badlands(true),
        B::Meadow => meadow_or_cherry_grove(false),
        B::CherryGrove => meadow_or_cherry_grove(true),
        B::Grove => snowy_slopes_or_grove(true),
        B::SnowySlopes => snowy_slopes_or_grove(false),
        B::FrozenPeaks | B::JaggedPeaks => {
            let mut b = Builder::default();
            b.peaks();
            b
        }
        B::StonyPeaks => stony_peaks(),
        B::River => river(false),
        B::FrozenRiver => river(true),
        B::Beach | B::SnowyBeach | B::StonyShore => beach(),
        B::WarmOcean => ocean(WarmOceanVegetation, &[SeagrassWarm, SeaPickle]),
        B::LukewarmOcean => ocean(SeagrassWarm, &[KelpWarm]),
        B::DeepLukewarmOcean => ocean(SeagrassDeepWarm, &[KelpWarm]),
        B::Ocean => ocean(SeagrassNormal, &[KelpCold]),
        B::DeepOcean => ocean(SeagrassDeep, &[KelpCold]),
        B::ColdOcean => ocean(SeagrassCold, &[KelpCold]),
        B::DeepColdOcean => ocean(SeagrassDeepCold, &[KelpCold]),
        B::FrozenOcean | B::DeepFrozenOcean => frozen_ocean(),
        B::MushroomFields => mushroom_fields(),
        B::DripstoneCaves => dripstone_caves(),
        B::LushCaves => lush_caves(),
        B::DeepDark => deep_dark(),
        B::SulfurCaves => sulfur_caves(),
    };
    builder.steps
}
