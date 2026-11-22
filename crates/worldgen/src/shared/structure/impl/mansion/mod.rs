mod builder;
pub use builder::MansionBuilder;
mod chambers;
mod compass;
mod floor_plan;
mod layout;
mod roofs;

use std::sync::Arc;

use crate::shared::CHUNK_WIDTH;
use crate::shared::blocks::{AIR, BlockId, BlockTable, Blocks};
use crate::shared::chunk::proto::ProtoChunk;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::palette::register_named;
use crate::shared::structure::place::{Library, Options, Rotation};
use crate::shared::structure::plan::Plan;
use crate::shared::structure::template::{Template, TemplateLibrary};
use crate::shared::structure::{BoundingBox, Probed, Reaching, StartProbe, StructureKind, Structures};
use compass::{Pos, Side};
use floor_plan::FloorPlan;
use layout::{Layout, Mirror, Spec};

const ROTATIONS: i32 = 4;
const ORIGIN_IN_CHUNK: i32 = 8;
const SAMPLE_NEAR: i32 = 7;
const SAMPLE_FAR: i32 = 12;
const SAMPLE_NEAR_TURNED: i32 = 2;
const FOUNDATION_FLOOR: i32 = 2;
const TEMPLATE_PREFIX: &str = "mansion/";
const ALLAY_MARKER: &str = "Group of Allays";
const CHEST_MARKERS: [&str; 4] = ["ChestNorth", "ChestEast", "ChestSouth", "ChestWest"];
const STAIRS_STATES: &str = "minecraft:corner=none,upside_down_bit=0,weirdo_direction=";
const STAIRS_RUN_OFFSETS: [(i32, i32); 2] = [(9, 2), (9, 12)];
const STAIRS_RUN_SPACING: (i32, i32) = (1, 0);
const STAIRS_RUNS_PER_OFFSET: i32 = 2;

#[derive(Clone, Default)]
pub struct MansionBlocks {
    chests: [BlockId; 4],
    stairs: [BlockId; 4],
}

impl MansionBlocks {
    pub fn register(table: &mut BlockTable) -> Self {
        let sides = crate::shared::blocks::COMPASS;
        Self {
            chests: sides.map(|side| register_named(table, "chest", &format!("minecraft:cardinal_direction={side}"))),
            stairs: std::array::from_fn(|weirdo| register_named(table, "stone_stairs", &format!("{STAIRS_STATES}{weirdo}"))),
        }
    }

    fn chest(&self, side: Side) -> BlockId {
        self.chests[side as usize]
    }

    fn is_chest(&self, block: BlockId) -> bool {
        self.chests.contains(&block)
    }

    fn stairs_rising(&self, side: Side) -> BlockId {
        let weirdo = match side {
            Side::East => 0,
            Side::West => 1,
            Side::South => 2,
            Side::North => 3,
        };
        self.stairs[weirdo]
    }
}

pub struct Piece {
    template: String,
    at: Pos,
    rotation: Rotation,
    mirror: Mirror,
    pub bounds: BoundingBox,
}

pub struct Mansion {
    pub pieces: Vec<Piece>,
    origin: Pos,
    rotation: Rotation,
    pub bounds: BoundingBox,
}

fn orientation(rotation: Rotation, mirror: Mirror) -> (Rotation, bool) {
    match mirror {
        Mirror::None => (rotation, false),
        Mirror::LeftRight => (rotation, true),
        Mirror::FrontBack => (Rotation::from_index(rotation as i32 + 2), true),
    }
}

fn displaced(offset: Pos, rotation: Rotation, mirror: Mirror) -> Pos {
    let (turned, flipped) = orientation(rotation, mirror);
    turned.apply(if flipped { (offset.0, offset.1, -offset.2) } else { offset })
}

impl Piece {
    fn new(spec: Spec, template: &Template) -> Self {
        let (size_x, size_y, size_z) = template.size;
        let near = displaced((0, 0, 0), spec.rotation, spec.mirror);
        let far = displaced((size_x, size_y - 1, size_z), spec.rotation, spec.mirror);
        let (x, y, z) = spec.at;
        let bounds = BoundingBox::new(x + near.0.min(far.0), y, z + near.2.min(far.2), x + near.0.max(far.0), y + size_y - 1, z + near.2.max(far.2));
        Self {
            template: spec.template,
            at: spec.at,
            rotation: spec.rotation,
            mirror: spec.mirror,
            bounds,
        }
    }
}

impl Mansion {
    fn generate(templates: &TemplateLibrary, random: &mut MersenneTwister, start: (i32, i32), ground: &dyn Fn(i32, i32) -> Probed<i32>) -> Probed<Self> {
        let rotation = Rotation::from_index(random.next_int_bounded(ROTATIONS));
        let (far_x, far_z) = match rotation {
            Rotation::None => (SAMPLE_FAR, SAMPLE_FAR),
            Rotation::Clockwise90 => (SAMPLE_NEAR_TURNED, SAMPLE_FAR),
            Rotation::Clockwise180 => (SAMPLE_NEAR_TURNED, SAMPLE_NEAR_TURNED),
            Rotation::CounterClockwise90 => (SAMPLE_FAR, SAMPLE_NEAR_TURNED),
        };
        let (min_x, min_z) = (start.0 * CHUNK_WIDTH, start.1 * CHUNK_WIDTH);
        let samples = [(SAMPLE_NEAR, SAMPLE_NEAR), (SAMPLE_NEAR, far_z), (far_x, SAMPLE_NEAR), (far_x, far_z)];
        let mut lowest = i32::MAX;
        for &(x, z) in &samples {
            lowest = lowest.min(ground(min_x + x, min_z + z)?);
        }
        let origin = (min_x + ORIGIN_IN_CHUNK, lowest + 1, min_z + ORIGIN_IN_CHUNK);
        let plan = FloorPlan::generate(random);
        let specs = Layout::build(&plan, random, origin, rotation);
        let pieces: Vec<Piece> = specs
            .into_iter()
            .filter_map(|spec| {
                let template = templates.get(&format!("{TEMPLATE_PREFIX}{}", spec.template))?;
                Some(Piece::new(spec, &template))
            })
            .collect();
        let seed_box = BoundingBox::new(origin.0, origin.1, origin.2, origin.0, origin.1, origin.2);
        let bounds = pieces.iter().fold(seed_box, |all, piece| all.union(&piece.bounds));
        Ok(Self { pieces, origin, rotation, bounds })
    }
}

fn is_open(chunk: &ProtoChunk, block: BlockId) -> bool {
    block == AIR || chunk.is_fluid(block)
}

impl Structures {
    pub fn mansion_summary(&self, probe: &dyn StartProbe, start: (i32, i32)) -> Option<(i32, usize, BoundingBox, Pos)> {
        let mansion = self.mansion(probe, start).expect("supplied starts")?;
        Some((mansion.rotation as i32, mansion.pieces.len(), mansion.bounds, mansion.origin))
    }

    pub(in crate::shared::structure) fn mansion(&self, probe: &dyn StartProbe, start: (i32, i32)) -> Probed<Option<Arc<Mansion>>> {
        Ok(self.starts_at(probe, start)?.get::<Mansion>(StructureKind::Mansion))
    }

    pub(in crate::shared::structure) fn build_mansion(&self, probe: &dyn StartProbe, start: (i32, i32)) -> Probed<Option<Mansion>> {
        let Some(templates) = self.data.library.as_ref() else { return Ok(None) };
        let Some(grid) = self.registry().grid(StructureKind::Mansion) else { return Ok(None) };
        let (_, mut random) = grid.candidate_random(self.level_seed, start);
        let ground = |x: i32, z: i32| -> Probed<i32> {
            let (solid, top) = self.column_at(probe, x, z)?;
            Ok(solid.max(top))
        };
        let mansion = Mansion::generate(templates, &mut random, start, &ground)?;
        Ok(Some(mansion))
    }

    pub(in crate::shared::structure) fn place_mansions(&self, probe: &dyn StartProbe, blocks: &Blocks, reaching: &Reaching, chunk: &mut ProtoChunk) -> Probed<()> {
        let (Some(palette), Some(templates)) = (self.blocks.palette.as_ref(), self.data.library.as_ref()) else {
            return Ok(());
        };
        let library = Library { templates, palette };
        let (chunk_x, chunk_z) = (chunk.min_x().div_euclid(CHUNK_WIDTH), chunk.min_z().div_euclid(CHUNK_WIDTH));
        let column = BoundingBox::chunk(chunk_x, chunk_z, chunk.definition());
        let columns = |x: i32, z: i32| self.column_at(probe, x, z);
        let mut mansions = Vec::new();
        for &(_, start) in reaching.starts.iter().filter(|&&(kind, _)| kind == StructureKind::Mansion) {
            mansions.extend(self.mansion(probe, start)?);
        }
        for mansion in mansions {
            let mut plan = Plan::new(blocks, &self.blocks, &columns, probe);
            plan.clip = Some(column);
            let touching: Vec<&Piece> = mansion.pieces.iter().filter(|piece| piece.bounds.intersects(&column)).collect();
            for piece in &touching {
                self.place_piece(&library, &mut plan, piece)?;
            }
            self.apply(blocks, chunk, StructureKind::Mansion, &plan.finish());
            self.lay_cobblestone_base(chunk, &mansion, &touching);
            self.lay_entrance_stairs(chunk, &mansion);
        }
        Ok(())
    }

    fn place_piece(&self, library: &Library, plan: &mut Plan, piece: &Piece) -> Probed<()> {
        let Some(template) = library.templates.get(&format!("{TEMPLATE_PREFIX}{}", piece.template)) else {
            return Ok(());
        };
        let (rotation, flipped) = orientation(piece.rotation, piece.mirror);
        library.place(
            plan,
            &template,
            piece.at,
            rotation,
            Options {
                mirror: flipped,
                ..Options::default()
            },
        );
        for (index, name) in &template.markers {
            let block = template.blocks[*index];
            let offset = displaced((block.x as i32, block.y as i32, block.z as i32), piece.rotation, piece.mirror);
            let (x, y, z) = (piece.at.0 + offset.0, piece.at.1 + offset.1, piece.at.2 + offset.2);
            if name == ALLAY_MARKER {
                plan.set(x, y, z, AIR);
            } else if let Some(side) = CHEST_MARKERS.iter().position(|marker| marker == name) {
                let swapped = match piece.mirror {
                    Mirror::LeftRight => side % 2 == 0,
                    Mirror::FrontBack => side % 2 == 1,
                    Mirror::None => false,
                };
                let turned = Side::ALL[side].quarter_turns((if swapped { 2 } else { 0 }) + piece.rotation as i32);
                if !self.blocks.mansion.is_chest(plan.get(x, y, z)?) {
                    plan.set(x, y, z, self.blocks.mansion.chest(turned));
                }
            }
        }
        Ok(())
    }

    fn fill_down(&self, chunk: &mut ProtoChunk, local: (i32, i32), from: i32) {
        let mut y = from;
        while y >= FOUNDATION_FLOOR {
            let block = chunk.get(local.0, y, local.1);
            if !is_open(chunk, block) {
                break;
            }
            chunk.set(local.0, y, local.1, self.blocks.cobblestone);
            y -= 1;
        }
    }

    fn lay_cobblestone_base(&self, chunk: &mut ProtoChunk, mansion: &Mansion, touching: &[&Piece]) {
        let floor = mansion.bounds.y0;
        if floor <= FOUNDATION_FLOOR {
            return;
        }
        let (min_x, min_z) = (chunk.min_x(), chunk.min_z());
        for local_x in 0..CHUNK_WIDTH {
            for local_z in 0..CHUNK_WIDTH {
                let (x, z) = (min_x + local_x, min_z + local_z);
                if chunk.get(local_x, floor, local_z) != AIR && mansion.bounds.contains(x, floor, z) && touching.iter().any(|piece| piece.bounds.contains(x, floor, z)) {
                    self.fill_down(chunk, (local_x, local_z), floor - 1);
                }
            }
        }
    }

    fn lay_entrance_stairs(&self, chunk: &mut ProtoChunk, mansion: &Mansion) {
        let (origin_x, origin_y, origin_z) = mansion.origin;
        let rotation = mansion.rotation;
        for (index, (offset_x, offset_z)) in STAIRS_RUN_OFFSETS.into_iter().enumerate() {
            let heading = if index == 0 { Side::North } else { Side::South };
            let run = heading.quarter_turns(rotation as i32);
            let stairs = self.blocks.mansion.stairs_rising(run.behind());
            for k in 0..STAIRS_RUNS_PER_OFFSET {
                let (spaced_x, _, spaced_z) = rotation.apply((offset_x + STAIRS_RUN_SPACING.0 * k, 0, offset_z + STAIRS_RUN_SPACING.1 * k));
                self.descend(chunk, (origin_x + spaced_x, origin_y - 1, origin_z + spaced_z), run.delta(), stairs);
            }
        }
    }

    fn descend(&self, chunk: &mut ProtoChunk, (mut x, mut y, mut z): Pos, (dx, dz): (i32, i32), stairs: BlockId) {
        let (min_x, min_z) = (chunk.min_x(), chunk.min_z());
        let inside = |x: i32, z: i32| (min_x..min_x + CHUNK_WIDTH).contains(&x) && (min_z..min_z + CHUNK_WIDTH).contains(&z);
        if !inside(x, z) || chunk.get(x - min_x, y, z - min_z) != AIR {
            return;
        }
        loop {
            chunk.set(x - min_x, y, z - min_z, stairs);
            if y > FOUNDATION_FLOOR {
                self.fill_down(chunk, (x - min_x, z - min_z), y - 1);
            }
            (x, y, z) = (x + dx, y - 1, z + dz);
            if !inside(x, z) || chunk.get(x - min_x, y, z - min_z) != AIR {
                break;
            }
        }
    }
}
