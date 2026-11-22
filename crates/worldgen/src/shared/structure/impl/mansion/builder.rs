use crate::shared::blocks::Blocks;
use crate::shared::chunk::proto::ProtoChunk;
use crate::shared::structure::builder::{Built, Region, StructureBuilder, StructureContext, covers, erase, passes_filter};
use crate::shared::structure::{BoundingBox, Probed, Reaching, StructureKind};

pub struct MansionBuilder;

impl StructureBuilder for MansionBuilder {
    fn is_start(&self, context: &StructureContext, _: StructureKind, chunk: (i32, i32)) -> Probed<bool> {
        Ok(passes_filter(context, StructureKind::Mansion, chunk))
    }

    fn build(&self, context: &StructureContext, _: StructureKind, chunk: (i32, i32)) -> Probed<Option<Built>> {
        Ok(context.structures.build_mansion(context.probe, chunk)?.map(erase))
    }

    fn reaches(&self, context: &StructureContext, _: StructureKind, start: (i32, i32), area: &BoundingBox) -> Probed<bool> {
        Ok(context
            .structures
            .mansion(context.probe, start)?
            .is_some_and(|mansion| covers(mansion.pieces.iter().map(|piece| &piece.bounds), area)))
    }

    fn regions(&self, context: &StructureContext, _: &Blocks, _: StructureKind, start: (i32, i32)) -> Probed<Vec<Region>> {
        Ok(context
            .structures
            .mansion(context.probe, start)?
            .map(|built| {
                built
                    .pieces
                    .iter()
                    .map(|piece| Region {
                        bounds: piece.bounds,
                        template: true,
                        settled_walls: true,
                        keep_template_post: false,
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    fn place(&self, context: &StructureContext, blocks: &Blocks, reaching: &Reaching, chunk: &mut ProtoChunk) -> Probed<()> {
        context.structures.place_mansions(context.probe, blocks, reaching, chunk)
    }

    fn neighbour_rank(&self) -> Option<u8> {
        Some(7)
    }
}
