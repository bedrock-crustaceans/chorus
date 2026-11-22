use crate::shared::blocks::Blocks;
use crate::shared::chunk::proto::ProtoChunk;
use crate::shared::structure::builder::{Built, StructureBuilder, StructureContext, erase};
use crate::shared::structure::{BoundingBox, Probed, Reaching, StructureKind};

pub struct MonumentBuilder;

impl StructureBuilder for MonumentBuilder {
    fn is_start(&self, context: &StructureContext, _: StructureKind, chunk: (i32, i32)) -> Probed<bool> {
        Ok(context.structures.monument_biomes(context.probe, chunk))
    }

    fn build(&self, context: &StructureContext, _: StructureKind, chunk: (i32, i32)) -> Probed<Option<Built>> {
        Ok(Some(erase(context.structures.build_monument(chunk, context.probe.sea_level()))))
    }

    fn reaches(&self, context: &StructureContext, _: StructureKind, start: (i32, i32), area: &BoundingBox) -> Probed<bool> {
        Ok(context.structures.monument(context.probe, start)?.is_some_and(|monument| monument.bounds.intersects(area)))
    }

    fn place(&self, context: &StructureContext, blocks: &Blocks, reaching: &Reaching, chunk: &mut ProtoChunk) -> Probed<()> {
        context.structures.place_monuments(context.probe, blocks, reaching, chunk)
    }

    fn neighbour_rank(&self) -> Option<u8> {
        Some(6)
    }
}
