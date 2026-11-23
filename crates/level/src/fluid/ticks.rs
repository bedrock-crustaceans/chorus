use glam::IVec3;

use super::flow;
use super::{FluidRules, Grid, PendingUpdate};

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

type Key = (IVec3, bool, u64);

pub struct FluidTicks {
    now: u64,
    sequence: u64,
    pending: std::collections::BinaryHeap<std::cmp::Reverse<Order>>,
    scheduled: rustc_hash::FxHashMap<Key, u32>,
    due: Vec<ScheduledUpdate>,
    parked: rustc_hash::FxHashMap<(i32, i32), Vec<ScheduledUpdate>>,
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
            pending: std::collections::BinaryHeap::new(),
            scheduled: rustc_hash::FxHashMap::default(),
            due: Vec::new(),
            parked: rustc_hash::FxHashMap::default(),
        }
    }

    pub fn now(&self) -> u64 {
        self.now
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty() && self.parked.is_empty()
    }

    pub fn len(&self) -> usize {
        self.pending.len() + self.parked.values().map(Vec::len).sum::<usize>()
    }

    pub fn peek_chunk(&self, chunk: (i32, i32)) -> Vec<PendingUpdate> {
        let remaining = |due: u64| due.saturating_sub(self.now);
        let mut updates: Vec<PendingUpdate> = self
            .pending
            .iter()
            .map(|entry| entry.0)
            .filter(|order| (order.x >> 4, order.z >> 4) == chunk)
            .map(|order| PendingUpdate {
                pos: IVec3::new(order.x, order.y, order.z),
                lava: order.lava,
                delay: remaining(order.due),
            })
            .collect();
        if let Some(parked) = self.parked.get(&chunk) {
            updates.extend(parked.iter().map(|update| PendingUpdate {
                pos: update.pos,
                lava: update.lava,
                delay: remaining(update.due),
            }));
        }
        updates
    }

    pub fn drop_chunk(&mut self, chunk: (i32, i32)) {
        self.parked.remove(&chunk);
        let entries = std::mem::take(&mut self.pending).into_vec();
        let mut kept = Vec::with_capacity(entries.len());
        for entry in entries {
            if (entry.0.x >> 4, entry.0.z >> 4) == chunk {
                self.release(entry.0.update());
            } else {
                kept.push(entry);
            }
        }
        self.pending = std::collections::BinaryHeap::from(kept);
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
        self.sequence += 1;
        self.pending.push(std::cmp::Reverse(Order::of(update, self.sequence)));
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
        let update = self.pending.pop().map(|entry| entry.0.update())?;
        self.release(update);
        Some(update)
    }

    pub fn advance<G: Grid>(&mut self, grid: &mut G, rules: &FluidRules) {
        self.now += 1;
        self.due.clear();
        self.release_parked(grid);
        while let Some(update) = self.next_due(self.now) {
            if grid.can_tick(update.pos) {
                self.due.push(update);
            } else if grid.is_loaded(update.pos) {
                self.parked.entry((update.pos.x >> 4, update.pos.z >> 4)).or_default().push(update);
            }
        }
        self.due.sort_unstable_by_key(|update| (update.pos.x, update.pos.y, update.pos.z, update.lava));
        for index in 0..self.due.len() {
            let update = self.due[index];
            flow::step(grid, rules, self, update);
        }
    }

    fn release_parked<G: Grid>(&mut self, grid: &G) {
        if self.parked.is_empty() {
            return;
        }
        let due = &mut self.due;
        self.parked.retain(|_, updates| {
            let Some(first) = updates.first() else { return false };
            if !grid.is_loaded(first.pos) {
                return false;
            }
            if grid.can_tick(first.pos) {
                due.append(updates);
                return false;
            }
            true
        });
    }

    fn next_due(&mut self, now: u64) -> Option<ScheduledUpdate> {
        let due = self.pending.peek().map(|entry| entry.0.due)?;
        if due <= now { self.take_next() } else { None }
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

        fn is_loaded(&self, pos: IVec3) -> bool {
            self.loaded.contains(&(pos.x >> 4, pos.z >> 4))
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
    fn generated_ticks_that_cannot_change_anything_are_dropped() {
        let rules = rules();
        let mut chunk = crate::chunk::Chunk::new(0, 0, -4, 24, AIR, 1);
        let enclosed = IVec3::new(5, 0, 5);
        for offset in [IVec3::X, IVec3::NEG_X, IVec3::Y, IVec3::NEG_Y, IVec3::Z, IVec3::NEG_Z] {
            let at = enclosed + offset;
            chunk.set_block(at.x as u8, at.y, at.z as u8, 0, STONE);
        }
        chunk.set_block(5, 0, 5, 0, WATER);
        let open = IVec3::new(10, 0, 10);
        chunk.set_block(10, 0, 10, 0, WATER);
        let border = IVec3::new(0, 0, 3);
        chunk.set_block(0, 0, 3, 0, WATER);
        let update = |pos| crate::fluid::PendingUpdate { pos, lava: false, delay: 1 };
        let kept = crate::fluid::retain_active(&chunk, &rules, vec![update(enclosed), update(open), update(border)]);
        assert_eq!(kept.iter().map(|update| update.pos).collect::<Vec<_>>(), vec![open, border]);
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
    fn updates_of_an_unloaded_chunk_are_dropped() {
        let rules = rules();
        let (mut grid, mut ticks) = source_in_the_air();
        grid.loaded.remove(&(0, 0));
        for _ in 0..WATER_DELAY + 2 {
            ticks.advance(&mut grid, &rules);
        }
        assert!(ticks.is_empty());
        assert!(grid.writes.is_empty());
    }

    #[test]
    fn updates_wait_for_the_neighbouring_chunks_and_do_not_cost_anything_meanwhile() {
        let rules = rules();
        let (mut grid, mut ticks) = source_in_the_air();
        grid.loaded.remove(&(1, 0));
        for _ in 0..WATER_DELAY + 2 {
            ticks.advance(&mut grid, &rules);
        }
        assert_eq!(ticks.len(), 1);
        assert!(grid.writes.is_empty());
        grid.loaded.insert((1, 0));
        ticks.advance(&mut grid, &rules);
        assert_eq!(grid.writes, vec![IVec3::new(5, -1, 5)]);
    }

    #[test]
    fn parked_updates_are_dropped_when_their_chunk_unloads() {
        let rules = rules();
        let (mut grid, mut ticks) = source_in_the_air();
        grid.loaded.remove(&(1, 0));
        for _ in 0..WATER_DELAY + 2 {
            ticks.advance(&mut grid, &rules);
        }
        assert_eq!(ticks.len(), 1);
        grid.loaded.remove(&(0, 0));
        ticks.advance(&mut grid, &rules);
        assert!(ticks.is_empty());
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
