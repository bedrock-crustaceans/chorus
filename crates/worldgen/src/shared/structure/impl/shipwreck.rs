use crate::shared::blocks::Blocks;
use crate::shared::chunk::proto::ProtoChunk;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::Probed;
use crate::shared::structure::builder::{Built, StructureBuilder, StructureContext, place_plan};
use crate::shared::structure::place::{Library, Options, Rotation, Wood, markers_at};
use crate::shared::structure::plan::Plan;
use crate::shared::structure::plan::PlanWrites;
use crate::shared::structure::probe::StartProbe;
use crate::shared::structure::{Reaching, StructureKind};
use chorus_structure_data::StatePoolBuilder;
use std::sync::Arc;

pub const BEACHED_TEMPLATES: i32 = 14;
pub const TEMPLATE_COUNT: i32 = 20;
pub const WOOD_CHOICES: i32 = 9;
const SEA_PROBE_DEPTH: i32 = 32;
const DEPTH_BEACHED: i32 = -2;
const DEPTH_SUNK: i32 = -4;
const CHEST_REMOVAL_ROLL: i32 = 10;
const CHEST_REMOVAL_BELOW: i32 = 5;
const OAK: usize = 0;
const DARK_OAK: usize = 5;

pub const TEMPLATES: [&str; 20] = [
    "swwithmast",
    "swwithmastdegraded",
    "swsidewaysfull",
    "swsidewaysfronthalf",
    "swsidewaysbackhalf",
    "swsidewaysfulldegraded",
    "swsidewaysfronthalfdegraded",
    "swsidewaysbackhalfdegraded",
    "swrightsideupfull",
    "swrightsideupfronthalf",
    "swrightsideupbackhalf",
    "swrightsideupfulldegraded",
    "swrightsideupfronthalfdegraded",
    "swrightsideupbackhalfdegraded",
    "swupsidedownfull",
    "swupsidedownfronthalf",
    "swupsidedownbackhalf",
    "swupsidedownfulldegraded",
    "swupsidedownfronthalfdegraded",
    "swupsidedownbackhalfdegraded",
];
const WOOD_FAMILIES: [[&str; 6]; 10] = [
    ["oak_log", "spruce_log", "birch_log", "jungle_log", "acacia_log", "dark_oak_log"],
    ["oak_slab", "spruce_slab", "birch_slab", "jungle_slab", "acacia_slab", "dark_oak_slab"],
    [
        "oak_double_slab",
        "spruce_double_slab",
        "birch_double_slab",
        "jungle_double_slab",
        "acacia_double_slab",
        "dark_oak_double_slab",
    ],
    ["oak_planks", "spruce_planks", "birch_planks", "jungle_planks", "acacia_planks", "dark_oak_planks"],
    ["oak_fence", "spruce_fence", "birch_fence", "jungle_fence", "acacia_fence", "dark_oak_fence"],
    ["oak_wood", "spruce_wood", "birch_wood", "jungle_wood", "acacia_wood", "dark_oak_wood"],
    ["wooden_door", "spruce_door", "birch_door", "jungle_door", "acacia_door", "dark_oak_door"],
    ["trapdoor", "spruce_trapdoor", "birch_trapdoor", "jungle_trapdoor", "acacia_trapdoor", "dark_oak_trapdoor"],
    ["oak_stairs", "spruce_stairs", "birch_stairs", "jungle_stairs", "acacia_stairs", "dark_oak_stairs"],
    [
        "stripped_oak_log",
        "stripped_spruce_log",
        "stripped_birch_log",
        "stripped_jungle_log",
        "stripped_acacia_log",
        "stripped_dark_oak_log",
    ],
];
pub fn declare_wood_states(pool: &mut StatePoolBuilder) {
    let placed = pool.entries().to_vec();
    for entry in placed {
        for family in WOOD_FAMILIES.iter().filter(|family| family.contains(&entry.name.as_str())) {
            for member in family {
                pool.intern(member, &entry.states, entry.waterlogged);
            }
        }
    }
}

const WOODS: [(u8, u8); 9] = [(0, 1), (3, 1), (5, 3), (5, 1), (1, 0), (1, 3), (1, 5), (0, 2), (1, 4)];
const CORNERS: [(i32, i32); 4] = crate::shared::structure::bounds::CHUNK_CORNERS;

pub fn is_beached(probe: &dyn StartProbe, beaches: &[i32], (x, z): (i32, i32)) -> bool {
    let beach = |x: i32, y: i32, z: i32| beaches.contains(&probe.biome(x, y, z));
    let sea_level = probe.sea_level();
    let y = if beach(x * 16 + 8, sea_level, z * 16 + 8) { sea_level } else { sea_level - SEA_PROBE_DEPTH };
    beach(x * 16, y, z * 16)
}

fn average_floor(plan: &Plan, size: (i32, i32, i32), (cx, cz): (i32, i32), rotation: Rotation) -> Probed<i32> {
    let (sx, sz) = (size.0, size.2);
    let (ax, _, az) = rotation.apply((sx / 4, 0, sz / 4));
    let (bx, _, bz) = rotation.apply((sx * 3 / 4, 0, sz * 3 / 4));
    let (mut sum, mut count) = (0, 0);
    for x in cx + ax.min(bx)..=cx + ax.max(bx) {
        for z in cz + az.min(bz)..=cz + az.max(bz) {
            sum += plan.floor_height(x, z)?;
            count += 1;
        }
    }
    Ok(sum / count)
}

pub fn place(plan: &mut Plan, library: &Library, random: &mut MersenneTwister, start: (i32, i32), beached: bool) -> Probed<()> {
    let sea_level = plan.sea_level();
    let (x0, z0) = (start.0 * 16, start.1 * 16);
    let rotation = Rotation::from_index(random.next_int_bounded(4));
    let mut index = random.next_int_bounded(TEMPLATE_COUNT);
    if beached {
        index = random.next_int_bounded(BEACHED_TEMPLATES);
    }
    let Some(template) = library.templates.get(&format!("shipwreck/{}", TEMPLATES[index as usize])) else {
        return Ok(());
    };
    let (oak, dark_oak) = WOODS[random.next_int_bounded(WOOD_CHOICES) as usize];
    let (dx, dz) = CORNERS[rotation as usize];
    let corner = (x0 + dx, z0 + dz);
    let mut y = average_floor(plan, template.size, corner, rotation)? + if index < BEACHED_TEMPLATES { DEPTH_BEACHED } else { DEPTH_SUNK };
    if beached && y < sea_level {
        y = sea_level;
    }
    let origin = (corner.0, y, corner.1);
    let mut remap = [None; 6];
    remap[OAK] = Some(oak);
    remap[DARK_OAK] = Some(dark_oak);
    let options = Options {
        flood: !beached,
        wood: Some(Wood { families: &WOOD_FAMILIES, remap }),
        ..Options::default()
    };
    let chests = library.place(plan, &template, origin, rotation, options);
    random.skip(chests);
    let fill = |y: i32, ids: &crate::shared::structure::StructureBlocks| if y < sea_level { ids.water } else { ids.air };
    for ((x, y, z), marker) in markers_at(&template, origin, rotation) {
        plan.set(x, y, z, fill(y, plan.ids));
        let is_chest = matches!(marker, "treasureChest" | "supplyChest");
        let removed = is_chest && random.next_int_bounded(CHEST_REMOVAL_ROLL) < CHEST_REMOVAL_BELOW;
        if removed {
            let below = if sea_level < y { plan.ids.air } else { plan.ids.water };
            plan.set(x, y - 1, z, below);
        } else if is_chest || marker == "mapChest" {
            random.next_u32();
        }
    }
    Ok(())
}

pub struct ShipwreckBuilder;

impl StructureBuilder for ShipwreckBuilder {
    fn is_start(&self, context: &StructureContext, _: StructureKind, chunk: (i32, i32)) -> Probed<bool> {
        Ok(context.structures.shipwreck_biomes(context.probe, chunk) && !context.structures.near_monument(context.probe, chunk)?)
    }

    fn build(&self, context: &StructureContext, kind: StructureKind, chunk: (i32, i32)) -> Probed<Option<Built>> {
        let Some(blocks) = context.structures.table.get() else { return Ok(None) };
        Ok(self.plan(context, blocks, kind, chunk)?.map(|plan| plan as Built))
    }

    fn plan(&self, context: &StructureContext, blocks: &Blocks, _: StructureKind, start: (i32, i32)) -> Probed<Option<Arc<PlanWrites>>> {
        let (structures, probe) = (context.structures, context.probe);
        context.structures.build_plan(probe, blocks, start, |plan, library, random| {
            place(plan, library, random, start, is_beached(probe, structures.tag("beaches"), start))
        })
    }

    fn place(&self, context: &StructureContext, blocks: &Blocks, reaching: &Reaching, chunk: &mut ProtoChunk) -> Probed<()> {
        for &(kind, start) in reaching.starts.iter().filter(|&&(kind, _)| kind == StructureKind::Shipwreck) {
            place_plan(context, blocks, kind, start, chunk)?;
        }
        Ok(())
    }
}
