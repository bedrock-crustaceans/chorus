mod place;

pub use place::VillageBlocks;

use crate::shared::structure::Adjustments;
use crate::shared::structure::builder::{Built, Region, StructureBuilder, StructureContext, covers, erase, passes_filter};
use std::sync::Arc;

use crate::shared::CHUNK_WIDTH;
use crate::shared::blocks::Blocks;
use crate::shared::chunk::proto::ProtoChunk;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::layout::{Grower, Part, PoolCatalogue, middle};
use crate::shared::structure::place::Rotation;
use crate::shared::structure::plan::Plan;
use crate::shared::structure::{BoundingBox, Probed, Reaching, StartProbe, StructureKind, Structures};
use place::Surface;

const POOL_CATALOGUE: &str = "village";
const MAX_DEPTH: i32 = 6;
const ROTATION_COUNT: i32 = 4;
const ABANDONED_CHANCE: f32 = 0.02;
const START_Y_OFFSET: i32 = 1;
const EMPTY_COLUMN_HEIGHT: i32 = 0;
const SANDY_BIOMES: [i32; 5] = [2, 16, 17, 26, 130];

pub struct Village {
    pub pieces: Vec<Part>,
}

fn village_type(structures: &Structures, biome: i32) -> &'static str {
    ["desert", "snowy", "savanna", "taiga"]
        .into_iter()
        .find(|kind| structures.tag(&format!("village_{kind}")).contains(&biome))
        .unwrap_or("plains")
}

impl Structures {
    pub fn layout(&self) -> Option<&PoolCatalogue> {
        self.data.catalogue(POOL_CATALOGUE)
    }

    pub fn village_is_abandoned(&self, start: (i32, i32)) -> bool {
        let (_, mut random) = self.registry().grid(StructureKind::Village).expect("village grid").candidate_random(self.level_seed, start);
        random.next_int_bounded(ROTATION_COUNT);
        random.next_float() < ABANDONED_CHANCE
    }

    pub fn village(&self, probe: &dyn StartProbe, start: (i32, i32)) -> Probed<Option<Arc<Village>>> {
        Ok(self.starts_at(probe, start)?.get::<Village>(StructureKind::Village))
    }

    pub(in crate::shared::structure) fn build_village(&self, probe: &dyn StartProbe, start: (i32, i32)) -> Probed<Option<Village>> {
        let (Some(data), Some(library)) = (self.layout(), self.data.library.as_ref()) else {
            return Ok(None);
        };
        let (_, mut random) = self.registry().grid(StructureKind::Village).expect("village grid").candidate_random(self.level_seed, start);
        let surface = probe.preliminary_surface(start);
        let kind = village_type(self, Self::centre_biome(probe, start));
        let rotation = Rotation::from_index(random.next_int_bounded(ROTATION_COUNT));
        let abandoned = random.next_float() < ABANDONED_CHANCE;
        let pool_name = if abandoned {
            format!("village/abandoned/{kind}/town_centers")
        } else {
            format!("village/{kind}/town_centers")
        };
        let pool = data.pool(&pool_name).expect("village town centres");
        let element = pool.list[random.next_int_bounded(pool.list.len() as i32) as usize];
        let height = |x: i32, z: i32| self.raw_height(probe, x, z);
        let mut centre = Part::new(data, element, (start.0 * 16, surface, start.1 * 16), rotation, 0);
        let bounds = centre.bounds;
        centre.shift_y(height(middle(bounds.x0, bounds.x1), middle(bounds.z0, bounds.z1))? - START_Y_OFFSET - bounds.y0);
        let pieces = Grower::new(data, library, &mut random, &height, MAX_DEPTH).run(centre)?;
        Ok(Some(Village { pieces }))
    }

    pub(in crate::shared::structure) fn place_villages(&self, probe: &dyn StartProbe, blocks: &Blocks, reaching: &Reaching, chunk: &mut ProtoChunk) -> Probed<()> {
        let mut villages = Vec::new();
        for &(_, start) in reaching.starts.iter().filter(|&&(kind, _)| kind == StructureKind::Village) {
            villages.extend(self.village(probe, start)?);
        }
        self.place_parts(probe, blocks, chunk, true, villages.iter().flat_map(|village| village.pieces.iter()))
    }

    pub(in crate::shared::structure) fn place_parts<'p>(
        &self,
        probe: &dyn StartProbe,
        blocks: &Blocks,
        chunk: &mut ProtoChunk,
        rejects_floating: bool,
        parts: impl Iterator<Item = &'p Part>,
    ) -> Probed<()> {
        let (Some(palette), Some(library), Some(data)) = (self.blocks.palette.as_ref(), self.data.library.as_ref(), self.layout()) else {
            return Ok(());
        };
        let (chunk_x, chunk_z) = (chunk.min_x().div_euclid(CHUNK_WIDTH), chunk.min_z().div_euclid(CHUNK_WIDTH));
        let seed = self.seeds.chunk(chunk_x, chunk_z);
        let columns = |x: i32, z: i32| self.column_at(probe, x, z);
        let beyond = Plan::new(blocks, &self.blocks, &columns, probe);
        let generated: Vec<i32> = (0..CHUNK_WIDTH * CHUNK_WIDTH)
            .map(|column| chunk.worldgen_height(false, column & 15, column >> 4).unwrap_or(chunk.min_y))
            .collect();
        let tops = |x: i32, z: i32| generated[(((z & 15) << 4) | (x & 15)) as usize];
        let top_material = |x: i32, y: i32, z: i32| {
            if SANDY_BIOMES.contains(&probe.biome(x, y, z)) {
                self.blocks.village.sand
            } else {
                self.blocks.village.grass_block
            }
        };
        let bounds = BoundingBox::chunk(chunk_x, chunk_z, chunk.definition());
        let mut surface = Surface {
            probe,
            chunk,
            blocks,
            ids: &self.blocks,
            palette,
            library,
            data,
            beyond: &beyond,
            bounds,
            tops: &tops,
            top_material: &top_material,
            rejects_floating,
        };
        let mut random = MersenneTwister::new(seed);
        for piece in parts.filter(|piece| piece.bounds.intersects(&bounds)) {
            surface.place(piece, seed, &mut random)?;
        }
        Ok(())
    }

    pub(in crate::shared::structure) fn raw_height(&self, probe: &dyn StartProbe, x: i32, z: i32) -> Probed<i32> {
        let (_, top) = self.column_at(probe, x, z)?;
        let min_y = probe.definition().min_y;
        Ok((if top <= min_y { EMPTY_COLUMN_HEIGHT } else { top }).max(min_y))
    }
}

pub struct VillageBuilder;

impl StructureBuilder for VillageBuilder {
    fn is_start(&self, context: &StructureContext, _: StructureKind, chunk: (i32, i32)) -> Probed<bool> {
        Ok(passes_filter(context, StructureKind::Village, chunk))
    }

    fn build(&self, context: &StructureContext, _: StructureKind, chunk: (i32, i32)) -> Probed<Option<Built>> {
        Ok(context.structures.build_village(context.probe, chunk)?.map(erase))
    }

    fn reaches(&self, context: &StructureContext, _: StructureKind, start: (i32, i32), area: &BoundingBox) -> Probed<bool> {
        Ok(context
            .structures
            .village(context.probe, start)?
            .is_some_and(|village| covers(village.pieces.iter().map(|piece| &piece.bounds), area)))
    }

    fn regions(&self, context: &StructureContext, _: &Blocks, _: StructureKind, start: (i32, i32)) -> Probed<Vec<Region>> {
        Ok(context
            .structures
            .village(context.probe, start)?
            .map(|built| {
                built
                    .pieces
                    .iter()
                    .map(|piece| Region {
                        bounds: piece.bounds,
                        template: true,
                        settled_walls: false,
                        keep_template_post: false,
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    fn adjust(&self, context: &StructureContext, chunk: (i32, i32), adjustments: &mut Adjustments) -> Probed<()> {
        context.structures.adjust_villages(context.probe, chunk, adjustments)
    }

    fn place(&self, context: &StructureContext, blocks: &Blocks, reaching: &Reaching, chunk: &mut ProtoChunk) -> Probed<()> {
        context.structures.place_villages(context.probe, blocks, reaching, chunk)
    }

    fn neighbour_rank(&self) -> Option<u8> {
        Some(0)
    }
}
