use crate::bedrock::overworld::noise::NormalNoise;
use crate::shared::biome::Biome;
use crate::shared::biome_documents::ROOT;
use crate::shared::blocks::{AIR, BlockId, BlockTable};
use crate::shared::rng::xoroshiro::PositionalFactory;
use chorus_block::block_id::{COARSE_DIRT, DIRT, HARDENED_CLAY, STONE, WATER};
use rustc_hash::FxHashMap;
use serde_json::Value;

const NAME: &str = "name";
const STATES: &str = "states";
const DIRT_TYPE: &str = "dirt_type";
const COARSE: &str = "coarse";
const SURFACE_BUILDER: &str = "minecraft:surface_builder";
pub(super) const MATERIAL_ADJUSTMENTS: &str = "minecraft:surface_material_adjustments";
const MOUNTAIN_PARAMETERS: &str = "minecraft:mountain_parameters";
const PARTIALLY_FROZEN: &str = "minecraft:partially_frozen";
const CLIMATE: &str = "minecraft:climate";
const OVERWORLD_BUILDER: &str = "minecraft:overworld";
const SWAMP_BUILDER: &str = "minecraft:swamp";
const FROZEN_OCEAN_BUILDER: &str = "minecraft:frozen_ocean";
const MESA_BUILDER: &str = "minecraft:mesa";
const NOISE_GRADIENT_BUILDER: &str = "minecraft:noise_gradient";

#[derive(Clone, Copy)]
pub struct Materials {
    pub top: BlockId,
    pub mid: BlockId,
    pub sea_floor: BlockId,
    pub foundation: BlockId,
    pub sea: BlockId,
}

pub struct Adjustment {
    pub scale: f32,
    pub low: f32,
    pub high: f32,
    top: Option<BlockId>,
    mid: Option<BlockId>,
    sea_floor: Option<BlockId>,
    foundation: Option<BlockId>,
    sea: Option<BlockId>,
}

impl Adjustment {
    pub fn apply(&self, materials: &mut Materials) {
        materials.top = self.top.unwrap_or(materials.top);
        materials.mid = self.mid.unwrap_or(materials.mid);
        materials.sea_floor = self.sea_floor.unwrap_or(materials.sea_floor);
        materials.foundation = self.foundation.unwrap_or(materials.foundation);
        materials.sea = self.sea.unwrap_or(materials.sea);
    }
}

pub struct Steep {
    pub block: BlockId,
    pub north: bool,
    pub south: bool,
    pub east: bool,
    pub west: bool,
}

pub struct MesaSettings {
    pub bryce_pillars: bool,
    pub has_forest: bool,
    pub hard_clay: BlockId,
}

pub struct Specifier {
    pub low: f32,
    pub high: f32,
    pub block: BlockId,
}

pub struct GradientSettings {
    pub noise: NormalNoise,
    pub specifiers: Vec<Specifier>,
    pub non_replaceable: Vec<BlockId>,
}

pub enum Builder {
    Overworld,
    Swamp { puddle_depth: i32 },
    FrozenOcean,
    Mesa(MesaSettings),
    NoiseGradient(GradientSettings),
}

pub struct BiomeSurface {
    pub temperature: f32,
    pub partially_frozen: bool,
    pub builder: Builder,
    pub materials: Materials,
    pub sea_floor_depth: i32,
    pub steep: Option<Steep>,
    pub adjustments: Vec<Adjustment>,
}

pub struct SurfaceData {
    biomes: FxHashMap<Biome, BiomeSurface>,
}

pub fn climate(biome: &Biome) -> Option<(f32, bool)> {
    let root: &'static Value = &ROOT;
    let components = &root.get(biome.pack_name())?["components"];
    Some((number(components[CLIMATE].get("temperature")), components.get(PARTIALLY_FROZEN).is_some()))
}

pub fn tags(biome: &Biome) -> impl Iterator<Item = &'static str> {
    crate::shared::biome_documents::tags(biome.pack_name())
}

pub(super) fn material_name(value: &'static Value) -> Option<&'static str> {
    let name = value.as_str().or_else(|| value.get(NAME)?.as_str())?;
    if name == DIRT && value.get(STATES).and_then(|states| states.get(DIRT_TYPE)).and_then(Value::as_str) == Some(COARSE) {
        return Some(COARSE_DIRT);
    }
    Some(name)
}

fn block(table: &mut BlockTable, value: Option<&'static Value>, default: BlockId) -> BlockId {
    value.and_then(material_name).map_or(default, |name| table.name(name))
}

fn optional_block(table: &mut BlockTable, value: Option<&'static Value>) -> Option<BlockId> {
    value.and_then(material_name).map(|name| table.name(name))
}

fn flag(value: Option<&Value>) -> bool {
    value.and_then(Value::as_bool).unwrap_or(false)
}

fn number(value: Option<&Value>) -> f32 {
    value.and_then(Value::as_f64).expect("an embedded vanilla biome document is missing a number") as f32
}

fn parse_adjustments(table: &mut BlockTable, components: &'static Value) -> Vec<Adjustment> {
    let Some(entries) = components.get(MATERIAL_ADJUSTMENTS).and_then(|adjustments| adjustments["adjustments"].as_array()) else {
        return Vec::new();
    };
    entries
        .iter()
        .filter(|entry| entry.get("height_range").is_none())
        .map(|entry| {
            let materials = &entry["materials"];
            Adjustment {
                scale: number(entry.get("noise_frequency_scale")),
                low: number(entry["noise_range"].get(0)),
                high: number(entry["noise_range"].get(1)),
                top: optional_block(table, materials.get("top_material")),
                mid: optional_block(table, materials.get("mid_material")),
                sea_floor: optional_block(table, materials.get("sea_floor_material")),
                foundation: optional_block(table, materials.get("foundation_material")),
                sea: optional_block(table, materials.get("sea_material")),
            }
        })
        .collect()
}

fn parse_steep(table: &mut BlockTable, components: &'static Value) -> Option<Steep> {
    let steep = components.get(MOUNTAIN_PARAMETERS)?.get("steep_material_adjustment")?;
    Some(Steep {
        block: block(table, steep.get("material"), AIR),
        north: flag(steep.get("north_slopes")),
        south: flag(steep.get("south_slopes")),
        east: flag(steep.get("east_slopes")),
        west: flag(steep.get("west_slopes")),
    })
}

fn parse_gradient(table: &mut BlockTable, factory: &PositionalFactory, builder: &'static Value) -> GradientSettings {
    let descriptor = &builder["noise_descriptor"];
    let amplitudes: Vec<f64> = descriptor["amplitudes"]
        .as_array()
        .expect("an embedded vanilla noise_descriptor has no amplitudes list")
        .iter()
        .map(|amplitude| amplitude.as_f64().expect("an embedded vanilla noise amplitude is not a number"))
        .collect();
    let name = descriptor["name"].as_str().expect("an embedded vanilla noise_descriptor has no name");
    let first_octave = descriptor["first_octave"].as_i64().expect("an embedded vanilla noise_descriptor has no first_octave") as i32;
    let specifiers = builder["noise_block_specifiers"]
        .as_array()
        .expect("an embedded vanilla gradient has no noise_block_specifiers list")
        .iter()
        .map(|specifier| Specifier {
            low: number(specifier["range"].get("min")),
            high: number(specifier["range"].get("max")),
            block: block(table, specifier.get("block"), AIR),
        })
        .collect();
    let non_replaceable = builder["non_replaceable_blocks"]
        .as_array()
        .map(|blocks| blocks.iter().map(|value| block(table, Some(value), AIR)).collect())
        .unwrap_or_default();
    GradientSettings {
        noise: NormalNoise::new(&mut factory.hash_of(name), first_octave, &amplitudes),
        specifiers,
        non_replaceable,
    }
}

fn parse_biome(table: &mut BlockTable, factory: &PositionalFactory, entry: &'static Value) -> Option<BiomeSurface> {
    let components = &entry["components"];
    let builder = components.get(SURFACE_BUILDER)?.get("builder")?;
    let hardened_clay = table.name(HARDENED_CLAY);
    let stone = table.name(STONE);
    let water = table.name(WATER);
    let kind = match builder["type"].as_str()? {
        OVERWORLD_BUILDER => Builder::Overworld,
        SWAMP_BUILDER => Builder::Swamp {
            puddle_depth: builder["max_puddle_depth_below_sea_level"].as_i64().unwrap_or(0) as i32,
        },
        FROZEN_OCEAN_BUILDER => Builder::FrozenOcean,
        MESA_BUILDER => Builder::Mesa(MesaSettings {
            bryce_pillars: flag(builder.get("bryce_pillars")),
            has_forest: flag(builder.get("has_forest")),
            hard_clay: block(table, builder.get("hard_clay_material"), hardened_clay),
        }),
        NOISE_GRADIENT_BUILDER => Builder::NoiseGradient(parse_gradient(table, factory, builder)),
        _ => return None,
    };
    Some(BiomeSurface {
        temperature: number(components[CLIMATE].get("temperature")),
        partially_frozen: components.get(PARTIALLY_FROZEN).is_some(),
        builder: kind,
        materials: Materials {
            top: block(table, builder.get("top_material"), AIR),
            mid: block(table, builder.get("mid_material"), AIR),
            sea_floor: block(table, builder.get("sea_floor_material"), AIR),
            foundation: block(table, builder.get("foundation_material"), stone),
            sea: block(table, builder.get("sea_material"), water),
        },
        sea_floor_depth: builder["sea_floor_depth"].as_i64().unwrap_or(0) as i32,
        steep: parse_steep(table, components),
        adjustments: parse_adjustments(table, components),
    })
}

impl SurfaceData {
    pub fn parse(table: &mut BlockTable, factory: &PositionalFactory) -> Self {
        let root: &'static Value = &ROOT;
        let biomes = Biome::ALL
            .into_iter()
            .filter_map(|biome| {
                let surface = root.get(biome.pack_name()).and_then(|entry| parse_biome(table, factory, entry))?;
                Some((biome, surface))
            })
            .collect();
        Self { biomes }
    }

    pub fn get(&self, biome: &Biome) -> Option<&BiomeSurface> {
        self.biomes.get(biome)
    }
}
