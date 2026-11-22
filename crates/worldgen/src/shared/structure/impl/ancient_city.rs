use crate::shared::structure::builder::{Built, Region, StructureBuilder, StructureContext, covers, erase};
use crate::shared::structure::{ANCIENT_CITY_Y, Adjustments, BoundingBox, CHUNK_WIDTH, contains_only};
use std::sync::Arc;

use crate::shared::blocks::Blocks;
use crate::shared::chunk::proto::ProtoChunk;
use crate::shared::structure::layout::{CitySpec, Part};
use crate::shared::structure::{Probed, Reaching, StartProbe, StructureKind, Structures};

const SPEC: CitySpec = CitySpec {
    pool: "ancient_city/city_center",
    anchor: "minecraft:city_anchor",
    max_depth: 7,
    radius: 128,
    ground: -27,
};

pub struct AncientCity {
    pub pieces: Vec<Part>,
}

impl Structures {
    pub(in crate::shared::structure) fn ancient_city(&self, probe: &dyn StartProbe, start: (i32, i32)) -> Probed<Option<Arc<AncientCity>>> {
        Ok(self.starts_at(probe, start)?.get::<AncientCity>(StructureKind::AncientCity))
    }

    pub fn build_ancient_city(&self, start: (i32, i32)) -> Option<AncientCity> {
        let (catalogue, library) = (self.layout()?, self.data.library.as_ref()?);
        let (_, mut random) = self.registry().grid(StructureKind::AncientCity).expect("ancient city grid").candidate_random(self.level_seed, start);
        Some(AncientCity {
            pieces: SPEC.assemble(catalogue, library, &mut random, start),
        })
    }

    pub(in crate::shared::structure) fn place_ancient_cities(&self, probe: &dyn StartProbe, blocks: &Blocks, reaching: &Reaching, chunk: &mut ProtoChunk) -> Probed<()> {
        let mut cities = Vec::new();
        for &(_, start) in reaching.starts.iter().filter(|&&(kind, _)| kind == StructureKind::AncientCity) {
            cities.extend(self.ancient_city(probe, start)?);
        }
        self.place_parts(probe, blocks, chunk, false, cities.iter().flat_map(|city| city.pieces.iter()))
    }
}

pub struct AncientCityBuilder;

impl StructureBuilder for AncientCityBuilder {
    fn is_start(&self, context: &StructureContext, _: StructureKind, chunk: (i32, i32)) -> Probed<bool> {
        Ok(contains_only(
            context.probe,
            (chunk.0 * CHUNK_WIDTH, ANCIENT_CITY_Y, chunk.1 * CHUNK_WIDTH),
            0,
            context.structures.tag("deep_dark"),
        ))
    }

    fn build(&self, context: &StructureContext, _: StructureKind, chunk: (i32, i32)) -> Probed<Option<Built>> {
        Ok(context.structures.build_ancient_city(chunk).map(erase))
    }

    fn reaches(&self, context: &StructureContext, _: StructureKind, start: (i32, i32), area: &BoundingBox) -> Probed<bool> {
        Ok(context
            .structures
            .ancient_city(context.probe, start)?
            .is_some_and(|city| covers(city.pieces.iter().map(|piece| &piece.bounds), area)))
    }

    fn regions(&self, context: &StructureContext, _: &Blocks, _: StructureKind, start: (i32, i32)) -> Probed<Vec<Region>> {
        Ok(context
            .structures
            .ancient_city(context.probe, start)?
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
        context.structures.adjust_cities(context.probe, chunk, adjustments)
    }

    fn place(&self, context: &StructureContext, blocks: &Blocks, reaching: &Reaching, chunk: &mut ProtoChunk) -> Probed<()> {
        context.structures.place_ancient_cities(context.probe, blocks, reaching, chunk)
    }

    fn neighbour_rank(&self) -> Option<u8> {
        Some(4)
    }
}
