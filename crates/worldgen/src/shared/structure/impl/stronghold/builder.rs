use crate::shared::blocks::Blocks;
use crate::shared::chunk::proto::ProtoChunk;
use crate::shared::structure::builder::{Built, Region, StructureBuilder, StructureContext, erase};
use crate::shared::structure::{Adjustments, BoundingBox, Probed, Reaching, StructureKind, placement};

pub struct StrongholdBuilder;

impl StructureBuilder for StrongholdBuilder {
    fn is_start(&self, context: &StructureContext, _: StructureKind, chunk: (i32, i32)) -> Probed<bool> {
        let structures = context.structures;
        Ok(structures.stronghold_spiral(context.probe)?.contains(&chunk) || placement::stronghold_grid_start(structures.level_seed, chunk) == Some(chunk))
    }

    fn build(&self, context: &StructureContext, _: StructureKind, chunk: (i32, i32)) -> Probed<Option<Built>> {
        Ok(Some(erase(context.structures.build_stronghold(chunk, context.probe.definition(), context.probe.sea_level()))))
    }

    fn reaches(&self, context: &StructureContext, _: StructureKind, start: (i32, i32), area: &BoundingBox) -> Probed<bool> {
        Ok(context.structures.stronghold(context.probe, start)?.is_some_and(|keep| keep.bounds.intersects(area)))
    }

    fn regions(&self, context: &StructureContext, _: &Blocks, _: StructureKind, start: (i32, i32)) -> Probed<Vec<Region>> {
        Ok(context
            .structures
            .stronghold(context.probe, start)?
            .map(|keep| keep.bounds)
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

    fn adjust(&self, context: &StructureContext, chunk: (i32, i32), adjustments: &mut Adjustments) -> Probed<()> {
        context.structures.adjust_strongholds(context.probe, chunk, adjustments)
    }

    fn contains(&self, context: &StructureContext, chunk: (i32, i32), (x, y, z): (i32, i32, i32), _: bool) -> Probed<bool> {
        let structures = context.structures;
        let reach = structures.registry().reach(StructureKind::Stronghold);
        for start in structures.starts_near(context.probe, StructureKind::Stronghold, chunk, reach)? {
            if structures.stronghold(context.probe, start)?.is_some_and(|keep| keep.bounds.contains(x, y, z)) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn near_window(&self, context: &StructureContext, chunk: (i32, i32), margin: i32) -> Probed<()> {
        let structures = context.structures;
        structures.starts_near(context.probe, StructureKind::Stronghold, chunk, structures.registry().reach(StructureKind::Stronghold) + margin)?;
        Ok(())
    }

    fn starts_near(&self, context: &StructureContext, _: StructureKind, centre: (i32, i32), radius: i32) -> Probed<Option<Vec<(i32, i32)>>> {
        let structures = context.structures;
        let (x0, z0, x1, z1) = (centre.0 - radius, centre.1 - radius, centre.0 + radius, centre.1 + radius);
        let inside = |(x, z): (i32, i32)| (x0..=x1).contains(&x) && (z0..=z1).contains(&z);
        let cells = |low: i32, high: i32| placement::stronghold_cell(low)..=placement::stronghold_cell(high);
        let grid = cells(x0, x1)
            .flat_map(|cell_x| cells(z0, z1).map(move |cell_z| (cell_x, cell_z)))
            .filter_map(|(cell_x, cell_z)| placement::stronghold_grid_start(structures.level_seed, (cell_x * 200 + 100, cell_z * 200 + 100)));
        Ok(Some(structures.stronghold_spiral(context.probe)?.iter().copied().chain(grid).filter(|&start| inside(start)).collect()))
    }

    fn place(&self, context: &StructureContext, blocks: &Blocks, reaching: &Reaching, chunk: &mut ProtoChunk) -> Probed<()> {
        context.structures.place_strongholds(context.probe, blocks, reaching, chunk)
    }

    fn neighbour_rank(&self) -> Option<u8> {
        Some(1)
    }
}
