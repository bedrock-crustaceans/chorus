use chorus_block::block_id::LAVA;
use chorus_block::state::common::LIQUID_DEPTH;

use super::decorate::{View, solid};

const GENERATIONS: usize = 128;
const SIDES: [[i32; 3]; 4] = [[-1, 0, 0], [1, 0, 0], [0, 0, -1], [0, 0, 1]];
const NEIGHBOURS: [[i32; 3]; 6] = [[-1, 0, 0], [1, 0, 0], [0, 0, 1], [0, 0, -1], [0, 1, 0], [0, -1, 0]];
const NO_DROP: i32 = 1000;
const FALLING: i32 = 8;
const MAX_DEPTH: i32 = 7;
const LAVA_DELAY: u64 = 10;
const LAVA_SLOPE_REACH: i32 = 2;
const STEEP_FIRST: i32 = 5;
const STEEP_LAST: i32 = 7;
const STEEP_DROP: i32 = 3;
const FLAT_START: i32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tick {
    pub pos: [i32; 3],
    pub lava: bool,
    pub time: u64,
}

#[derive(Default)]
struct Heap(Vec<Tick>);

impl Heap {
    fn after(first: &Tick, second: &Tick) -> bool {
        first.time > second.time
    }

    fn push(&mut self, tick: Tick) {
        self.0.push(tick);
        let mut hole = self.0.len() - 1;
        while hole > 0 {
            let parent = (hole - 1) / 2;
            if !Self::after(&self.0[parent], &tick) {
                break;
            }
            self.0[hole] = self.0[parent];
            hole = parent;
        }
        self.0[hole] = tick;
    }

    fn pop(&mut self) -> Option<Tick> {
        let first = *self.0.first()?;
        let last = self.0.pop()?;
        let count = self.0.len();
        if count == 0 {
            return Some(first);
        }
        let last_parent = (count - 1) / 2;
        let mut hole = 0;
        while hole < last_parent {
            let mut child = 2 * hole + 2;
            if Self::after(&self.0[child], &self.0[child - 1]) {
                child -= 1;
            }
            self.0[hole] = self.0[child];
            hole = child;
        }
        if hole == last_parent && count.is_multiple_of(2) {
            self.0[hole] = self.0[count - 1];
            hole = count - 1;
        }
        while hole > 0 {
            let parent = (hole - 1) / 2;
            if !Self::after(&self.0[parent], &last) {
                break;
            }
            self.0[hole] = self.0[parent];
            hole = parent;
        }
        self.0[hole] = last;
        Some(first)
    }

    fn peek(&self) -> Option<&Tick> {
        self.0.first()
    }
}

pub struct TickQueue {
    now: u64,
    pending: Heap,
    scheduled: Vec<(Tick, u32)>,
}

impl TickQueue {
    pub fn chunk() -> Self {
        Self {
            now: 0,
            pending: Heap::default(),
            scheduled: Vec::new(),
        }
    }

    pub fn add(&mut self, pos: [i32; 3], lava: bool, delay: u64) {
        let tick = Tick {
            pos,
            lava,
            time: self.now.wrapping_add(delay),
        };
        match self.scheduled.iter_mut().find(|(known, _)| *known == tick) {
            Some((_, count)) => *count += 1,
            None => self.scheduled.push((tick, 1)),
        }
        self.pending.push(tick);
    }

    pub fn is_due(&self, pos: [i32; 3], lava: bool, delay: u64) -> bool {
        let time = self.now.wrapping_add(delay);
        self.scheduled.iter().any(|(tick, _)| tick.pos == pos && tick.lava == lava && tick.time == time)
    }

    fn take_next(&mut self) -> Option<Tick> {
        let tick = self.pending.pop()?;
        if let Some(index) = self.scheduled.iter().position(|(known, _)| *known == tick) {
            self.scheduled[index].1 -= 1;
            if self.scheduled[index].1 == 0 {
                self.scheduled.swap_remove(index);
            }
        }
        Some(tick)
    }

    pub fn is_empty(&self) -> bool {
        self.pending.peek().is_none()
    }

    pub fn take_generation(&mut self) -> Vec<Tick> {
        self.now = u64::MAX;
        let mut active = Heap::default();
        while self.pending.peek().is_some_and(|tick| tick.time <= self.now) {
            let tick = self.take_next().expect("a pending tick");
            active.push(tick);
        }
        std::iter::from_fn(|| active.pop()).collect()
    }
}

fn liquid_depth(view: &View, [x, y, z]: [i32; 3]) -> Option<i32> {
    let block = view.get(x, y, z);
    if view.blocks().name_of(block) != LAVA {
        return None;
    }
    Some(view.blocks().int(block, &LIQUID_DEPTH).unwrap_or(0))
}

fn stops(view: &View, [x, y, z]: [i32; 3]) -> bool {
    let block = view.get(x, y, z);
    let name = view.blocks().name_of(block);
    solid(view.blocks(), block) && name != chorus_block::block_id::WEEPING_VINES && name != chorus_block::block_id::TWISTING_VINES
}

fn add([x, y, z]: [i32; 3], [dx, dy, dz]: [i32; 3]) -> [i32; 3] {
    [x + dx, y + dy, z + dz]
}

pub fn instatick(view: &mut View) {
    run_generations(view, GENERATIONS);
}

fn run_generations(view: &mut View, generations: usize) {
    for _ in 0..generations {
        if view.ticks.is_empty() {
            break;
        }
        for tick in view.ticks.take_generation() {
            run(view, tick);
        }
    }
}

fn run(view: &mut View, tick: Tick) {
    if !view.can_tick(tick.pos[0], tick.pos[2]) {
        if view.has_block(tick.pos[0], tick.pos[1], tick.pos[2]) {
            view.ticks.add(tick.pos, tick.lava, 0);
        }
        return;
    }
    if liquid_depth(view, tick.pos).is_some() {
        flow(view, tick.pos);
    }
}

fn flow(view: &mut View, pos: [i32; 3]) {
    let Some(mut depth) = liquid_depth(view, pos) else { return };
    if depth >= 1 {
        depth = settle(view, pos, depth);
    }
    let below = add(pos, [0, -1, 0]);
    if view.has_block(below[0], below[1], below[2]) {
        let occupied = liquid_depth(view, below).is_some();
        if !occupied && !stops(view, below) {
            spread_to(view, below, if depth >= FALLING { depth } else { depth + FALLING });
            return;
        }
    }
    if depth < 0 || depth != 0 && !stops(view, below) {
        return;
    }
    sideways(view, pos, depth);
}

fn settle(view: &mut View, pos: [i32; 3], depth: i32) -> i32 {
    let mut nearest = None::<i32>;
    for side in SIDES {
        let Some(side_depth) = liquid_depth(view, add(pos, side)) else { continue };
        nearest = Some(if side_depth >= FALLING { 0 } else { nearest.map_or(side_depth, |lowest| lowest.min(side_depth)) });
    }
    let mut target = nearest.map_or(-1, |lowest| if lowest + 1 > MAX_DEPTH { -1 } else { lowest + 1 });
    let above = add(pos, [0, 1, 0]);
    if let Some(above_depth) = liquid_depth(view, above) {
        target = if above_depth >= FALLING { above_depth } else { above_depth + FALLING };
    }
    if target == depth {
        return depth;
    }
    if target < 0 {
        view.set(pos[0], pos[1], pos[2], super::AIR);
    } else {
        let block = view.lava_level(target as u8);
        view.set(pos[0], pos[1], pos[2], block);
        wake(view, pos);
        view.schedule(pos, true, LAVA_DELAY);
    }
    target
}

fn wake(view: &mut View, pos: [i32; 3]) {
    for offset in NEIGHBOURS {
        let neighbour = add(pos, offset);
        if liquid_depth(view, neighbour).is_some() && !view.ticks.is_due(neighbour, true, LAVA_DELAY) {
            view.schedule(neighbour, true, LAVA_DELAY);
        }
    }
}

fn sideways(view: &mut View, pos: [i32; 3], depth: i32) {
    let mut costs = [NO_DROP; 4];
    for (index, side) in SIDES.into_iter().enumerate() {
        let next = add(pos, side);
        if stops(view, next) || liquid_depth(view, next) == Some(0) {
            continue;
        }
        costs[index] = if !stops(view, add(next, [0, -1, 0])) {
            0
        } else {
            let own = liquid_depth(view, pos).unwrap_or(-1);
            let start = if (STEEP_FIRST..=STEEP_LAST).contains(&own) { own - STEEP_DROP } else { FLAT_START };
            slope(view, next, start, index)
        };
    }
    let next_depth = if depth >= FALLING { 1 } else { depth + 1 };
    if next_depth > MAX_DEPTH {
        return;
    }
    let cheapest = costs.into_iter().min().unwrap_or(NO_DROP);
    for (index, side) in SIDES.into_iter().enumerate() {
        if costs[index] == cheapest {
            spread_to(view, add(pos, side), next_depth);
        }
    }
}

fn slope(view: &View, from: [i32; 3], pass: i32, entered_by: usize) -> i32 {
    let mut best = NO_DROP;
    for (index, side) in SIDES.into_iter().enumerate() {
        if index == entered_by ^ 1 {
            continue;
        }
        let next = add(from, side);
        if stops(view, next) || liquid_depth(view, next) == Some(0) {
            continue;
        }
        if !stops(view, add(next, [0, -1, 0])) {
            return pass;
        }
        if pass < LAVA_SLOPE_REACH {
            best = best.min(slope(view, next, pass + 1, index));
        }
    }
    best
}

fn spread_to(view: &mut View, pos: [i32; 3], depth: i32) {
    if !view.has_block(pos[0], pos[1], pos[2]) {
        return;
    }
    if liquid_depth(view, pos).is_some() || stops(view, pos) {
        return;
    }
    let block = view.lava_level(depth as u8);
    view.set(pos[0], pos[1], pos[2], block);
    wake(view, pos);
    view.schedule(pos, true, LAVA_DELAY);
}
