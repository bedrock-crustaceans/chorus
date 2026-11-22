use std::sync::Arc;

use super::{Scattered, Site, pyramid_height_chunk};
use crate::shared::blocks::Blocks;
use crate::shared::chunk::proto::ProtoChunk;
use crate::shared::structure::builder::{Built, Region, StructureBuilder, StructureContext, place_plan, surface_centre};
use crate::shared::structure::r#impl::igloo;
use crate::shared::structure::plan::PlanWrites;
use crate::shared::structure::{BoundingBox, CHUNK_WIDTH, Probed, Reaching, StructureKind, Structures};

pub struct TempleBuilder;

impl StructureBuilder for TempleBuilder {
    fn is_start(&self, context: &StructureContext, _: StructureKind, chunk: (i32, i32)) -> Probed<bool> {
        let (x, y, z) = surface_centre(context, chunk);
        Ok(context.structures.tag("temple").contains(&context.probe.biome(x, y, z)) && context.probe.lowest_column_top(chunk)? >= context.probe.sea_level())
    }

    fn reaches(&self, context: &StructureContext, _: StructureKind, start: (i32, i32), _: &BoundingBox) -> Probed<bool> {
        Ok(context.structures.tag("temple").contains(&Structures::centre_biome(context.probe, start)))
    }

    fn regions(&self, context: &StructureContext, _: &Blocks, kind: StructureKind, start: (i32, i32)) -> Probed<Vec<Region>> {
        let huts = matches!(
            Scattered::of_biome(Structures::centre_biome(context.probe, start), context.structures.registry()),
            Some(Scattered::JungleTemple | Scattered::SwampHut)
        );
        if !huts {
            return Ok(Vec::new());
        }
        Ok(vec![Region {
            bounds: context.structures.registry().rough_area(kind, start, context.probe.definition()),
            template: false,
            settled_walls: false,
            keep_template_post: false,
        }])
    }

    fn build(&self, context: &StructureContext, kind: StructureKind, chunk: (i32, i32)) -> Probed<Option<Built>> {
        if Scattered::of_biome(Structures::centre_biome(context.probe, chunk), context.structures.registry()).is_some() {
            return Ok(None);
        }
        let Some(blocks) = context.structures.table.get() else { return Ok(None) };
        Ok(self.plan(context, blocks, kind, chunk)?.map(|plan| plan as Built))
    }

    fn plan(&self, context: &StructureContext, blocks: &Blocks, _: StructureKind, start: (i32, i32)) -> Probed<Option<Arc<PlanWrites>>> {
        context
            .structures
            .build_plan(context.probe, blocks, start, |plan, library, random| igloo::place(plan, library, random, start))
    }

    fn place(&self, context: &StructureContext, blocks: &Blocks, reaching: &Reaching, chunk: &mut ProtoChunk) -> Probed<()> {
        let structures = context.structures;
        let probe = context.probe;
        let chunk_coords = (chunk.min_x().div_euclid(CHUNK_WIDTH), chunk.min_z().div_euclid(CHUNK_WIDTH));
        for &(kind, start) in reaching.starts.iter().filter(|&&(kind, _)| kind == StructureKind::Temple) {
            match Scattered::of_biome(Structures::centre_biome(probe, start), structures.registry()) {
                Some(temple) => {
                    let columns = |x: i32, z: i32| structures.column_at(probe, x, z);
                    let (height_x, height_z) = pyramid_height_chunk(start);
                    let site = Site {
                        start,
                        first_random: structures.seeds.random(height_x, height_z),
                        columns: &columns,
                    };
                    let mut random = structures.seeds.random(chunk_coords.0, chunk_coords.1);
                    temple.post_process(chunk, blocks, &structures.blocks.scattered, &site, &mut random)?;
                }
                None => place_plan(context, blocks, kind, start, chunk)?,
            }
        }
        Ok(())
    }
}
