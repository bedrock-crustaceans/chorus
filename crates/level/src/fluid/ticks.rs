use glam::IVec3;

use super::flow;
use super::{FluidRules, Grid};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScheduledUpdate {
    pub pos: IVec3,
    pub lava: bool,
    pub due: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Order {
    due: u64,
    x: i32,
    y: i32,
    z: i32,
    lava: bool,
    sequence: u64,
}

impl Order {
    fn of(update: ScheduledUpdate, sequence: u64) -> Self {
        Self {
            due: update.due,
            x: update.pos.x,
            y: update.pos.y,
            z: update.pos.z,
            lava: update.lava,
            sequence,
        }
    }

    fn update(self) -> ScheduledUpdate {
        ScheduledUpdate {
            pos: IVec3::new(self.x, self.y, self.z),
            lava: self.lava,
            due: self.due,
        }
    }
}

#[derive(Default)]
struct GenerationHeap(Vec<ScheduledUpdate>);

impl GenerationHeap {
    fn later(first: &ScheduledUpdate, second: &ScheduledUpdate) -> bool {
        first.due > second.due
    }

    fn push(&mut self, update: ScheduledUpdate) {
        self.0.push(update);
        let mut hole = self.0.len() - 1;
        while hole > 0 {
            let parent = (hole - 1) / 2;
            if !Self::later(&self.0[parent], &update) {
                break;
            }
            self.0[hole] = self.0[parent];
            hole = parent;
        }
        self.0[hole] = update;
    }

    fn pop(&mut self) -> Option<ScheduledUpdate> {
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
            if Self::later(&self.0[child], &self.0[child - 1]) {
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
            if !Self::later(&self.0[parent], &last) {
                break;
            }
            self.0[hole] = self.0[parent];
            hole = parent;
        }
        self.0[hole] = last;
        Some(first)
    }

    fn peek(&self) -> Option<&ScheduledUpdate> {
        self.0.first()
    }
}

enum Pending {
    Game(std::collections::BinaryHeap<std::cmp::Reverse<Order>>),
    Generation(GenerationHeap),
}

type Key = (IVec3, bool, u64);

pub struct FluidTicks {
    now: u64,
    sequence: u64,
    pending: Pending,
    scheduled: rustc_hash::FxHashMap<Key, u32>,
    due: Vec<ScheduledUpdate>,
    deferred: Vec<ScheduledUpdate>,
}

impl Default for FluidTicks {
    fn default() -> Self {
        Self::game()
    }
}

impl FluidTicks {
    pub fn game() -> Self {
        Self {
            now: 0,
            sequence: 0,
            pending: Pending::Game(std::collections::BinaryHeap::new()),
            scheduled: rustc_hash::FxHashMap::default(),
            due: Vec::new(),
            deferred: Vec::new(),
        }
    }

    pub fn generation_clock(clock: u64) -> Self {
        Self {
            now: clock,
            sequence: 0,
            pending: Pending::Generation(GenerationHeap::default()),
            scheduled: rustc_hash::FxHashMap::default(),
            due: Vec::new(),
            deferred: Vec::new(),
        }
    }

    pub fn now(&self) -> u64 {
        self.now
    }

    pub fn is_empty(&self) -> bool {
        match &self.pending {
            Pending::Game(heap) => heap.is_empty(),
            Pending::Generation(heap) => heap.0.is_empty(),
        }
    }

    pub fn len(&self) -> usize {
        match &self.pending {
            Pending::Game(heap) => heap.len(),
            Pending::Generation(heap) => heap.0.len(),
        }
    }

    pub fn add(&mut self, pos: IVec3, lava: bool, delay: u64) {
        self.schedule_at(pos, lava, self.now.wrapping_add(delay));
    }

    pub fn schedule_at(&mut self, pos: IVec3, lava: bool, due: u64) {
        self.push(ScheduledUpdate { pos, lava, due });
    }

    pub fn is_scheduled(&self, pos: IVec3, lava: bool, delay: u64) -> bool {
        self.scheduled.contains_key(&(pos, lava, self.now.wrapping_add(delay)))
    }

    fn push(&mut self, update: ScheduledUpdate) {
        *self.scheduled.entry((update.pos, update.lava, update.due)).or_default() += 1;
        match &mut self.pending {
            Pending::Game(heap) => {
                self.sequence += 1;
                heap.push(std::cmp::Reverse(Order::of(update, self.sequence)));
            }
            Pending::Generation(heap) => heap.push(update),
        }
    }

    fn release(&mut self, update: ScheduledUpdate) {
        let key = (update.pos, update.lava, update.due);
        if let Some(count) = self.scheduled.get_mut(&key) {
            *count -= 1;
            if *count == 0 {
                self.scheduled.remove(&key);
            }
        }
    }

    fn take_next(&mut self) -> Option<ScheduledUpdate> {
        let update = match &mut self.pending {
            Pending::Game(heap) => heap.pop().map(|entry| entry.0.update()),
            Pending::Generation(heap) => heap.pop(),
        }?;
        self.release(update);
        Some(update)
    }

    pub fn absorb(&mut self, mut other: FluidTicks) {
        while let Some(update) = other.take_next() {
            self.push(update);
        }
    }

    pub fn advance<G: Grid>(&mut self, grid: &mut G, rules: &FluidRules) {
        self.now += 1;
        self.due.clear();
        self.deferred.clear();
        while let Some(update) = self.next_due(self.now) {
            if !grid.can_tick(update.pos) {
                self.deferred.push(update);
            } else {
                self.due.push(update);
            }
        }
        self.due.sort_unstable_by_key(|update| (update.pos.x, update.pos.y, update.pos.z, update.lava));
        for index in 0..self.due.len() {
            let update = self.due[index];
            flow::step(grid, rules, self, update);
        }
        for index in 0..self.deferred.len() {
            let update = self.deferred[index];
            self.push(update);
        }
    }

    fn next_due(&mut self, now: u64) -> Option<ScheduledUpdate> {
        let due = match &self.pending {
            Pending::Game(heap) => heap.peek().map(|entry| entry.0.due)?,
            Pending::Generation(heap) => heap.peek()?.due,
        };
        if due <= now { self.take_next() } else { None }
    }

    pub fn advance_generation<G: Grid>(&mut self, grid: &mut G, rules: &FluidRules) {
        self.now = u64::MAX;
        let mut active = GenerationHeap::default();
        while let Some(update) = self.take_next() {
            active.push(update);
        }
        while let Some(update) = active.pop() {
            if grid.can_tick(update.pos) {
                flow::step(grid, rules, self, update);
            } else {
                self.add(update.pos, update.lava, 0);
            }
        }
    }

    pub fn take_pending(&mut self) -> Vec<ScheduledUpdate> {
        self.scheduled.clear();
        let mut pending = Vec::with_capacity(self.len());
        match &mut self.pending {
            Pending::Game(heap) => pending.extend(heap.drain().map(|entry| entry.0.update())),
            Pending::Generation(heap) => pending.append(&mut heap.0),
        }
        pending.sort_unstable_by_key(|update| (update.due, update.pos.x, update.pos.y, update.pos.z, update.lava));
        pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fluid::{CellRules, FluidBlocks, Liquid, LiquidMotion, WATER_DELAY};
    use std::collections::{HashMap, HashSet};

    const AIR: i32 = 0;
    const STONE: i32 = 1;
    const WATER: i32 = 2;
    const WATER_FALLING: i32 = 3;
    const LAVA: i32 = 7;
    const LAVA_FALLING: i32 = 8;

    fn rules() -> FluidRules {
        rules_with(LiquidMotion::WATER, LiquidMotion::LAVA)
    }

    fn rules_with(water_motion: LiquidMotion, lava_motion: LiquidMotion) -> FluidRules {
        let mut cells = rustc_hash::FxHashMap::default();
        cells.insert(
            STONE,
            CellRules {
                stops_water: true,
                stops_lava: true,
                solid: true,
                ..CellRules::default()
            },
        );
        cells.insert(
            WATER,
            CellRules {
                liquid: Some(Liquid { lava: false, depth: 0 }),
                water_source: true,
                ..CellRules::default()
            },
        );
        cells.insert(
            WATER_FALLING,
            CellRules {
                liquid: Some(Liquid { lava: false, depth: 8 }),
                ..CellRules::default()
            },
        );
        cells.insert(
            LAVA,
            CellRules {
                liquid: Some(Liquid { lava: true, depth: 0 }),
                ..CellRules::default()
            },
        );
        cells.insert(
            LAVA_FALLING,
            CellRules {
                liquid: Some(Liquid { lava: true, depth: 8 }),
                ..CellRules::default()
            },
        );
        let mut water = [WATER; 16];
        water[8] = WATER_FALLING;
        let mut lava = [LAVA; 16];
        lava[8] = LAVA_FALLING;
        FluidRules::new(
            FluidBlocks {
                air: AIR,
                stone: STONE,
                obsidian: 4,
                cobblestone: 5,
                bubble_column: 6,
                water,
                lava,
                water_motion,
                lava_motion,
            },
            cells,
        )
    }

    #[derive(Default)]
    struct TestGrid {
        blocks: HashMap<IVec3, i32>,
        loaded: HashSet<(i32, i32)>,
        writes: Vec<IVec3>,
    }

    impl TestGrid {
        fn around() -> Self {
            let mut grid = Self::default();
            for dx in -1..=1 {
                for dz in -1..=1 {
                    grid.loaded.insert((dx, dz));
                }
            }
            grid
        }

        fn across() -> Self {
            let mut grid = Self::default();
            for dx in -1..=2 {
                for dz in -1..=1 {
                    grid.loaded.insert((dx, dz));
                }
            }
            grid
        }
    }

    impl Grid for TestGrid {
        fn read(&self, pos: IVec3) -> Option<i32> {
            if !self.loaded.contains(&(pos.x >> 4, pos.z >> 4)) || !(-64..320).contains(&pos.y) {
                return None;
            }
            Some(self.blocks.get(&pos).copied().unwrap_or(AIR))
        }

        fn write(&mut self, pos: IVec3, block: i32) -> bool {
            if self.read(pos).is_none() {
                return false;
            }
            self.blocks.insert(pos, block);
            self.writes.push(pos);
            true
        }

        fn can_tick(&self, pos: IVec3) -> bool {
            if self.read(pos).is_none() {
                return false;
            }
            let (cx, cz) = (pos.x >> 4, pos.z >> 4);
            (-1..=1).all(|dx| (-1..=1).all(|dz| self.loaded.contains(&(cx + dx, cz + dz))))
        }
    }

    fn source_in_the_air() -> (TestGrid, FluidTicks) {
        let mut grid = TestGrid::around();
        grid.blocks.insert(IVec3::new(5, 0, 5), WATER);
        let mut ticks = FluidTicks::game();
        ticks.add(IVec3::new(5, 0, 5), false, WATER_DELAY);
        (grid, ticks)
    }

    #[test]
    fn an_update_fires_only_once_its_delay_has_passed() {
        let rules = rules();
        let (mut grid, mut ticks) = source_in_the_air();
        for _ in 0..WATER_DELAY - 1 {
            ticks.advance(&mut grid, &rules);
            assert!(grid.writes.is_empty());
        }
        ticks.advance(&mut grid, &rules);
        assert_eq!(grid.writes, vec![IVec3::new(5, -1, 5)]);
        assert_eq!(grid.blocks[&IVec3::new(5, -1, 5)], WATER_FALLING);
    }

    #[test]
    fn updates_of_an_unloaded_chunk_wait_until_it_loads() {
        let rules = rules();
        let (mut grid, mut ticks) = source_in_the_air();
        grid.loaded.remove(&(0, 0));
        for _ in 0..WATER_DELAY + 2 {
            ticks.advance(&mut grid, &rules);
        }
        assert_eq!(ticks.len(), 1);
        assert!(grid.writes.is_empty());
        grid.loaded.insert((0, 0));
        ticks.advance(&mut grid, &rules);
        assert_eq!(grid.writes, vec![IVec3::new(5, -1, 5)]);
    }

    #[test]
    fn a_grid_of_unloaded_neighbours_defers_the_update_instead_of_dropping_it() {
        let rules = rules();
        let (mut grid, mut ticks) = source_in_the_air();
        grid.loaded.remove(&(1, 0));
        for _ in 0..WATER_DELAY + 2 {
            ticks.advance(&mut grid, &rules);
        }
        assert_eq!(ticks.len(), 1);
        assert!(grid.writes.is_empty());
    }

    #[test]
    fn flowing_into_a_loaded_neighbour_writes_its_chunk() {
        let rules = rules();
        let mut grid = TestGrid::across();
        let (source, floor) = (IVec3::new(15, 0, 5), IVec3::new(15, -1, 5));
        grid.blocks.insert(source, WATER);
        grid.blocks.insert(floor, STONE);
        let mut ticks = FluidTicks::game();
        ticks.add(source, false, 0);
        ticks.advance(&mut grid, &rules);
        assert!(grid.writes.contains(&IVec3::new(16, 0, 5)));
        assert_eq!(grid.blocks[&IVec3::new(16, 0, 5)], WATER);
    }

    #[test]
    fn a_generation_runs_due_updates_in_time_order() {
        let rules = rules();
        let mut grid = TestGrid::around();
        grid.blocks.insert(IVec3::new(1, 0, 1), WATER);
        grid.blocks.insert(IVec3::new(2, 0, 1), WATER);
        let mut ticks = FluidTicks::generation_clock(1);
        ticks.add(IVec3::new(1, 0, 1), false, 5);
        ticks.add(IVec3::new(2, 0, 1), false, 1);
        ticks.advance_generation(&mut grid, &rules);
        assert_eq!(grid.writes, vec![IVec3::new(2, -1, 1), IVec3::new(1, -1, 1)]);
    }

    #[test]
    fn each_dimension_has_its_own_lava_motion() {
        let nether = rules_with(LiquidMotion::WATER, LiquidMotion::FAST_LAVA);
        let overworld = rules_with(LiquidMotion::WATER, LiquidMotion::LAVA);
        assert_eq!(nether.motion(true), LiquidMotion { delay: 10, decay: 1 });
        assert_eq!(overworld.motion(true), LiquidMotion { delay: 30, decay: 2 });
        assert_eq!(nether.motion(false), LiquidMotion { delay: 5, decay: 1 });
        assert_eq!(overworld.motion(false), LiquidMotion { delay: 5, decay: 1 });
    }

    #[test]
    fn fast_lava_fires_ten_ticks_after_it_is_scheduled() {
        let rules = rules_with(LiquidMotion::WATER, LiquidMotion::FAST_LAVA);
        let mut grid = TestGrid::around();
        let source = IVec3::new(5, 0, 5);
        grid.blocks.insert(source, LAVA);
        let mut ticks = FluidTicks::game();
        ticks.add(source, true, rules.motion(true).delay);
        for _ in 0..9 {
            ticks.advance(&mut grid, &rules);
            assert!(grid.writes.is_empty(), "it fired before its delay");
        }
        ticks.advance(&mut grid, &rules);
        assert_eq!(grid.writes, vec![IVec3::new(5, -1, 5)]);
        assert_eq!(grid.blocks[&IVec3::new(5, -1, 5)], LAVA_FALLING);
    }
}
