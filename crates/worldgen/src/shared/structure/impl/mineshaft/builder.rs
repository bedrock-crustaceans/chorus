use crate::shared::blocks::Blocks;
use crate::shared::chunk::proto::ProtoChunk;
use crate::shared::structure::builder::{Built, Region, StructureBuilder, StructureContext, erase};
use crate::shared::structure::{BoundingBox, Probed, Reaching, StructureKind};

pub struct MineshaftBuilder;

impl StructureBuilder for MineshaftBuilder {
    fn is_start(&self, context: &StructureContext, _: StructureKind, chunk: (i32, i32)) -> Probed<bool> {
        Ok(context.structures.mineshaft.is_start(chunk))
    }

    fn build(&self, context: &StructureContext, _: StructureKind, chunk: (i32, i32)) -> Probed<Option<Built>> {
        Ok(Some(erase(context.structures.build_mineshaft(context.probe, chunk))))
    }

    fn reaches(&self, context: &StructureContext, _: StructureKind, start: (i32, i32), area: &BoundingBox) -> Probed<bool> {
        Ok(context.structures.mineshaft(context.probe, start)?.is_some_and(|shaft| shaft.bounds.intersects(area)))
    }

    fn regions(&self, context: &StructureContext, _: &Blocks, _: StructureKind, start: (i32, i32)) -> Probed<Vec<Region>> {
        Ok(context
            .structures
            .mineshaft_bounds(context.probe, start)?
            .map(|bounds| {
                vec![Region {
                    bounds,
                    template: false,
                    settled_walls: false,
                    keep_template_post: false,
                }]
            })
            .unwrap_or_default())
    }

    fn contains(&self, context: &StructureContext, chunk: (i32, i32), (x, y, z): (i32, i32, i32), _: bool) -> Probed<bool> {
        let structures = context.structures;
        let reach = structures.registry().reach(StructureKind::Mineshaft);
        for start in structures.starts_near(context.probe, StructureKind::Mineshaft, chunk, reach)? {
            if structures.mineshaft(context.probe, start)?.is_some_and(|shaft| shaft.bounds.contains(x, y, z)) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn near_window(&self, context: &StructureContext, chunk: (i32, i32), margin: i32) -> Probed<()> {
        let structures = context.structures;
        structures.starts_near(context.probe, StructureKind::Mineshaft, chunk, structures.registry().reach(StructureKind::Mineshaft) + margin)?;
        Ok(())
    }

    fn place(&self, context: &StructureContext, blocks: &Blocks, reaching: &Reaching, chunk: &mut ProtoChunk) -> Probed<()> {
        context.structures.place_mineshafts(context.probe, blocks, reaching, chunk)
    }

    fn neighbour_rank(&self) -> Option<u8> {
        Some(2)
    }
}
