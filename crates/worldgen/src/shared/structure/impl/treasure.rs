use crate::shared::blocks::Blocks;
use crate::shared::chunk::proto::ProtoChunk;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::StructureBlocks;
use crate::shared::structure::builder::{StructureBuilder, StructureContext, passes_filter};
use crate::shared::structure::{Probed, Reaching, StructureKind};

const LOWEST_CHEST_Y: i32 = 6;
const BURIAL_DEPTH: i32 = 4;
const BURIAL_JITTER: i32 = 2;
const CHUNK_CENTRE: i32 = 8;
const ENCLOSING_NEIGHBOURS: [(i32, i32, i32); 5] = [(0, 1, 0), (0, 0, -1), (1, 0, 0), (0, 0, 1), (-1, 0, 0)];

pub fn place(blocks: &Blocks, structure_blocks: &StructureBlocks, random: &mut MersenneTwister, chunk: &mut ProtoChunk) {
    let (x, z) = (CHUNK_CENTRE, CHUNK_CENTRE);
    let solid = |chunk: &ProtoChunk, x: i32, y: i32, z: i32| {
        if !(0..16).contains(&x) || !(0..16).contains(&z) || y < chunk.min_y {
            return false;
        }
        let entry = blocks.entry(chunk.get(x, y, z));
        entry.solid && !entry.liquid
    };
    let floor = (chunk.min_y..chunk.max_y() + 1).rev().find(|&y| blocks.entry(chunk.get(x, y, z)).solid).map_or(chunk.min_y, |y| y + 1);
    let mut y = floor - random.next_int_bounded(BURIAL_JITTER) - BURIAL_DEPTH;
    loop {
        let enclosed = ENCLOSING_NEIGHBOURS.iter().all(|&(dx, dy, dz)| solid(chunk, x + dx, y + dy, z + dz));
        let low = y < LOWEST_CHEST_Y;
        y -= 1;
        if enclosed || low {
            break;
        }
    }
    let mut chest = y + 1;
    while !(solid(chunk, x, chest - 1, z) || chest < LOWEST_CHEST_Y) {
        chest -= 1;
    }
    chunk.set(x, chest, z, structure_blocks.chest_north);
}

pub struct BuriedTreasureBuilder;

impl StructureBuilder for BuriedTreasureBuilder {
    fn is_start(&self, context: &StructureContext, _: StructureKind, chunk: (i32, i32)) -> Probed<bool> {
        Ok(passes_filter(context, StructureKind::BuriedTreasure, chunk))
    }

    fn place(&self, context: &StructureContext, blocks: &Blocks, reaching: &Reaching, chunk: &mut ProtoChunk) -> Probed<()> {
        let structures = context.structures;
        for &(_, start) in reaching.starts.iter().filter(|&&(kind, _)| kind == StructureKind::BuriedTreasure) {
            place(blocks, &structures.blocks, &mut structures.seeds.random(start.0, start.1), chunk);
        }
        Ok(())
    }
}
