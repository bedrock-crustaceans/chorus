use super::super::NetherBlocks;
use super::ores::{LegacyOres, position, scatter_ore, spread, zyx_offset};
use super::view::{View, opaque_solid, opaque_to_light};
use super::{CHUNK_WIDTH, FACES};
use crate::shared::blocks::{AIR, BlockId};
use crate::shared::rng::mersenne::MersenneTwister;

const SPRINGS: i32 = 8;

const SPRING_HEIGHT_LOW: i32 = 4;

const SPRING_HEIGHT_SPAN: i32 = 120;

const FIRE_SPAN: i32 = 10;

const FIRE_BASE: i32 = 1;

const MUSHROOM_DRAW: i32 = 1;

const GLOWSTONE_SPAN: i32 = 10;

const GLOWSTONE_PLACED: i32 = 10;

const WHOLE_HEIGHT: i32 = 128;

const MUSHROOM_TRIES: i32 = 64;

const GLOWSTONE_TRIES: i32 = 1500;

const GLOWSTONE_REACH: u32 = 7;

const GLOWSTONE_DEPTH: i32 = 12;

const FIRE_TRIES: i32 = 64;

const ORE_HEIGHT_LOW: i32 = 10;

const ORE_HEIGHT_SPAN: i32 = 108;

const GOLD_ORE_COUNT: i32 = 10;

const QUARTZ_ORE_COUNT: i32 = 16;

const ROCK_SPRING_COUNT: i32 = 16;

const MAGMA_COUNT: i32 = 9;

const MAGMA_LOW: i32 = 23;

const MAGMA_SPAN: i32 = 14;

const GRAVEL_COUNT: i32 = 2;

const GRAVEL_LOW: i32 = 5;

const GRAVEL_SPAN: i32 = 32;

const MUSHROOM_SHADE: [[i32; 2]; 13] = [[0, 0], [0, -1], [1, 0], [0, 1], [-1, 0], [1, -1], [-1, -1], [1, 1], [-1, 1], [0, -2], [2, 0], [0, 2], [-2, 0]];

const SHADE_DEPTH: i32 = 10;

fn spring(view: &mut View, [x, y, z]: [i32; 3], inside_rock: bool, ids: &NetherBlocks) {
    let there = view.get(x, y, z);
    if view.get(x, y + 1, z) == ids.netherrack && (there == ids.air || there == ids.netherrack) {
        let sides = [[x - 1, y, z], [x + 1, y, z], [x, y, z - 1], [x, y, z + 1], [x, y - 1, z]];
        let count = |block: BlockId| sides.iter().filter(|&&[x, y, z]| view.get(x, y, z) == block).count();
        let (rock, air) = (count(ids.netherrack), count(ids.air));
        if rock == 4 && !inside_rock && air == 1 || rock == 5 {
            view.set(x, y, z, ids.lava);
        }
    }
    view.schedule([x, y, z], true, 0);
}

fn fire(view: &mut View, random: &mut MersenneTwister, at: [i32; 3], ids: &NetherBlocks) {
    for _ in 0..FIRE_TRIES {
        let [x, y, z] = zyx_offset(random, at);
        if view.get(x, y, z) != ids.air {
            continue;
        }
        let below = view.get(x, y - 1, z);
        if below == ids.netherrack {
            view.set(x, y, z, ids.fire);
        } else if below == ids.soul_soil {
            view.set(x, y, z, ids.soul_fire);
        }
    }
}

fn glowstone(view: &mut View, random: &mut MersenneTwister, [x, y, z]: [i32; 3], ids: &NetherBlocks) {
    let above = view.get(x, y + 1, z);
    let ceiling = above == ids.netherrack || view.blocks().same_kind(above, ids.basalt) || above == ids.blackstone;
    if view.get(x, y, z) != ids.air || !ceiling {
        return;
    }
    view.set(x, y, z, ids.glowstone);
    for _ in 0..GLOWSTONE_TRIES {
        let dz = spread(random, GLOWSTONE_REACH);
        let dy = random.next_int_bounded(GLOWSTONE_DEPTH);
        let dx = spread(random, GLOWSTONE_REACH);
        let (tx, ty, tz) = (x + dx, y - dy, z + dz);
        if view.get(tx, ty, tz) != ids.air {
            continue;
        }
        let touching = FACES.iter().filter(|[dx, dy, dz]| view.get(tx + dx, ty + dy, tz + dz) == ids.glowstone).count();
        if touching == 1 {
            view.set(tx, ty, tz, ids.glowstone);
        }
    }
}

fn shaded(view: &View, [x, y, z]: [i32; 3]) -> bool {
    view.heightmap(x, z) > y || (y + 1..=y + SHADE_DEPTH).any(|above| opaque_to_light(view.blocks(), view.get(x, above, z)))
}

fn mushroom(view: &mut View, random: &mut MersenneTwister, at: [i32; 3], block: BlockId) {
    let mut p = at;
    for _ in 0..MUSHROOM_TRIES {
        let [x, y, z] = p;
        let below = view.get(x, y - 1, z);
        if view.get(x, y, z) == AIR && below != AIR {
            let wood = view.blocks().same_kind(below, view.ids().crimson_stem) || view.blocks().same_kind(below, view.ids().warped_stem);
            let soil = wood || below == view.ids().mycelium || below == view.ids().podzol;
            let ground = opaque_solid(view.blocks(), below) && view.blocks().entry(below).cube;
            if soil || MUSHROOM_SHADE.iter().all(|&[dx, dz]| shaded(view, [x + dx, y, z + dz])) && ground {
                view.set(x, y, z, block);
            }
        }
        p = zyx_offset(random, at);
    }
}

pub fn hardcoded(random: &mut MersenneTwister, source: (i32, i32), view: &mut View, ids: &NetherBlocks, ores: &LegacyOres) {
    let centre = (source.0 * CHUNK_WIDTH + CHUNK_WIDTH / 2, source.1 * CHUNK_WIDTH + CHUNK_WIDTH / 2);
    let corner = (source.0 * CHUNK_WIDTH, source.1 * CHUNK_WIDTH);
    for _ in 0..SPRINGS {
        let at = position(random, centre, (SPRING_HEIGHT_LOW, SPRING_HEIGHT_SPAN));
        spring(view, at, false, ids);
    }
    let fires = {
        let bound = random.next_int_bounded(FIRE_SPAN) + FIRE_BASE;
        random.next_int_bounded(bound) + FIRE_BASE
    };
    for _ in 0..fires {
        let at = position(random, centre, (SPRING_HEIGHT_LOW, SPRING_HEIGHT_SPAN));
        fire(view, random, at, ids);
    }
    let glows = {
        let bound = random.next_int_bounded(GLOWSTONE_SPAN) + FIRE_BASE;
        random.next_int_bounded(bound)
    };
    for _ in 0..glows {
        let at = position(random, centre, (SPRING_HEIGHT_LOW, SPRING_HEIGHT_SPAN));
        glowstone(view, random, at, ids);
    }
    for _ in 0..GLOWSTONE_PLACED {
        let at = position(random, centre, (0, WHOLE_HEIGHT));
        glowstone(view, random, at, ids);
    }
    for block in [ids.brown_mushroom, ids.red_mushroom] {
        if random.next_int_bounded(MUSHROOM_DRAW) == 0 {
            let at = position(random, centre, (0, WHOLE_HEIGHT));
            mushroom(view, random, at, block);
        }
    }
    for (ore, count) in [(&ores.gold, GOLD_ORE_COUNT), (&ores.quartz, QUARTZ_ORE_COUNT)] {
        scatter_ore(view, random, corner, ore, count, (ORE_HEIGHT_LOW, ORE_HEIGHT_SPAN));
    }
    for _ in 0..ROCK_SPRING_COUNT {
        let at = position(random, corner, (ORE_HEIGHT_LOW, ORE_HEIGHT_SPAN));
        spring(view, at, true, ids);
    }
    for (ore, count, range) in [
        (&ores.magma, MAGMA_COUNT, (MAGMA_LOW, MAGMA_SPAN)),
        (&ores.gravel, GRAVEL_COUNT, (GRAVEL_LOW, GRAVEL_SPAN)),
        (&ores.blackstone, GRAVEL_COUNT, (GRAVEL_LOW, GRAVEL_SPAN)),
    ] {
        scatter_ore(view, random, corner, ore, count, range);
    }
}
