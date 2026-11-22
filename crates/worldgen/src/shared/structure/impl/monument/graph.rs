use crate::shared::rng::mersenne::MersenneTwister;

pub const DOWN: usize = 0;
pub const UP: usize = 1;
pub const NORTH: usize = 2;
pub const SOUTH: usize = 3;
pub const WEST: usize = 4;
pub const EAST: usize = 5;

const FACES: usize = 6;
const OPPOSITE: [usize; FACES] = [UP, DOWN, SOUTH, NORTH, EAST, WEST];
const STEPS: [(i32, i32, i32); FACES] = [(0, -1, 0), (0, 1, 0), (0, 0, -1), (0, 0, 1), (-1, 0, 0), (1, 0, 0)];
const SIZE_X: i32 = 5;
const SIZE_Y: i32 = 3;
const SIZE_Z: i32 = 5;
const GRID_ROOMS: usize = 75;
pub const TOP: usize = 75;
const LEFT_WING: usize = 76;
const RIGHT_WING: usize = 77;
const TOTAL_ROOMS: usize = 78;
const CORE_ROW: i32 = 2;
const CORE_COLUMNS: i32 = 4;
const SOURCE: (i32, i32, i32) = (2, 0, 0);
const MAX_CLOSED: u32 = 2;
const MAX_TRIES: u32 = 5;

#[derive(Debug, Clone, Default)]
pub struct Room {
    pub grid_index: usize,
    pub links: [Option<usize>; FACES],
    pub open: [bool; FACES],
    pub claimed: bool,
    pub is_source: bool,
    mark: u32,
}

pub struct Graph {
    rooms: Vec<Option<Room>>,
    pub core: usize,
    pub order: Vec<usize>,
}

pub fn slot(x: i32, y: i32, z: i32) -> usize {
    (y * SIZE_X * SIZE_Z + z * SIZE_X + x) as usize
}

fn link(rooms: &mut [Option<Room>], from: usize, face: usize, to: usize) {
    rooms[from].as_mut().expect("room").links[face] = Some(to);
    rooms[to].as_mut().expect("room").links[OPPOSITE[face]] = Some(from);
}

impl Graph {
    pub fn room(&self, slot: usize) -> &Room {
        self.rooms[slot].as_ref().expect("room")
    }

    fn room_mut(&mut self, slot: usize) -> &mut Room {
        self.rooms[slot].as_mut().expect("room")
    }

    pub fn through(&self, slot: usize, face: usize) -> usize {
        self.room(slot).links[face].expect("linked room")
    }

    pub fn is_free(&self, slot: usize, face: usize) -> bool {
        self.room(slot).open[face] && !self.room(self.through(slot, face)).claimed
    }

    pub fn claim(&mut self, slots: &[usize]) {
        for &slot in slots {
            self.room_mut(slot).claimed = true;
        }
    }

    pub fn build(random: &mut MersenneTwister, shuffle_seed: u32) -> Self {
        let mut rooms: Vec<Option<Room>> = vec![None; TOTAL_ROOMS];
        let floors = [(0, SIZE_X, 0, 4), (0, SIZE_X, 0, 4), (1, 4, 0, 2)];
        for (y, &(x0, x1, z0, z1)) in floors.iter().enumerate() {
            for x in x0..x1 {
                for z in z0..z1 {
                    let index = slot(x, y as i32, z);
                    rooms[index] = Some(Room { grid_index: index, ..Room::default() });
                }
            }
        }
        for x in 0..SIZE_X {
            for z in 0..SIZE_Z {
                for y in 0..SIZE_Y {
                    if rooms[slot(x, y, z)].is_none() {
                        continue;
                    }
                    for (face, &(dx, dy, dz)) in STEPS.iter().enumerate() {
                        let (nx, ny, nz) = (x + dx, y + dy, z + dz);
                        if !(0..SIZE_X).contains(&nx) || !(0..SIZE_Y).contains(&ny) || !(0..SIZE_Z).contains(&nz) || rooms[slot(nx, ny, nz)].is_none() {
                            continue;
                        }
                        let stored = if nz == z { face } else { OPPOSITE[face] };
                        link(&mut rooms, slot(x, y, z), stored, slot(nx, ny, nz));
                    }
                }
            }
        }
        for (index, grid_index) in [(TOP, 1003), (LEFT_WING, 1001), (RIGHT_WING, 1002)] {
            rooms[index] = Some(Room {
                grid_index,
                claimed: true,
                ..Room::default()
            });
        }
        link(&mut rooms, slot(2, 2, 0), UP, TOP);
        link(&mut rooms, slot(0, 1, 0), SOUTH, LEFT_WING);
        link(&mut rooms, slot(4, 1, 0), SOUTH, RIGHT_WING);
        let mut graph = Graph { rooms, core: 0, order: Vec::new() };
        graph.room_mut(slot(SOURCE.0, SOURCE.1, SOURCE.2)).is_source = true;
        graph.core = slot(random.next_int_bounded(CORE_COLUMNS), 0, CORE_ROW);
        let core = graph.core;
        let (east, north, above) = (graph.through(core, EAST), graph.through(core, NORTH), graph.through(core, UP));
        let north_east = graph.through(east, NORTH);
        let covered = [core, east, north, north_east, above, graph.through(east, UP), graph.through(north, UP), graph.through(north_east, UP)];
        graph.claim(&covered);
        let mut list: Vec<usize> = (0..GRID_ROOMS).filter(|&slot| graph.rooms[slot].is_some()).collect();
        for &slot in list.iter().chain([&TOP]) {
            let room = graph.room_mut(slot);
            room.open = room.links.map(|linked| linked.is_some());
        }
        let mut shuffle = MersenneTwister::new(shuffle_seed);
        for i in (1..list.len()).rev() {
            let j = shuffle.next_int_bounded(i as i32) as usize;
            list.swap(i, j);
        }
        graph.close_openings(&list, random);
        list.extend([TOP, LEFT_WING, RIGHT_WING]);
        graph.order = list;
        graph
    }

    fn close_openings(&mut self, list: &[usize], random: &mut MersenneTwister) {
        let mut mark = 1;
        for &slot in list {
            let (mut closed, mut tries) = (0, 0);
            while closed < MAX_CLOSED && tries < MAX_TRIES {
                tries += 1;
                let face = random.next_int_bounded(FACES as i32) as usize;
                if !self.room(slot).open[face] {
                    continue;
                }
                let other = self.through(slot, face);
                self.room_mut(slot).open[face] = false;
                self.room_mut(other).open[OPPOSITE[face]] = false;
                mark += 1;
                let first = self.reaches_source(slot, mark - 1);
                let second = first && {
                    mark += 1;
                    self.reaches_source(other, mark - 1)
                };
                if first && second {
                    closed += 1;
                } else {
                    self.room_mut(slot).open[face] = true;
                    self.room_mut(other).open[OPPOSITE[face]] = true;
                }
            }
        }
    }

    fn reaches_source(&mut self, slot: usize, mark: u32) -> bool {
        if self.room(slot).is_source {
            return true;
        }
        self.room_mut(slot).mark = mark;
        for face in 0..FACES {
            let room = self.room(slot);
            if let Some(next) = room.links[face]
                && room.open[face]
                && self.room(next).mark != mark
                && self.reaches_source(next, mark)
            {
                return true;
            }
        }
        false
    }
}
