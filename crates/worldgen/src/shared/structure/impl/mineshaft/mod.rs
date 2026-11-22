use chorus_level::dimension_definition::DimensionDefinition;
mod builder;
pub use builder::MineshaftBuilder;
mod blocks;
mod canvas;
mod corridor;
mod frame;
mod rooms;

pub use blocks::MineshaftBlocks;
use canvas::{Canvas, Cell};
use corridor::Corridor;
use frame::{Frame, NORTH, SOUTH, WEST};

use crate::shared::CHUNK_WIDTH;
use crate::shared::blocks::{BlockId, Blocks};
use crate::shared::chunk::proto::ProtoChunk;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::{BoundingBox, MineshaftSeed, Probed, StructureBlocks};

const DEEP_DARK: i32 = 190;
const MAX_DEPTH: i32 = 8;
const MAX_DISTANCE: i32 = 80;
const ROOM_SPAN: i32 = 6;
const ROOM_MIN: i32 = 2;
const ROOM_BASE_Y: i32 = 50;
const ROOM_FIRST_SPAN: i32 = 9;
const ROOM_BASE_HEIGHT: i32 = 54;
const CROSSING_ROLL: i32 = 80;
const STAIRS_ROLL: i32 = 70;
const ROLL_RANGE: i32 = 100;
const TWO_FLOOR_ODDS: i32 = 4;
const CROSSING_UPPER_FLOOR: i32 = 4;
const STAIRS_DROP: i32 = 5;
const STAIRS_LENGTH: i32 = 8;
const MESA_ROOM_CHANCE: f32 = 0.5;
const FLOOR_MARGIN: i32 = 10;
const QUART_SHIFT: i32 = 2;
const NO_SURFACE: i32 = 0x7fff;

#[derive(Debug, Clone, Copy)]
pub struct Materials {
    dark: bool,
    room_chance: f32,
}

impl Materials {
    fn planks(self, shaft: &MineshaftBlocks) -> BlockId {
        if self.dark { shaft.dark_oak_planks } else { shaft.oak_planks }
    }

    fn log(self, shaft: &MineshaftBlocks) -> BlockId {
        if self.dark { shaft.dark_oak_log } else { shaft.oak_log }
    }

    fn fences(self, shaft: &MineshaftBlocks) -> &[BlockId; 16] {
        if self.dark { &shaft.dark_oak_fences } else { &shaft.oak_fences }
    }

    fn is_fence(self, shaft: &MineshaftBlocks, block: BlockId) -> bool {
        self.fences(shaft).contains(&block)
    }
}

#[derive(Debug, Clone)]
enum Kind {
    Room(Vec<BoundingBox>),
    Corridor(Corridor),
    Crossing { direction: u8, two_floors: bool },
    Stairs,
}

#[derive(Debug, Clone)]
struct Piece {
    frame: Frame,
    kind: Kind,
}

#[derive(Debug, Clone)]
pub struct Mineshaft {
    pieces: Vec<Piece>,
    materials: Materials,
    pub bounds: BoundingBox,
}

struct Builder {
    pieces: Vec<Piece>,
    materials: Materials,
    start: BoundingBox,
}

impl Mineshaft {
    pub fn generate(seed: &MineshaftSeed, start: (i32, i32), mesa: bool, definition: DimensionDefinition, sea_level: i32, surface: &dyn Fn(i32, i32) -> i32) -> Self {
        let (_, mut random) = seed.random(start);
        let materials = Materials {
            dark: mesa,
            room_chance: if mesa { MESA_ROOM_CHANCE } else { 1.0 },
        };
        let (x, z) = (start.0 * CHUNK_WIDTH, start.1 * CHUNK_WIDTH);
        let extra_z = random.next_int_bounded(ROOM_SPAN);
        let extra_y = random.next_int_bounded(ROOM_SPAN);
        let extra_x = random.next_int_bounded(ROOM_SPAN);
        let room = BoundingBox::new(
            x + ROOM_MIN,
            ROOM_BASE_Y,
            z + ROOM_MIN,
            x + ROOM_FIRST_SPAN + extra_x,
            ROOM_BASE_HEIGHT + extra_y,
            z + ROOM_FIRST_SPAN + extra_z,
        );
        let mut builder = Builder {
            pieces: vec![Piece {
                frame: Frame {
                    bounds: room,
                    orientation: None,
                    depth: 0,
                },
                kind: Kind::Room(Vec::new()),
            }],
            materials,
            start: room,
        };
        rooms::room_children(&mut builder, 0, &mut random);
        let bounds = builder.pieces.iter().fold(room, |all, piece| all.union(&piece.frame.bounds));
        let middle = |low: i32, high: i32| low + (high - low + 1) / 2;
        let offset = if mesa {
            let top = surface(middle(bounds.x0, bounds.x1) >> QUART_SHIFT, middle(bounds.z0, bounds.z1) >> QUART_SHIFT);
            let top = if top == NO_SURFACE { sea_level } else { top };
            let y = if sea_level < top { sea_level + random.next_int_bounded(top - sea_level + 1) } else { sea_level };
            y - middle(bounds.y0, bounds.y1)
        } else {
            let mut y = definition.min_y + (bounds.y1 - bounds.y0) + 2;
            if y < sea_level - FLOOR_MARGIN {
                y += random.next_int_bounded(sea_level - FLOOR_MARGIN - y);
            }
            y - bounds.y1
        };
        for piece in &mut builder.pieces {
            piece.frame.bounds = piece.frame.bounds.shifted_y(offset);
            if let Kind::Room(entrances) = &mut piece.kind {
                entrances.iter_mut().for_each(|entrance| *entrance = entrance.shifted_y(offset));
            }
        }
        Self {
            pieces: builder.pieces,
            materials,
            bounds: bounds.shifted_y(offset),
        }
    }

    pub fn describe(&self) -> Vec<String> {
        self.pieces
            .iter()
            .map(|piece| format!("{:?} {:?} {:?} depth {}", piece.kind, piece.frame.bounds, piece.frame.orientation, piece.frame.depth))
            .collect()
    }

    pub fn post_process(&self, canvas: &mut Canvas, random: &mut MersenneTwister) -> Probed<()> {
        let column = canvas.bounds;
        for piece in self.pieces.iter().filter(|piece| piece.frame.bounds.intersects(&column)) {
            if invalid_location(&piece.frame, canvas)? {
                continue;
            }
            let frame = &piece.frame;
            match &piece.kind {
                Kind::Room(entrances) => rooms::room(frame, entrances, canvas)?,
                Kind::Corridor(corridor) => corridor.post_process(frame, self.materials, canvas, random)?,
                Kind::Crossing { two_floors, .. } => rooms::crossing(frame, *two_floors, self.materials, canvas)?,
                Kind::Stairs => rooms::stairs(frame, canvas)?,
            }
        }
        Ok(())
    }
}

impl Builder {
    fn collides(&self, bounds: &BoundingBox) -> bool {
        self.pieces.iter().any(|piece| piece.frame.bounds.intersects(bounds))
    }

    fn add(&mut self, random: &mut MersenneTwister, (x, y, z): Cell, direction: u8, depth: i32) {
        if depth > MAX_DEPTH || (x - self.start.x0).abs() > MAX_DISTANCE || (z - self.start.z0).abs() > MAX_DISTANCE {
            return;
        }
        let depth = depth + 1;
        let roll = random.next_int_bounded(ROLL_RANGE);
        let piece = if roll >= CROSSING_ROLL {
            let two_floors = random.next_int_bounded(TWO_FLOOR_ODDS) == 0;
            let top = y + 2 + if two_floors { CROSSING_UPPER_FLOOR } else { 0 };
            let bounds = match direction {
                SOUTH => BoundingBox::new(x - 1, y, z, x + 3, top, z + 4),
                WEST => BoundingBox::new(x - 4, y, z - 1, x, top, z + 3),
                NORTH => BoundingBox::new(x - 1, y, z - 4, x + 3, top, z),
                _ => BoundingBox::new(x, y, z - 1, x + 4, top, z + 3),
            };
            (!self.collides(&bounds)).then_some(Piece {
                frame: Frame { bounds, orientation: None, depth },
                kind: Kind::Crossing { direction, two_floors },
            })
        } else if roll >= STAIRS_ROLL {
            let bounds = match direction {
                SOUTH => BoundingBox::new(x, y - STAIRS_DROP, z, x + 2, y + 2, z + STAIRS_LENGTH),
                WEST => BoundingBox::new(x - STAIRS_LENGTH, y - STAIRS_DROP, z, x, y + 2, z + 2),
                NORTH => BoundingBox::new(x, y - STAIRS_DROP, z - STAIRS_LENGTH, x + 2, y + 2, z),
                _ => BoundingBox::new(x, y - STAIRS_DROP, z, x + STAIRS_LENGTH, y + 2, z + 2),
            };
            (!self.collides(&bounds)).then_some(Piece {
                frame: Frame {
                    bounds,
                    orientation: Some(direction),
                    depth,
                },
                kind: Kind::Stairs,
            })
        } else {
            Corridor::find(self, random, (x, y, z), direction, depth)
        };
        let Some(piece) = piece else { return };
        let index = self.pieces.len();
        let kind = piece.kind.clone();
        self.pieces.push(piece);
        match kind {
            Kind::Corridor(_) => corridor::children(self, index, random),
            Kind::Crossing { .. } => rooms::crossing_children(self, index, random),
            Kind::Stairs => rooms::stairs_children(self, index, random),
            Kind::Room(_) => {}
        }
    }
}

fn invalid_location(frame: &Frame, canvas: &Canvas) -> Probed<bool> {
    let (chunk, bounds) = (canvas.bounds, frame.bounds);
    let low = ((bounds.x0 - 1).max(chunk.x0), (bounds.y0 - 1).max(chunk.y0), (bounds.z0 - 1).max(chunk.z0));
    let high = ((bounds.x1 + 1).min(chunk.x1), (bounds.y1 + 1).min(chunk.y1), (bounds.z1 + 1).min(chunk.z1));
    let centre = ((low.0 + high.0) / 2, (low.1 + high.1) / 2, (low.2 + high.2) / 2);
    if canvas.chunk.biome_at(centre.0 & (CHUNK_WIDTH - 1), centre.1, centre.2 & (CHUNK_WIDTH - 1)).bedrock_id() == DEEP_DARK {
        return Ok(true);
    }
    for x in low.0..=high.0 {
        for y in low.1..=high.1 {
            for z in low.2..=high.2 {
                let face = x == low.0 || x == high.0 || y == low.1 || y == high.1 || z == low.2 || z == high.2;
                if face && canvas.is_fluid(canvas.get(x, y, z)?) {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

fn fence_at(materials: Materials, canvas: &Canvas, (x, y, z): Cell) -> Probed<BlockId> {
    let joins = |block: BlockId| materials.is_fence(canvas.shaft, block) || canvas.full_solid(block);
    let mut bits = 0usize;
    for (dx, dz, bit) in [(1, 0, 1usize), (0, -1, 2), (0, 1, 4), (-1, 0, 8)] {
        if joins(canvas.get(x + dx, y, z + dz)?) {
            bits += bit;
        }
    }
    Ok(materials.fences(canvas.shaft)[bits])
}

pub fn place(
    blocks: &Blocks,
    ids: &StructureBlocks,
    chunk: &mut ProtoChunk,
    columns: &dyn Fn(i32, i32) -> Probed<(i32, i32)>,
    world: &dyn Fn(i32, i32, i32) -> Probed<Option<BlockId>>,
    shafts: &[&Mineshaft],
    random: &mut MersenneTwister,
) -> Probed<()> {
    let mut canvas = Canvas::new(chunk, blocks, ids, columns, world);
    for shaft in shafts {
        shaft.post_process(&mut canvas, random)?;
    }
    Ok(())
}
