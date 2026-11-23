use glam::IVec3;

use super::ticks::{FluidTicks, ScheduledUpdate};
use super::{FluidRules, Grid, Liquid};

const SIDES: [IVec3; 4] = [IVec3::NEG_X, IVec3::X, IVec3::NEG_Z, IVec3::Z];
const NEIGHBOURS: [IVec3; 6] = [IVec3::NEG_X, IVec3::X, IVec3::Z, IVec3::NEG_Z, IVec3::Y, IVec3::NEG_Y];
const HARDENING_SOURCES: [IVec3; 5] = [IVec3::Z, IVec3::NEG_Z, IVec3::X, IVec3::NEG_X, IVec3::Y];
const NO_DROP: i32 = 1000;
const FALLING: i32 = 8;
const MAX_SPREAD_DEPTH: i32 = 7;
const WATER_SLOPE_REACH: i32 = 4;
const LAVA_SLOPE_REACH: i32 = 2;
const STEEP_FLOW_FIRST: i32 = 5;
const STEEP_FLOW_LAST: i32 = 7;
const STEEP_FLOW_DROP: i32 = 3;
const FLAT_SLOPE_START: i32 = 1;

fn delay_of(rules: &FluidRules, lava: bool) -> u64 {
    rules.motion(lava).delay
}

fn decay_of(rules: &FluidRules, lava: bool) -> i32 {
    rules.motion(lava).decay
}

fn read<G: Grid>(grid: &G, rules: &FluidRules, pos: IVec3) -> i32 {
    grid.read(pos).unwrap_or_else(|| rules.blocks().air)
}

fn liquid<G: Grid>(grid: &G, rules: &FluidRules, pos: IVec3) -> Option<Liquid> {
    rules.liquid(read(grid, rules, pos))
}

fn liquid_depth<G: Grid>(grid: &G, rules: &FluidRules, pos: IVec3, lava: bool) -> Option<i32> {
    liquid(grid, rules, pos).filter(|liquid| liquid.lava == lava).map(|liquid| liquid.depth as i32)
}

fn stops<G: Grid>(grid: &G, rules: &FluidRules, pos: IVec3, lava: bool) -> bool {
    let cell = rules.cell(read(grid, rules, pos));
    if lava { cell.stops_lava } else { cell.stops_water }
}

fn put<G: Grid>(grid: &mut G, rules: &FluidRules, pos: IVec3, block: i32) {
    if rules.cell(read(grid, rules, pos)).waterlogged {
        return;
    }
    grid.write(pos, block);
}

pub fn step<G: Grid>(grid: &mut G, rules: &FluidRules, ticks: &mut FluidTicks, update: ScheduledUpdate) {
    let here = read(grid, rules, update.pos);
    if rules.cell(here).magma {
        start_bubble_column(grid, rules, update.pos);
    } else if liquid_depth(grid, rules, update.pos, update.lava).is_some() {
        flow(grid, rules, ticks, update.pos, update.lava);
    }
}

fn start_bubble_column<G: Grid>(grid: &mut G, rules: &FluidRules, magma: IVec3) {
    let mut above = magma + IVec3::Y;
    while grid.read(above).is_some() && rules.cell(read(grid, rules, above)).water_source {
        grid.write(above, rules.blocks().bubble_column);
        above += IVec3::Y;
    }
}

fn wake_neighbours<G: Grid>(grid: &mut G, rules: &FluidRules, ticks: &mut FluidTicks, pos: IVec3) {
    for offset in NEIGHBOURS {
        let neighbour = pos + offset;
        let Some(found) = liquid(grid, rules, neighbour) else { continue };
        if found.lava {
            harden(grid, rules, neighbour);
        }
        let delay = delay_of(rules, found.lava);
        if liquid_depth(grid, rules, neighbour, found.lava).is_some() && !ticks.is_scheduled(neighbour, found.lava, delay) {
            ticks.add(neighbour, found.lava, delay);
        }
    }
}

fn harden<G: Grid>(grid: &mut G, rules: &FluidRules, pos: IVec3) {
    let Some(lava) = liquid(grid, rules, pos).filter(|liquid| liquid.lava) else { return };
    if HARDENING_SOURCES.iter().any(|&offset| liquid_depth(grid, rules, pos + offset, false).is_some()) {
        let hardness = if lava.depth == 0 { rules.blocks().obsidian } else { rules.blocks().cobblestone };
        put(grid, rules, pos, hardness);
    }
}

fn flow<G: Grid>(grid: &mut G, rules: &FluidRules, ticks: &mut FluidTicks, pos: IVec3, lava: bool) {
    let Some(mut depth) = liquid_depth(grid, rules, pos, lava) else { return };
    let drop = decay_of(rules, lava);
    if depth >= 1 {
        depth = settle_depth(grid, rules, ticks, pos, lava, depth, drop);
    }
    let below = pos + IVec3::NEG_Y;
    if grid.read(below).is_some() {
        let held = liquid(grid, rules, below);
        let occupied = held.is_some_and(|liquid| liquid.lava == lava || liquid.lava);
        if !occupied && !stops(grid, rules, below, lava) {
            if lava && held.is_some() {
                put(grid, rules, below, rules.blocks().stone);
            } else {
                spread_to(grid, rules, ticks, below, if depth >= FALLING { depth } else { depth + FALLING }, lava);
            }
            return;
        }
    }
    if depth < 0 || depth != 0 && !stops(grid, rules, below, lava) {
        return;
    }
    spread_sideways(grid, rules, ticks, pos, lava, depth, drop);
}

fn settle_depth<G: Grid>(grid: &mut G, rules: &FluidRules, ticks: &mut FluidTicks, pos: IVec3, lava: bool, depth: i32, drop: i32) -> i32 {
    let (mut nearest, mut sources) = (None::<i32>, 0);
    for side in SIDES {
        let Some(side_depth) = liquid_depth(grid, rules, pos + side, lava) else { continue };
        sources += (side_depth == 0) as i32;
        nearest = Some(if side_depth >= FALLING { 0 } else { nearest.map_or(side_depth, |lowest| lowest.min(side_depth)) });
    }
    let mut target = nearest.map_or(-1, |lowest| if lowest + drop > MAX_SPREAD_DEPTH { -1 } else { lowest + drop });
    let above = liquid_depth(grid, rules, pos + IVec3::Y, lava);
    if let Some(above) = above {
        target = if above >= FALLING { above } else { above + FALLING };
    }
    let below = pos + IVec3::NEG_Y;
    let below_cell = rules.cell(read(grid, rules, below));
    let forms_source = sources >= 2 || sources == 1 && above == Some(0);
    if !lava && forms_source && (below_cell.solid || liquid_depth(grid, rules, below, lava) == Some(0)) {
        target = 0;
    }
    if target == depth {
        return depth;
    }
    if target < 0 {
        put(grid, rules, pos, rules.blocks().air);
    } else {
        put(grid, rules, pos, rules.liquid_block(lava, target as u8));
        wake_neighbours(grid, rules, ticks, pos);
        ticks.add(pos, lava, delay_of(rules, lava));
    }
    target
}

fn spread_sideways<G: Grid>(grid: &mut G, rules: &FluidRules, ticks: &mut FluidTicks, pos: IVec3, lava: bool, depth: i32, drop: i32) {
    let mut costs = [NO_DROP; 4];
    for (index, side) in SIDES.into_iter().enumerate() {
        let next = pos + side;
        if stops(grid, rules, next, lava) || liquid_depth(grid, rules, next, lava) == Some(0) {
            continue;
        }
        costs[index] = if !stops(grid, rules, next + IVec3::NEG_Y, lava) {
            0
        } else {
            let own = liquid_depth(grid, rules, pos, lava).unwrap_or(-1);
            let start = if (STEEP_FLOW_FIRST..=STEEP_FLOW_LAST).contains(&own) {
                own - STEEP_FLOW_DROP
            } else {
                FLAT_SLOPE_START
            };
            drop_distance(grid, rules, next, start, index, lava)
        };
    }
    let next_depth = if depth >= FALLING { 1 } else { depth + drop };
    if next_depth > MAX_SPREAD_DEPTH {
        return;
    }
    let cheapest = costs.into_iter().min().unwrap_or(NO_DROP);
    for (index, side) in SIDES.into_iter().enumerate() {
        if costs[index] == cheapest {
            spread_to(grid, rules, ticks, pos + side, next_depth, lava);
        }
    }
}

fn drop_distance<G: Grid>(grid: &G, rules: &FluidRules, from: IVec3, pass: i32, entered_by: usize, lava: bool) -> i32 {
    let reach = if lava { LAVA_SLOPE_REACH } else { WATER_SLOPE_REACH };
    let mut best = NO_DROP;
    for (index, side) in SIDES.into_iter().enumerate() {
        if index == entered_by ^ 1 {
            continue;
        }
        let next = from + side;
        if stops(grid, rules, next, lava) || liquid_depth(grid, rules, next, lava) == Some(0) {
            continue;
        }
        if !stops(grid, rules, next + IVec3::NEG_Y, lava) {
            return pass;
        }
        if pass < reach {
            best = best.min(drop_distance(grid, rules, next, pass + 1, index, lava));
        }
    }
    best
}

fn spread_to<G: Grid>(grid: &mut G, rules: &FluidRules, ticks: &mut FluidTicks, pos: IVec3, depth: i32, lava: bool) {
    if grid.read(pos).is_none() {
        return;
    }
    let occupied = liquid(grid, rules, pos).is_some_and(|liquid| liquid.lava == lava || liquid.lava);
    if occupied || stops(grid, rules, pos, lava) {
        return;
    }
    put(grid, rules, pos, rules.liquid_block(lava, depth as u8));
    wake_neighbours(grid, rules, ticks, pos);
    ticks.add(pos, lava, delay_of(rules, lava));
}
