use std::sync::Arc;

use crate::shared::blocks::{AIR, BlockId, Blocks};
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::layout::{EntryKind, Palette, Part, PoolCatalogue, QueuedSpec, Sample, process};
use crate::shared::structure::{BoundingBox, CHESTS, PaletteBlocks, StateKind, TemplateLibrary, entities_at, is_falling_block, report_structure_entities};

use super::super::NetherChunk;
use super::{CHUNK_MASK, Cell};

const BASTION: &str = "bastion";
const START_POOLS: [&str; 4] = ["bastion/bridge/start", "bastion/treasure/starters", "bastion/hoglin_stable/origin", "bastion/units/air_base"];
const GROUND: i32 = super::super::SEA_LEVEL + 1;
const MAX_DEPTH: i32 = 15;
const MAX_DISTANCE: i32 = 80;
const PLUG_RADIUS: i32 = 12;
const FLOOR: i32 = 0;
const NAMESPACE_LENGTH: usize = "minecraft:".len();
const PLUG_HOLES: u8 = 2;

pub const REACH: i32 = 7;

pub struct Bastion {
    pub pieces: Vec<Part>,
    pub pivot: (i32, i32, i32),
}

pub fn generate(random: &mut MersenneTwister, start: (i32, i32), catalogue: &PoolCatalogue, library: &TemplateLibrary) -> Bastion {
    let spec = QueuedSpec {
        pools: &START_POOLS,
        anchor: None,
        max_depth: MAX_DEPTH,
        radius: MAX_DISTANCE,
        ground: GROUND,
    };
    let pieces = spec.assemble(catalogue, library, random, start);
    let pivot = pieces[0].origin;
    Bastion { pieces, pivot }
}

impl Bastion {
    pub fn reaches(&self, column: &BoundingBox) -> bool {
        self.pieces.iter().any(|piece| piece.bounds.intersects(column))
    }
}

pub struct Canvas<'a> {
    chunk: &'a mut NetherChunk,
    chunk_x: i32,
    chunk_z: i32,
    blocks: &'a Blocks,
    neighbour: &'a dyn Fn(i32, i32) -> Arc<NetherChunk>,
}

impl<'a> Canvas<'a> {
    pub fn new(chunk: &'a mut NetherChunk, (chunk_x, chunk_z): (i32, i32), blocks: &'a Blocks, neighbour: &'a dyn Fn(i32, i32) -> Arc<NetherChunk>) -> Self {
        Self {
            chunk,
            chunk_x,
            chunk_z,
            blocks,
            neighbour,
        }
    }

    fn inside(&self, x: i32, z: i32) -> bool {
        x >> 4 == self.chunk_x && z >> 4 == self.chunk_z
    }

    fn in_view(&self, x: i32, z: i32) -> bool {
        ((x >> 4) - self.chunk_x).abs() <= 1 && ((z >> 4) - self.chunk_z).abs() <= 1
    }

    pub fn get(&self, x: i32, y: i32, z: i32) -> BlockId {
        if !self.chunk.contains_y(y) || !self.in_view(x, z) {
            AIR
        } else if self.inside(x, z) {
            self.chunk.get(x & CHUNK_MASK, y, z & CHUNK_MASK)
        } else {
            (self.neighbour)(x >> 4, z >> 4).get(x & CHUNK_MASK, y, z & CHUNK_MASK)
        }
    }

    pub fn set(&mut self, x: i32, y: i32, z: i32, block: BlockId) {
        if self.chunk.contains_y(y) && self.inside(x, z) {
            self.chunk.set(x & CHUNK_MASK, y, z & CHUNK_MASK, block);
        }
    }

    fn short_name(&self, block: BlockId) -> &str {
        &self.blocks.name_of(block)[NAMESPACE_LENGTH..]
    }

    fn falls(&self, block: BlockId) -> bool {
        let name = self.short_name(block);
        is_falling_block(name)
    }
}

fn sample(blocks: &Blocks, block: BlockId) -> Sample<'_> {
    Sample {
        name: &blocks.name_of(block)[NAMESPACE_LENGTH..],
        default_state: true,
    }
}

pub fn place(
    bastion: &Bastion,
    chunk: &mut NetherChunk,
    (chunk_x, chunk_z): (i32, i32),
    ids: &super::super::NetherBlocks,
    blocks: &Blocks,
    palette: &PaletteBlocks,
    library: &TemplateLibrary,
    catalogue: &PoolCatalogue,
    neighbour: &dyn Fn(i32, i32) -> Arc<NetherChunk>,
    seed: u32,
    random: &mut MersenneTwister,
) {
    let column = BoundingBox::chunk(chunk_x, chunk_z, chunk.definition());
    let rules = Palette {
        entries: library.entries(),
        default_states: &catalogue.default_states,
    };
    let mut canvas = Canvas::new(chunk, (chunk_x, chunk_z), blocks, neighbour);
    for piece in bastion.pieces.iter().filter(|piece| piece.bounds.intersects(&column)) {
        let entry = &catalogue.entries[piece.entry];
        let EntryKind::Template { template, processors, foundation, .. } = &entry.kind else { continue };
        report_structure_entities(BASTION, &entities_at(template, piece.origin, piece.rotation, false), |x, z| {
            column.contains(x.floor() as i32, column.y0, z.floor() as i32)
        });
        let pivot = bastion.pivot;
        for connector in entry.jigsaws(library, piece.origin, piece.rotation) {
            let (x, y, z) = connector.pos;
            if !canvas.inside(x, z) || !canvas.chunk.contains_y(y) {
                continue;
            }
            let world = sample(blocks, canvas.get(x, y, z));
            let from_pivot = (x - pivot.0, y - pivot.1, z - pivot.2);
            let index = if processors.is_empty() {
                connector.final_state
            } else {
                process(processors, connector.final_state, &mut MersenneTwister::new(seed), &rules, &world, from_pivot)
            };
            canvas.set(x, y, z, palette.block(index));
        }
        let rotation = piece.rotation as usize;
        let mut rule_random = (!processors.is_empty()).then(|| MersenneTwister::new(seed));
        let mut cells: Vec<(Cell, u16)> = Vec::with_capacity(template.blocks.len());
        for block in &template.blocks {
            let kind = library.kind(block.state);
            if kind == StateKind::Jigsaw {
                continue;
            }
            let (dx, dy, dz) = piece.rotation.apply((block.x as i32, block.y as i32, block.z as i32));
            let at = (piece.origin.0 + dx, piece.origin.1 + dy, piece.origin.2 + dz);
            if !canvas.inside(at.0, at.2) || !canvas.chunk.contains_y(at.1) {
                continue;
            }
            if kind == StateKind::StructureBlock {
                cells.push((at, u16::MAX));
                continue;
            }
            let mut index = library.rotated(block.state, rotation);
            if let Some(rule_random) = rule_random.as_mut() {
                let world = sample(blocks, canvas.get(at.0, at.1, at.2));
                let from_pivot = (at.0 - pivot.0, at.1 - pivot.1, at.2 - pivot.2);
                index = process(processors, index, rule_random, &rules, &world, from_pivot);
            }
            cells.push((at, index));
        }
        for (at, index) in cells {
            let block = if index == u16::MAX { AIR } else { palette.block(index) };
            canvas.set(at.0, at.1, at.2, block);
            if CHESTS.contains(&canvas.short_name(block)) {
                random.next_u32();
            }
        }
        if foundation & PLUG_HOLES != 0 {
            plug_holes(&mut canvas, &piece.bounds, ids.netherrack);
        }
    }
}

fn plug_holes(canvas: &mut Canvas, bounds: &BoundingBox, netherrack: BlockId) {
    let middle_x = bounds.x0 + (bounds.x1 - bounds.x0 + 1).div_euclid(2);
    let middle_z = bounds.z0 + (bounds.z1 - bounds.z0 + 1).div_euclid(2);
    let y = bounds.y0 - 1;
    for x in middle_x - PLUG_RADIUS..=middle_x + PLUG_RADIUS {
        for z in middle_z - PLUG_RADIUS..=middle_z + PLUG_RADIUS {
            if !canvas.falls(canvas.get(x, y, z)) {
                continue;
            }
            let mut below = y - 1;
            while below > FLOOR {
                let block = canvas.get(x, below, z);
                if !canvas.falls(block) {
                    if block == AIR || canvas.blocks.entry(block).liquid {
                        canvas.set(x, below, z, netherrack);
                    }
                    break;
                }
                below -= 1;
            }
        }
    }
}
