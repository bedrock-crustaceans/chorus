use crate::shared::structure::builder::{Built, Region, StructureBuilder, StructureContext, covers, erase, passes_filter};
use std::sync::Arc;

use crate::shared::blocks::{AIR, Blocks};
use crate::shared::chunk::proto::ProtoChunk;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::place::{Library, Options, Rotation};
use crate::shared::structure::plan::Plan;
use crate::shared::structure::template::FULL_CUBE;
use crate::shared::structure::{BoundingBox, Probed, Reaching, StartProbe, StructureKind, Structures};
use crate::shared::{CHUNK_MASK, CHUNK_WIDTH};
use chorus_level::dimension_definition::DimensionDefinition;

const VIEW_RADIUS: u32 = 1;
const ROTATIONS: i32 = 4;
const TOWER: &str = "pillageroutpost/watchtower";
const OVERGROWN_TOWER: &str = "pillageroutpost/watchtower_overgrown";
const OVERGROWTH_INTEGRITY: f32 = 0.05;
const FEATURES: [&str; 7] = [
    "pillageroutpost/feature_cage1",
    "pillageroutpost/feature_cage2",
    "pillageroutpost/feature_cage_with_allays",
    "pillageroutpost/feature_logs",
    "pillageroutpost/feature_targets",
    "pillageroutpost/feature_tent1",
    "pillageroutpost/feature_tent2",
];
const FEATURE_SPOTS: [(i32, i32); 4] = [(16, 16), (-16, 16), (16, -16), (-16, -16)];
const SPOT_RANGE: i32 = 8;
const SPOT_SHIFT: i32 = 4;
const FEATURE_CHANCE: f32 = 0.9;
const CENTRE_OFFSET: i32 = 8;
const REACH: i32 = 15;
const MAX_SPREAD: i32 = 9;
const TOWER_SINK: i32 = 4;
const FOUNDATION_FLOOR: i32 = 2;

pub struct OutpostPiece {
    template: &'static str,
    origin: (i32, i32),
    rotation: Rotation,
    integrity: f32,
    feature: bool,
    pub bounds: BoundingBox,
}

pub struct Outpost {
    pub pieces: Vec<OutpostPiece>,
}

fn turned_box(origin: (i32, i32), rotation: Rotation, (size_x, _, size_z): (i32, i32, i32), definition: DimensionDefinition) -> BoundingBox {
    let (dx, _, dz) = rotation.apply((size_x, 0, size_z));
    BoundingBox::new(
        origin.0.min(origin.0 + dx),
        definition.min_y,
        origin.1.min(origin.1 + dz),
        origin.0.max(origin.0 + dx),
        definition.max_y(),
        origin.1.max(origin.1 + dz),
    )
}

impl Outpost {
    fn generate(library: &Library, random: &mut MersenneTwister, start: (i32, i32), definition: DimensionDefinition) -> Self {
        let mut pieces = Vec::new();
        let mut push = |template: &'static str, origin: (i32, i32), rotation: Rotation, integrity: f32, feature: bool| {
            if let Some(found) = library.templates.get(template) {
                pieces.push(OutpostPiece {
                    template,
                    origin,
                    rotation,
                    integrity,
                    feature,
                    bounds: turned_box(origin, rotation, found.size, definition),
                });
            }
        };
        let turn = random.next_int_bounded(ROTATIONS);
        let rotation = Rotation::from_index(turn);
        let (x0, z0) = (start.0 * CHUNK_WIDTH, start.1 * CHUNK_WIDTH);
        push(TOWER, (x0, z0), rotation, 1.0, false);
        push(OVERGROWN_TOWER, (x0, z0), rotation, OVERGROWTH_INTEGRITY, false);
        let (dx, dz) = match turn {
            1 => (-REACH, REACH),
            2 => (-REACH, -REACH),
            3 => (REACH, -REACH),
            _ => (REACH, REACH),
        };
        let centre = (x0.min(x0 + dx) + CENTRE_OFFSET, z0.min(z0 + dz) + CENTRE_OFFSET);
        let spots: Vec<(i32, i32)> = FEATURE_SPOTS
            .iter()
            .map(|&(spot_x, spot_z)| {
                let x = random.next_int_bounded(SPOT_RANGE) + spot_x - SPOT_SHIFT;
                (x, random.next_int_bounded(SPOT_RANGE) + spot_z - SPOT_SHIFT)
            })
            .collect();
        for (spot_x, spot_z) in spots {
            if random.next_float() >= FEATURE_CHANCE {
                continue;
            }
            let rotation = Rotation::from_index(random.next_int_bounded(ROTATIONS));
            let template = FEATURES[random.next_int_bounded(FEATURES.len() as i32) as usize];
            push(template, (centre.0 + spot_x, centre.1 + spot_z), rotation, 1.0, true);
        }
        Self { pieces }
    }
}

struct Surface<'a> {
    chunk: &'a ProtoChunk,
    columns: &'a dyn Fn(i32, i32) -> Probed<(i32, i32)>,
    view: (i32, i32),
}

impl Surface<'_> {
    fn in_view(&self, x: i32, z: i32) -> bool {
        (x >> 4).abs_diff(self.view.0) <= VIEW_RADIUS && (z >> 4).abs_diff(self.view.1) <= VIEW_RADIUS
    }

    fn height(&self, x: i32, z: i32) -> Probed<(i32, bool)> {
        if !self.in_view(x, z) {
            return Ok((self.chunk.min_y, false));
        }
        if (x >> 4, z >> 4) == self.view {
            let (local_x, local_z) = (x & CHUNK_MASK, z & CHUNK_MASK);
            let top = self.chunk.top(local_x, local_z).max(self.chunk.min_y);
            let wet = top > self.chunk.min_y && self.chunk.is_fluid(self.chunk.get(local_x, top - 1, local_z));
            return Ok((top, wet));
        }
        let (solid, top) = (self.columns)(x, z)?;
        Ok((top.max(solid), top > solid))
    }
}

impl OutpostPiece {
    fn level(&self, surface: &Surface) -> Probed<Option<i32>> {
        let b = &self.bounds;
        let (half_x, half_z) = ((b.x1 - b.x0 + 1) >> 1, (b.z1 - b.z0 + 1) >> 1);
        let (middle_x, middle_z) = (b.x0 + half_x, b.z0 + half_z);
        let points = [(middle_x, middle_z), (middle_x + half_x, middle_z), (middle_x, middle_z + half_z), (b.x0, middle_z), (middle_x, b.z0)];
        let min_y = surface.chunk.min_y;
        let mut columns = [(min_y, false); 5];
        for (slot, (x, z)) in columns.iter_mut().zip(points) {
            *slot = surface.height(x, z)?;
        }
        let high = columns.iter().map(|&(top, _)| top).max().unwrap_or(min_y);
        let low = columns.iter().map(|&(top, _)| top).min().unwrap_or(min_y).max(min_y);
        let level = high - low < MAX_SPREAD;
        Ok(if self.feature {
            let wet = columns.iter().any(|&(_, wet)| wet);
            (level && !wet).then_some(columns[0].0)
        } else if level {
            Some((low + high) / 2)
        } else {
            Some(high - TOWER_SINK)
        })
    }
}

impl Structures {
    pub(in crate::shared::structure) fn outpost(&self, probe: &dyn StartProbe, start: (i32, i32)) -> Probed<Option<Arc<Outpost>>> {
        Ok(self.starts_at(probe, start)?.get::<Outpost>(StructureKind::Outpost))
    }

    pub(in crate::shared::structure) fn build_outpost(&self, start: (i32, i32), definition: DimensionDefinition) -> Option<Outpost> {
        let (palette, templates) = (self.blocks.palette.as_ref()?, self.data.library.as_ref()?);
        let library = Library { templates, palette };
        let (_, mut random) = self.registry().grid(StructureKind::Outpost)?.candidate_random(self.level_seed, start);
        Some(Outpost::generate(&library, &mut random, start, definition))
    }

    pub(in crate::shared::structure) fn place_outposts(&self, probe: &dyn StartProbe, blocks: &Blocks, reaching: &Reaching, chunk: &mut ProtoChunk) -> Probed<()> {
        let (Some(palette), Some(templates)) = (self.blocks.palette.as_ref(), self.data.library.as_ref()) else {
            return Ok(());
        };
        let library = Library { templates, palette };
        let (chunk_x, chunk_z) = (chunk.min_x().div_euclid(CHUNK_WIDTH), chunk.min_z().div_euclid(CHUNK_WIDTH));
        let bounds = BoundingBox::chunk(chunk_x, chunk_z, chunk.definition());
        let seed = self.seeds.chunk(chunk_x, chunk_z);
        let columns = |x: i32, z: i32| self.column_at(probe, x, z);
        let mut outposts: Vec<Arc<Outpost>> = Vec::new();
        for &(_, start) in reaching.starts.iter().filter(|&&(kind, _)| kind == StructureKind::Outpost) {
            outposts.extend(self.outpost(probe, start)?);
        }
        let surface = Surface {
            chunk,
            columns: &columns,
            view: (chunk_x, chunk_z),
        };
        let mut placed: Vec<Vec<(&OutpostPiece, i32)>> = Vec::new();
        for outpost in &outposts {
            let mut levelled = Vec::new();
            for piece in outpost.pieces.iter().filter(|piece| piece.bounds.intersects(&bounds)) {
                if let Some(y) = piece.level(&surface)? {
                    levelled.push((piece, y));
                }
            }
            placed.push(levelled);
        }
        for pieces in placed {
            for &(piece, y) in &pieces {
                let Some(template) = templates.get(piece.template) else { continue };
                let mut plan = Plan::new(blocks, &self.blocks, &columns, probe);
                let integrity = (piece.integrity < 1.0).then_some((piece.integrity, seed));
                library.place(&mut plan, &template, (piece.origin.0, y, piece.origin.1), piece.rotation, Options { integrity, ..Options::default() });
                self.apply(blocks, chunk, StructureKind::Outpost, &plan.finish());
            }
            for &(piece, y) in &pieces {
                self.lay_foundation(blocks, chunk, &piece.bounds, y);
            }
        }
        Ok(())
    }

    fn lay_foundation(&self, blocks: &Blocks, chunk: &mut ProtoChunk, piece: &BoundingBox, y: i32) {
        let (min_x, min_z) = (chunk.min_x(), chunk.min_z());
        let palette = self.blocks.palette.as_ref();
        for x in piece.x0.max(min_x)..=piece.x1.min(min_x + CHUNK_WIDTH - 1) {
            for z in piece.z0.max(min_z)..=piece.z1.min(min_z + CHUNK_WIDTH - 1) {
                let (local_x, local_z) = (x - min_x, z - min_z);
                let top = chunk.get(local_x, y, local_z);
                if top == AIR {
                    continue;
                }
                let full = match palette.and_then(|palette| palette.flags_of(top)) {
                    Some(flags) => flags & FULL_CUBE != 0,
                    None => blocks.entry(top).full,
                };
                let fill = if full { top } else { self.blocks.cobblestone };
                let mut below = y - 1;
                while below >= FOUNDATION_FLOOR {
                    let block = chunk.get(local_x, below, local_z);
                    if block != AIR && !chunk.is_fluid(block) {
                        break;
                    }
                    chunk.set(local_x, below, local_z, fill);
                    below -= 1;
                }
            }
        }
    }
}

pub struct OutpostBuilder;

impl StructureBuilder for OutpostBuilder {
    fn is_start(&self, context: &StructureContext, _: StructureKind, chunk: (i32, i32)) -> Probed<bool> {
        Ok(passes_filter(context, StructureKind::Outpost, chunk))
    }

    fn build(&self, context: &StructureContext, _: StructureKind, chunk: (i32, i32)) -> Probed<Option<Built>> {
        Ok(context.structures.build_outpost(chunk, context.probe.definition()).map(erase))
    }

    fn reaches(&self, context: &StructureContext, _: StructureKind, start: (i32, i32), area: &BoundingBox) -> Probed<bool> {
        Ok(context
            .structures
            .outpost(context.probe, start)?
            .is_some_and(|outpost| covers(outpost.pieces.iter().map(|piece| &piece.bounds), area)))
    }

    fn regions(&self, context: &StructureContext, _: &Blocks, _: StructureKind, start: (i32, i32)) -> Probed<Vec<Region>> {
        Ok(context
            .structures
            .outpost(context.probe, start)?
            .map(|built| {
                built
                    .pieces
                    .iter()
                    .map(|piece| Region {
                        bounds: piece.bounds,
                        template: false,
                        settled_walls: false,
                        keep_template_post: false,
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    fn place(&self, context: &StructureContext, blocks: &Blocks, reaching: &Reaching, chunk: &mut ProtoChunk) -> Probed<()> {
        context.structures.place_outposts(context.probe, blocks, reaching, chunk)
    }

    fn neighbour_rank(&self) -> Option<u8> {
        Some(5)
    }
}
