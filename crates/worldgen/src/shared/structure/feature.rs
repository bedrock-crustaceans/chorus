use super::layout::{EntryKind, Part, PoolCatalogue, wide_position_seed};
use super::{BoundingBox, Probed, StartProbe, StructureContext, StructureKind, Structures};
use crate::shared::rng::mersenne::MersenneTwister;

const NAMESPACE: &str = "minecraft:";

pub struct FeatureSite {
    pub id: String,
    pub origin: (i32, i32, i32),
}

pub fn element_random(origin: (i32, i32, i32)) -> MersenneTwister {
    MersenneTwister::new(wide_position_seed(origin) as u32)
}

pub fn collect_sites(catalogue: &PoolCatalogue, parts: &[Part], area: &BoundingBox, sites: &mut Vec<FeatureSite>) {
    for part in parts.iter().filter(|part| part.bounds.intersects(area)) {
        if let EntryKind::Feature(name) = &catalogue.entries[part.entry].kind {
            sites.push(FeatureSite {
                id: format!("{NAMESPACE}{name}"),
                origin: part.origin,
            });
        }
    }
}

impl Structures {
    pub fn feature_sites(&self, probe: &dyn StartProbe, chunk: (i32, i32)) -> Probed<Vec<FeatureSite>> {
        let mut sites = Vec::new();
        let area = BoundingBox::chunk(chunk.0, chunk.1, probe.definition());
        let context = StructureContext { structures: self, probe };
        for builder in &self.builders {
            builder.feature_sites(&context, chunk, &area, &mut sites)?;
        }
        Ok(sites)
    }

    pub(super) fn v2_feature_sites(&self, probe: &dyn StartProbe, kind: StructureKind, chunk: (i32, i32), area: &BoundingBox, sites: &mut Vec<FeatureSite>) -> Probed<()> {
        let Some(catalogue) = self.v2_layout() else { return Ok(()) };
        for start in self.starts_near(probe, kind, chunk, self.registry().reach(kind))? {
            if !self.registry().rough_area(kind, start, probe.definition()).intersects(area) {
                continue;
            }
            if let Some(structure) = self.v2_structure(probe, kind, start)? {
                collect_sites(catalogue, &structure.pieces, area, sites);
            }
        }
        Ok(())
    }
}
