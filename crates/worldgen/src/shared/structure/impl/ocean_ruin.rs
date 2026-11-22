use super::scattered::Material;
use crate::shared::blocks::Blocks;
use crate::shared::chunk::proto::ProtoChunk;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::rng::random::{RandomSource, block_seed};
use crate::shared::rng::xoroshiro::Xoroshiro;
use crate::shared::structure::builder::{StructureBuilder, StructureContext, passes_filter};
use crate::shared::structure::place::{Library, Options, Rotation};
use crate::shared::structure::plan::Plan;
use crate::shared::structure::template::Template;
use crate::shared::structure::{BoundingBox, Probed};
use crate::shared::structure::{CHUNK_WIDTH, Reaching, StructureKind};
use chorus_level::dimension_definition::DimensionDefinition;

pub const WARM_OCEANS: &[i32] = &[40, 41, 42, 43];
const COLD_OCEANS: [i32; 3] = [24, 45, 47];
const WARM_LARGE_PROBABILITY: f32 = 0.3;
const WARM_CLUSTER_PROBABILITY: f32 = 0.5;
const COLD_LARGE_PROBABILITY: f32 = 0.5;
const COLD_CLUSTER_PROBABILITY: f32 = 0.4;
const MILD_LARGE_PROBABILITY: f32 = 0.3;
const MILD_CLUSTER_PROBABILITY: f32 = 0.25;
const LARGE_INTEGRITY: f32 = 0.9;
const SMALL_INTEGRITY: f32 = 0.8;
const CRACKED_INTEGRITY: f32 = 0.7;
const MOSSY_INTEGRITY: f32 = 0.5;
const CORNERS: [(i32, i32); 4] = crate::shared::structure::bounds::CHUNK_CORNERS;
const BIG_FOOTPRINT: (i32, i32) = (15, 15);
const SMALL_FOOTPRINT: (i32, i32) = (5, 6);
const CLUSTER_SPOTS: [(i32, i32, i32, i32); 8] = [
    (-15, 7, 17, 6),
    (-15, 7, 1, 6),
    (-15, 7, -12, 4),
    (1, 6, 17, 6),
    (1, 6, -12, 2),
    (17, 6, 19, 5),
    (17, 6, 1, 6),
    (17, 6, -12, 4),
];
const MIN_CLUSTER: i32 = 4;
const CLUSTER_RANGE: i32 = 4;
const BURIED_LIMIT: usize = 5;
const SINK_DEPTH: i32 = 3;
const SINK_MARGIN: i32 = 2;
const CHEST_MARKER: &str = "chest";
const WARM_TEMPLATES: [&str; 8] = ["ruin_warm1", "ruin_warm2", "ruin_warm3", "ruin_warm4", "ruin_warm5", "ruin_warm6", "ruin_warm7", "ruin_warm8"];
const BIG_WARM_TEMPLATES: [&str; 4] = ["big_ruin_warm4", "big_ruin_warm5", "big_ruin_warm6", "big_ruin_warm7"];
const COLD_TEMPLATES: [[&str; 3]; 9] = [
    ["ruin4", "ruin4", "ruin4"],
    ["ruin1_brick", "ruin1_cracked", "ruin1_mossy"],
    ["ruin2_brick", "ruin2_cracked", "ruin2_mossy"],
    ["ruin3_brick", "ruin3_cracked", "ruin3_mossy"],
    ["ruin4_brick", "ruin4_cracked", "ruin4_mossy"],
    ["ruin5_brick", "ruin5_cracked", "ruin5_mossy"],
    ["ruin6_brick", "ruin6_cracked", "ruin6_mossy"],
    ["ruin7_brick", "ruin7_cracked", "ruin7_mossy"],
    ["ruin8_brick", "ruin8_cracked", "ruin8_mossy"],
];
const BIG_COLD_TEMPLATES: [[&str; 3]; 4] = [
    ["big_ruin1_brick", "big_ruin1_cracked", "big_ruin1_mossy"],
    ["big_ruin2_brick", "big_ruin2_cracked", "big_ruin2_mossy"],
    ["big_ruin3_brick", "big_ruin3_cracked", "big_ruin3_mossy"],
    ["big_ruin8_brick", "big_ruin8_cracked", "big_ruin8_mossy"],
];

pub struct RuinPiece {
    pub name: &'static str,
    origin: (i32, i32),
    rotation: Rotation,
    integrity: f32,
    pub bounds: BoundingBox,
}

pub struct RuinSite {
    pub warm: bool,
    pub pieces: Vec<RuinPiece>,
    definition: DimensionDefinition,
}

pub struct Surface<'a> {
    pub plan: Plan<'a>,
    pub chunk: (i32, i32),
    pub seed: u32,
}

fn footprint(origin: (i32, i32), rotation: Rotation, (corner_x, corner_z): (i32, i32), definition: DimensionDefinition) -> BoundingBox {
    let (turned_x, _, turned_z) = rotation.apply((corner_x, 0, corner_z));
    let (end_x, end_z) = (origin.0 + turned_x, origin.1 + turned_z);
    BoundingBox::new(origin.0.min(end_x), definition.min_y, origin.1.min(end_z), origin.0.max(end_x), definition.max_y(), origin.1.max(end_z))
}

impl RuinSite {
    pub fn generate(mut random: MersenneTwister, library: &Library, start: (i32, i32), biome: i32, definition: DimensionDefinition) -> Self {
        let warm = WARM_OCEANS.contains(&biome);
        let (large_probability, cluster_probability) = if warm {
            (WARM_LARGE_PROBABILITY, WARM_CLUSTER_PROBABILITY)
        } else if COLD_OCEANS.contains(&biome) {
            (COLD_LARGE_PROBABILITY, COLD_CLUSTER_PROBABILITY)
        } else {
            (MILD_LARGE_PROBABILITY, MILD_CLUSTER_PROBABILITY)
        };
        let rotation = Rotation::from_index(random.next_int_bounded(4));
        let (dx, dz) = CORNERS[rotation as usize];
        let origin = (start.0 * 16 + dx, start.1 * 16 + dz);
        let mut site = Self { warm, pieces: Vec::new(), definition };
        let large = random.next_float() <= large_probability;
        site.add(&mut random, library, origin, rotation, large, if large { LARGE_INTEGRITY } else { SMALL_INTEGRITY });
        if large && random.next_float() <= cluster_probability {
            site.add_cluster(&mut random, library, origin, rotation);
        }
        site
    }

    fn push(&mut self, library: &Library, name: &'static str, origin: (i32, i32), rotation: Rotation, integrity: f32) {
        let Some(template) = library.templates.get(&format!("ruin/{name}")) else { return };
        let bounds = footprint(origin, rotation, (template.size.0 - 1, template.size.2 - 1), self.definition);
        self.pieces.push(RuinPiece {
            name,
            origin,
            rotation,
            integrity,
            bounds,
        });
    }

    fn add(&mut self, random: &mut MersenneTwister, library: &Library, origin: (i32, i32), rotation: Rotation, large: bool, integrity: f32) {
        if self.warm {
            let name = if large {
                BIG_WARM_TEMPLATES[random.next_int_bounded(BIG_WARM_TEMPLATES.len() as i32) as usize]
            } else {
                WARM_TEMPLATES[random.next_int_bounded(WARM_TEMPLATES.len() as i32) as usize]
            };
            self.push(library, name, origin, rotation, integrity);
            return;
        }
        let [brick, cracked, mossy] = if large {
            BIG_COLD_TEMPLATES[random.next_int_bounded(BIG_COLD_TEMPLATES.len() as i32) as usize]
        } else {
            COLD_TEMPLATES[random.next_int_bounded(COLD_TEMPLATES.len() as i32) as usize]
        };
        self.push(library, brick, origin, rotation, integrity);
        self.push(library, cracked, origin, rotation, CRACKED_INTEGRITY);
        self.push(library, mossy, origin, rotation, MOSSY_INTEGRITY);
    }

    fn add_cluster(&mut self, random: &mut MersenneTwister, library: &Library, origin: (i32, i32), rotation: Rotation) {
        let big = footprint(origin, rotation, BIG_FOOTPRINT, self.definition);
        let mut spots: Vec<(i32, i32)> = CLUSTER_SPOTS
            .iter()
            .map(|&(dx, range_x, dz, range_z)| {
                let x = big.x0 + dx + random.next_int_bounded(range_x);
                (x, big.z0 + dz + random.next_int_bounded(range_z))
            })
            .collect();
        let count = random.next_int_bounded(CLUSTER_RANGE) + MIN_CLUSTER;
        for _ in 0..count {
            let at = spots.remove(random.next_int_bounded(spots.len() as i32) as usize);
            let turn = Rotation::from_index(random.next_int_bounded(4));
            let small = footprint(at, turn, SMALL_FOOTPRINT, self.definition);
            let overlaps = small.x1 >= big.x0 && small.x0 <= big.x1 && small.z1 >= big.z0 && small.z0 <= big.z1;
            if !overlaps {
                self.add(random, library, at, turn, false, SMALL_INTEGRITY);
            }
        }
    }

    pub fn reaches(&self, chunk: (i32, i32)) -> bool {
        let column = BoundingBox::chunk(chunk.0, chunk.1, self.definition);
        self.pieces.iter().any(|piece| piece.bounds.intersects(&column))
    }

    pub fn place(&self, surface: &mut Surface, library: &Library) -> Probed<()> {
        let column = BoundingBox::chunk(surface.chunk.0, surface.chunk.1, surface.plan.definition());
        for piece in self.pieces.iter().filter(|piece| piece.bounds.intersects(&column)) {
            piece.place(surface, library, self.warm)?;
        }
        Ok(())
    }
}

impl RuinPiece {
    fn floor_y(&self, plan: &Plan) -> Probed<i32> {
        let height = plan.blocking_height(self.origin.0, self.origin.1, true)?;
        let (mut lowest, mut sunk) = (i32::MAX, 0);
        for x in self.bounds.x0..=self.bounds.x1 {
            for z in self.bounds.z0..=self.bounds.z1 {
                let floor = (plan.floor_height(x, z)? - 1).max(plan.definition().min_y + 1);
                lowest = lowest.min(floor);
                sunk += i32::from(floor < height - SINK_DEPTH);
            }
        }
        let width = self.bounds.x1 - self.bounds.x0;
        Ok(if height - lowest > SINK_DEPTH && sunk > width - SINK_MARGIN { lowest } else { height - 1 })
    }

    fn filler(&self, warm: bool) -> Option<Material> {
        match self.name {
            _ if warm => Some(Material::Sandstone),
            "ruin3_brick" => Some(Material::StoneBricks),
            "ruin2_brick" | "big_ruin1_brick" => Some(Material::Gravel),
            _ => None,
        }
    }

    fn place(&self, surface: &mut Surface, library: &Library, warm: bool) -> Probed<()> {
        let Some(template) = library.templates.get(&format!("ruin/{}", self.name)) else { return Ok(()) };
        let y = self.floor_y(&surface.plan)?;
        let origin = (self.origin.0, y, self.origin.1);
        let options = Options {
            flood: true,
            ignore_air: true,
            integrity: Some((self.integrity.clamp(0.0, 1.0), surface.seed)),
            ..Options::default()
        };
        let written = surface.plan.writes.len();
        library.place(&mut surface.plan, &template, origin, self.rotation, options);
        let floor = surface.plan.ids.scattered_id(if warm { Material::Sand } else { Material::Gravel });
        let inside = |x: i32, z: i32| x.div_euclid(16) == surface.chunk.0 && z.div_euclid(16) == surface.chunk.1;
        let buried: Vec<(i32, i32, i32)> = surface.plan.writes[written..]
            .iter()
            .filter(|&&(x, _, z, block)| block == floor && inside(x, z))
            .map(|&(x, y, z, _)| (x, y, z))
            .collect();
        self.bury_loot(surface, origin, buried, warm);
        self.place_chests(surface, &template, origin, warm);
        Ok(())
    }

    fn bury_loot(&self, surface: &mut Surface, origin: (i32, i32, i32), mut cells: Vec<(i32, i32, i32)>, warm: bool) {
        let mut random = Xoroshiro::new(block_seed(origin.0, origin.1, origin.2) as i32 as i64);
        for i in 1..cells.len() {
            let j = random.next_int_bounded(i as i32 + 1) as usize;
            cells.swap(i, j);
        }
        let suspicious = surface.plan.ids.scattered_id(if warm { Material::SuspiciousSand } else { Material::SuspiciousGravel });
        for &(x, y, z) in cells.iter().take(BURIED_LIMIT) {
            surface.plan.set(x, y, z, suspicious);
        }
    }

    fn place_chests(&self, surface: &mut Surface, template: &Template, origin: (i32, i32, i32), warm: bool) {
        let filler = self.filler(warm).map_or(surface.plan.ids.water, |material| surface.plan.ids.scattered_id(material));
        for (index, _) in template.markers.iter().filter(|(_, data)| data == CHEST_MARKER) {
            let block = template.blocks[*index];
            let (dx, dy, dz) = self.rotation.apply((block.x as i32, block.y as i32, block.z as i32));
            let (x, y, z) = (origin.0 + dx, origin.1 + dy, origin.2 + dz);
            if !(x.div_euclid(16) == surface.chunk.0 && z.div_euclid(16) == surface.chunk.1) {
                continue;
            }
            let chest = if y <= surface.plan.sea_level() {
                surface.plan.ids.scattered.flooded_chest_east
            } else {
                surface.plan.ids.scattered_id(Material::ChestEast)
            };
            surface.plan.set(x, y, z, filler);
            surface.plan.set(x, y - 1, z, chest);
        }
    }
}

pub struct OceanRuinsBuilder;

impl StructureBuilder for OceanRuinsBuilder {
    fn is_start(&self, context: &StructureContext, _: StructureKind, chunk: (i32, i32)) -> Probed<bool> {
        Ok(passes_filter(context, StructureKind::OceanRuins, chunk) && !context.structures.near_monument(context.probe, chunk)?)
    }

    fn place(&self, context: &StructureContext, blocks: &Blocks, reaching: &Reaching, chunk: &mut ProtoChunk) -> Probed<()> {
        let structures = context.structures;
        let chunk_coords = (chunk.min_x().div_euclid(CHUNK_WIDTH), chunk.min_z().div_euclid(CHUNK_WIDTH));
        for &(_, start) in reaching.starts.iter().filter(|&&(kind, _)| kind == StructureKind::OceanRuins) {
            let Some(writes) = structures.ocean_ruin_writes(context.probe, blocks, start, chunk_coords)? else {
                continue;
            };
            structures.apply(blocks, chunk, StructureKind::OceanRuins, &writes);
        }
        Ok(())
    }
}
