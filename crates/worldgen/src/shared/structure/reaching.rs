use super::entities::report_structure_entities;
use super::r#impl::mineshaft::{self, Mineshaft};
use super::r#impl::monument::{self, Monument};
use super::r#impl::ocean_ruin::{RuinSite, Surface};
use super::place::Library;
use super::plan::{Plan, PlanWrites};
use chorus_level::dimension_definition::DimensionDefinition;

use super::r#impl::stronghold::{self, Stronghold};
use super::{BoundingBox, Probed, StartProbe, StructureContext, StructureKind, Structures};
use crate::shared::CHUNK_WIDTH;
use crate::shared::blocks::Blocks;
use crate::shared::chunk::proto::ProtoChunk;
use crate::shared::rng::mersenne::MersenneTwister;
use chorus_block::block_id::MAGMA;

const CHUNK_CENTRE: i32 = 8;
const MESA_PROBE_Y: i32 = 0;
pub(super) const V2_REACH: i32 = 1;

#[derive(Default)]
pub struct Reaching {
    pub starts: Vec<(StructureKind, (i32, i32))>,
}

impl Structures {
    pub fn in_structure(&self, probe: &dyn StartProbe, (x, y, z): (i32, i32, i32), with_boxes: bool) -> Probed<bool> {
        let chunk = (x.div_euclid(CHUNK_WIDTH), z.div_euclid(CHUNK_WIDTH));
        let context = StructureContext { structures: self, probe };
        for builder in &self.builders {
            if builder.contains(&context, chunk, (x, y, z), with_boxes)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn in_structure_window(&self, probe: &dyn StartProbe, chunk: (i32, i32), margin: i32) -> Probed<()> {
        let context = StructureContext { structures: self, probe };
        for builder in &self.builders {
            builder.near_window(&context, chunk, margin)?;
        }
        Ok(())
    }

    pub(super) fn in_v2_box(&self, probe: &dyn StartProbe, kind: StructureKind, chunk: (i32, i32), at: (i32, i32, i32)) -> Probed<bool> {
        for start in self.starts_near(probe, kind, chunk, self.registry().reach(kind) + V2_REACH)? {
            let inside = self
                .v2_structure(probe, kind, start)?
                .is_some_and(|structure| parts_box(&structure.pieces).is_some_and(|bounds| bounds.contains(at.0, at.1, at.2)));
            if inside {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

fn parts_box(parts: &[super::layout::Part]) -> Option<BoundingBox> {
    parts.iter().map(|part| part.bounds).reduce(|all, part| all.union(&part))
}

impl Structures {
    pub fn reaching(&self, probe: &dyn StartProbe, chunk: (i32, i32)) -> Probed<Option<Reaching>> {
        let column = BoundingBox::chunk(chunk.0, chunk.1, probe.definition());
        let context = StructureContext { structures: self, probe };
        let mut starts: Vec<(StructureKind, (i32, i32))> = Vec::new();
        for kind in self.kinds() {
            for start in self.starts_near(probe, kind, chunk, self.registry().reach(kind))? {
                if !self.registry().rough_area(kind, start, probe.definition()).intersects(&column) {
                    continue;
                }
                let reaches = match self.builder_index(kind) {
                    Some(index) => self.builders[index].reaches(&context, kind, start, &column)?,
                    None => true,
                };
                if reaches {
                    starts.push((kind, start));
                }
            }
        }
        Ok((!starts.is_empty()).then_some(Reaching { starts }))
    }
}

impl Structures {
    pub fn centre_biome(probe: &dyn StartProbe, start: (i32, i32)) -> i32 {
        probe.biome(start.0 * CHUNK_WIDTH + CHUNK_CENTRE, probe.preliminary_surface(start), start.1 * CHUNK_WIDTH + CHUNK_CENTRE)
    }

    pub fn mineshaft_bounds(&self, probe: &dyn StartProbe, start: (i32, i32)) -> Probed<Option<BoundingBox>> {
        Ok(self.mineshaft(probe, start)?.map(|shaft| shaft.bounds))
    }

    pub fn mineshaft(&self, probe: &dyn StartProbe, start: (i32, i32)) -> Probed<Option<std::sync::Arc<Mineshaft>>> {
        Ok(self.starts_at(probe, start)?.get::<Mineshaft>(StructureKind::Mineshaft))
    }

    pub(super) fn build_mineshaft(&self, probe: &dyn StartProbe, start: (i32, i32)) -> Mineshaft {
        let mesa = self.tag("mesa").contains(&probe.biome(start.0 * CHUNK_WIDTH, MESA_PROBE_Y, start.1 * CHUNK_WIDTH));
        Mineshaft::generate(&self.mineshaft, start, mesa, probe.definition(), probe.sea_level(), &|quart_x, quart_z| {
            probe.surface_at_quart((quart_x, quart_z))
        })
    }

    pub(super) fn monument(&self, probe: &dyn StartProbe, start: (i32, i32)) -> Probed<Option<std::sync::Arc<Monument>>> {
        Ok(self.starts_at(probe, start)?.get::<Monument>(StructureKind::Monument))
    }

    pub(super) fn build_monument(&self, start: (i32, i32), sea_level: i32) -> Monument {
        Monument::generate(self.level_seed, start, sea_level)
    }

    pub(super) fn place_monuments(&self, probe: &dyn StartProbe, blocks: &Blocks, reaching: &Reaching, chunk: &mut ProtoChunk) -> Probed<()> {
        let mut monuments = Vec::new();
        for &(_, start) in reaching.starts.iter().filter(|&&(kind, _)| kind == StructureKind::Monument) {
            monuments.extend(self.monument(probe, start)?);
        }
        let (chunk_x, chunk_z) = (chunk.min_x().div_euclid(CHUNK_WIDTH), chunk.min_z().div_euclid(CHUNK_WIDTH));
        monument::place(blocks, &self.blocks.monument, chunk, &monuments, &mut self.seeds.random(chunk_x, chunk_z), probe.sea_level());
        Ok(())
    }

    pub(super) fn stronghold(&self, probe: &dyn StartProbe, start: (i32, i32)) -> Probed<Option<std::sync::Arc<Stronghold>>> {
        Ok(self.starts_at(probe, start)?.get::<Stronghold>(StructureKind::Stronghold))
    }

    pub fn build_stronghold(&self, start: (i32, i32), definition: DimensionDefinition, sea_level: i32) -> Stronghold {
        stronghold::generate(&mut self.seeds.blueprint(start), start, definition, sea_level)
    }

    pub(super) fn place_strongholds(&self, probe: &dyn StartProbe, blocks: &Blocks, reaching: &Reaching, chunk: &mut ProtoChunk) -> Probed<()> {
        let mut strongholds = Vec::new();
        for &(_, start) in reaching.starts.iter().filter(|&&(kind, _)| kind == StructureKind::Stronghold) {
            strongholds.extend(self.stronghold(probe, start)?);
        }
        let strongholds: Vec<&Stronghold> = strongholds.iter().map(|stronghold| stronghold.as_ref()).collect();
        let (chunk_x, chunk_z) = (chunk.min_x().div_euclid(CHUNK_WIDTH), chunk.min_z().div_euclid(CHUNK_WIDTH));
        stronghold::place(blocks, &self.blocks.stronghold, chunk, &strongholds, &mut self.seeds.random(chunk_x, chunk_z));
        Ok(())
    }

    pub(super) fn place_mineshafts(&self, probe: &dyn StartProbe, blocks: &Blocks, reaching: &Reaching, chunk: &mut ProtoChunk) -> Probed<()> {
        let mut starts: Vec<(i32, i32)> = reaching.starts.iter().filter(|&&(kind, _)| kind == StructureKind::Mineshaft).map(|&(_, start)| start).collect();
        starts.sort_unstable_by(|a, b| b.cmp(a));
        let mut shafts = Vec::new();
        for &start in &starts {
            shafts.extend(self.mineshaft(probe, start)?);
        }
        let shafts: Vec<&Mineshaft> = shafts.iter().map(|shaft| shaft.as_ref()).collect();
        let (chunk_x, chunk_z) = (chunk.min_x().div_euclid(CHUNK_WIDTH), chunk.min_z().div_euclid(CHUNK_WIDTH));
        let columns = |x: i32, z: i32| self.column_at(probe, x, z);
        let world = |x: i32, y: i32, z: i32| probe.staged_block(x, y, z);
        mineshaft::place(blocks, &self.blocks, chunk, &columns, &world, &shafts, &mut self.seeds.random(chunk_x, chunk_z))
    }

    pub(super) fn column_at(&self, probe: &dyn StartProbe, x: i32, z: i32) -> Probed<(i32, i32)> {
        let heights = probe.column_heights((x >> 4, z >> 4))?;
        let index = (((z & 15) << 4) | (x & 15)) as usize;
        Ok((heights.solid[index] as i32, heights.top[index] as i32))
    }

    pub(super) fn plan(&self, probe: &dyn StartProbe, blocks: &Blocks, kind: StructureKind, start: (i32, i32)) -> Probed<Option<std::sync::Arc<PlanWrites>>> {
        if let Some(planned) = self.starts_at(probe, start)?.get::<PlanWrites>(kind) {
            return Ok(Some(planned));
        }
        let context = StructureContext { structures: self, probe };
        let Some(index) = self.builder_index(kind) else { return Ok(None) };
        self.builders[index].plan(&context, blocks, kind, start)
    }

    pub(super) fn build_plan(
        &self,
        probe: &dyn StartProbe,
        blocks: &Blocks,
        start: (i32, i32),
        fill: impl FnOnce(&mut Plan<'_>, &Library<'_>, &mut MersenneTwister) -> Probed<()>,
    ) -> Probed<Option<std::sync::Arc<PlanWrites>>> {
        let (Some(palette), Some(templates)) = (self.blocks.palette.as_ref(), self.data.library.as_ref()) else {
            return Ok(None);
        };
        let library = Library { templates, palette };
        let columns = |x: i32, z: i32| self.column_at(probe, x, z);
        let mut plan = Plan::new(blocks, &self.blocks, &columns, probe);
        let mut random = self.seeds.random(start.0, start.1);
        fill(&mut plan, &library, &mut random)?;
        Ok(Some(std::sync::Arc::new(plan.finish())))
    }

    pub(super) fn apply(&self, blocks: &Blocks, chunk: &mut ProtoChunk, kind: StructureKind, plan: &PlanWrites) {
        let (min_x, min_z) = (chunk.min_x(), chunk.min_z());
        let in_chunk = |x: f64, z: f64| (0..CHUNK_WIDTH).contains(&(x.floor() as i32 - min_x)) && (0..CHUNK_WIDTH).contains(&(z.floor() as i32 - min_z));
        report_structure_entities(self.registry.name(kind), &plan.entities, in_chunk);
        for &(x, y, z, block) in &plan.writes {
            let (lx, lz) = (x - min_x, z - min_z);
            if !(0..CHUNK_WIDTH).contains(&lx) || !(0..CHUNK_WIDTH).contains(&lz) {
                continue;
            }
            let block = if chunk.get(lx, y, lz) == self.blocks.water { blocks.flooded(block) } else { block };
            if blocks.name_of(block) == MAGMA {
                chunk.mark_magma_tick(lx as u8, y as i16, lz as u8);
            }
            chunk.set(lx, y, lz, block);
        }
    }

    pub(super) fn ocean_ruin_writes(&self, probe: &dyn StartProbe, blocks: &Blocks, start: (i32, i32), chunk: (i32, i32)) -> Probed<Option<PlanWrites>> {
        let (Some(palette), Some(templates)) = (self.blocks.palette.as_ref(), self.data.library.as_ref()) else {
            return Ok(None);
        };
        let library = Library { templates, palette };
        let (_, random) = self
            .registry()
            .grid(StructureKind::Monument)
            .expect("the monument has a placement grid")
            .candidate_random(self.level_seed, (start.0 + 4, start.1 + 4));
        let site = RuinSite::generate(random, &library, start, Self::centre_biome(probe, start), probe.definition());
        if !site.reaches(chunk) {
            return Ok(None);
        }
        let columns = |x: i32, z: i32| self.column_at(probe, x, z);
        let mut surface = Surface {
            plan: Plan::new(blocks, &self.blocks, &columns, probe),
            chunk,
            seed: self.seeds.chunk(chunk.0, chunk.1),
        };
        site.place(&mut surface, &library)?;
        Ok(Some(surface.plan.finish()))
    }

    pub fn postprocess(&self, probe: &dyn StartProbe, blocks: &Blocks, reaching: &Reaching, chunk: &mut ProtoChunk) -> Probed<()> {
        let context = StructureContext { structures: self, probe };
        let mut placed: Vec<usize> = Vec::new();
        for &(kind, _) in &reaching.starts {
            let Some(index) = self.builder_index(kind) else { continue };
            if placed.contains(&index) {
                continue;
            }
            placed.push(index);
            self.builders[index].place(&context, blocks, reaching, chunk)?;
        }
        self.reconnect(probe, blocks, reaching, chunk)
    }
}
