use super::compass::Side;
use crate::shared::rng::mersenne::MersenneTwister;

pub const OPEN: u8 = 0;
pub const HALL: u8 = 1;
pub const ROOM: u8 = 2;
pub const LOBBY: u8 = 3;
pub const SOLID: u8 = 5;

const SIZE: i32 = 11;
const ENTRANCE: (i32, i32) = (7, 4);
const FIRST_ROOM_ID: u16 = 10;
const BRANCH_TRIES: i32 = 8;
const SIDE_HALLS: [(i32, i32, i32); 2] = [(0, -2, 6), (0, 3, 6)];
const INNER_HALLS: [(i32, i32, i32); 2] = [(-2, -1, 3), (-2, 2, 3)];
const THIRD_FLOOR_HALL_LENGTH: i32 = 4;
const SIDES: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
const DIAGONALS: [(i32, i32); 4] = [(1, 1), (-1, 1), (1, -1), (-1, -1)];

#[derive(Clone)]
pub struct Zones {
    cells: Vec<u8>,
    beyond: u8,
}

impl Zones {
    fn new() -> Self {
        Self {
            cells: vec![OPEN; (SIZE * SIZE) as usize],
            beyond: SOLID,
        }
    }

    pub fn size() -> i32 {
        SIZE
    }

    fn index(x: i32, y: i32) -> Option<usize> {
        ((0..SIZE).contains(&x) && (0..SIZE).contains(&y)).then(|| (x * SIZE + y) as usize)
    }

    pub fn at(&self, x: i32, y: i32) -> u8 {
        Self::index(x, y).map_or(self.beyond, |index| self.cells[index])
    }

    fn put(&mut self, x: i32, y: i32, zone: u8) {
        if let Some(index) = Self::index(x, y) {
            self.cells[index] = zone;
        }
    }

    fn put_if(&mut self, x: i32, y: i32, expected: u8, zone: u8) {
        if self.at(x, y) == expected {
            self.put(x, y, zone);
        }
    }

    fn fill(&mut self, (x0, y0): (i32, i32), (x1, y1): (i32, i32), zone: u8) {
        for y in y0..=y1 {
            for x in x0..=x1 {
                self.put(x, y, zone);
            }
        }
    }

    pub fn is_house(&self, x: i32, y: i32) -> bool {
        (HALL..=4).contains(&self.at(x, y))
    }

    fn touches(&self, x: i32, y: i32, zone: u8) -> bool {
        SIDES.iter().any(|&(dx, dy)| self.at(x + dx, y + dy) == zone)
    }

    pub fn positions() -> impl Iterator<Item = (i32, i32)> {
        (0..SIZE).flat_map(|y| (0..SIZE).map(move |x| (x, y)))
    }

    fn hall(&mut self, random: &mut MersenneTwister, (x, y): (i32, i32), heading: Side, length: i32) {
        if length <= 0 {
            return;
        }
        let (dx, dy) = heading.delta();
        self.put(x, y, HALL);
        self.put_if(x + dx, y + dy, OPEN, HALL);
        for _ in 0..BRANCH_TRIES {
            let branch = Side::from_draw(random.next_int_bounded(4));
            if branch == heading.behind() || branch == Side::East && random.next_boolean() {
                continue;
            }
            let (ahead_x, ahead_y) = (x + dx, y + dy);
            let (bx, by) = branch.delta();
            if self.at(ahead_x + bx, ahead_y + by) == OPEN && self.at(ahead_x + bx * 2, ahead_y + by * 2) == OPEN {
                self.hall(random, (ahead_x + bx, ahead_y + by), branch, length - 1);
                break;
            }
        }
        let (rx, ry) = heading.right().delta();
        let (lx, ly) = heading.left().delta();
        let spots = [(rx, ry), (lx, ly), (dx + rx, dy + ry), (dx + lx, dy + ly), (dx * 2, dy * 2), (rx * 2, ry * 2), (lx * 2, ly * 2)];
        for (sx, sy) in spots {
            self.put_if(x + sx, y + sy, OPEN, ROOM);
        }
    }

    fn close_pockets(&mut self) -> bool {
        let mut changed = false;
        for (x, y) in Self::positions() {
            if self.at(x, y) != OPEN {
                continue;
            }
            let walls = SIDES.iter().filter(|&&(dx, dy)| self.is_house(x + dx, y + dy)).count();
            let corners = DIAGONALS.iter().filter(|&&(dx, dy)| self.is_house(x + dx, y + dy)).count();
            if walls >= 3 || walls == 2 && corners <= 1 {
                self.put(x, y, ROOM);
                changed = true;
            }
        }
        changed
    }

    fn settle(&mut self) {
        while self.close_pockets() {}
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Shape {
    #[default]
    None,
    Single,
    Double,
    Quad,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Booking {
    pub id: u16,
    pub shape: Shape,
    pub origin: bool,
    pub door: bool,
    pub stairs: bool,
    pub carpet: bool,
}

impl Booking {
    const CARPET: Self = Self {
        id: 0,
        shape: Shape::None,
        origin: false,
        door: false,
        stairs: false,
        carpet: true,
    };

    fn is_free(&self) -> bool {
        self.id == 0 && !self.carpet
    }
}

#[derive(Clone)]
pub struct Rooms {
    cells: Vec<Booking>,
}

impl Rooms {
    fn new() -> Self {
        Self {
            cells: vec![Booking::default(); (SIZE * SIZE) as usize],
        }
    }

    pub fn at(&self, x: i32, y: i32) -> Booking {
        Zones::index(x, y).map_or_else(Booking::default, |index| self.cells[index])
    }

    fn put(&mut self, x: i32, y: i32, booking: Booking) {
        if let Some(index) = Zones::index(x, y) {
            self.cells[index] = booking;
        }
    }

    pub fn same_room(&self, x: i32, y: i32, id: u16) -> bool {
        self.at(x, y).id == id
    }

    pub fn partner_side(&self, x: i32, y: i32, id: u16) -> Option<Side> {
        Side::ALL.into_iter().find(|side| {
            let (nx, ny) = side.beside((x, y));
            self.same_room(nx, ny, id)
        })
    }

    fn carpet_strip(&mut self) {
        for y in ENTRANCE.1..=ENTRANCE.1 + 1 {
            self.put(ENTRANCE.0 + 1, y, Booking::CARPET);
        }
    }

    fn assign(&mut self, random: &mut MersenneTwister, zones: &Zones) {
        let mut cells: Vec<(i32, i32)> = Zones::positions().filter(|&(x, y)| zones.at(x, y) == ROOM).collect();
        for upper in (1..cells.len()).rev() {
            let lower = random.next_int_bounded(upper as i32) as usize;
            cells.swap(upper, lower);
        }
        let mut id = FIRST_ROOM_ID;
        for (x, y) in cells {
            if !self.at(x, y).is_free() {
                continue;
            }
            let vacant = |a: i32, b: i32| self.at(a, b).is_free() && zones.at(a, b) == ROOM;
            let (mut x0, mut x1, mut y0, mut y1) = (x, x, y, y);
            let shape = if vacant(x + 1, y) && vacant(x, y + 1) && vacant(x + 1, y + 1) {
                (x1, y1) = (x + 1, y + 1);
                Shape::Quad
            } else if vacant(x - 1, y) && vacant(x, y + 1) && vacant(x - 1, y + 1) {
                (x0, y1) = (x - 1, y + 1);
                Shape::Quad
            } else if vacant(x - 1, y) && vacant(x, y - 1) && vacant(x - 1, y - 1) {
                (x0, y0) = (x - 1, y - 1);
                Shape::Quad
            } else if vacant(x + 1, y) {
                x1 = x + 1;
                Shape::Double
            } else if vacant(x, y + 1) {
                y1 = y + 1;
                Shape::Double
            } else if vacant(x - 1, y) {
                x0 = x - 1;
                Shape::Double
            } else if vacant(x, y - 1) {
                y0 = y - 1;
                Shape::Double
            } else {
                Shape::Single
            };
            let mut door_x = if random.next_boolean() { x0 } else { x1 };
            let mut door_y = if random.next_boolean() { y0 } else { y1 };
            let other = |value: i32, low: i32, high: i32| if value == low { high } else { low };
            let mut has_door = true;
            if !zones.touches(door_x, door_y, HALL) {
                door_x = other(door_x, x0, x1);
                door_y = other(door_y, y0, y1);
                if !zones.touches(door_x, door_y, HALL) {
                    door_y = other(door_y, y0, y1);
                    if !zones.touches(door_x, door_y, HALL) {
                        door_x = other(door_x, x0, x1);
                        door_y = other(door_y, y0, y1);
                        if !zones.touches(door_x, door_y, HALL) {
                            has_door = false;
                            (door_x, door_y) = (x0, y0);
                        }
                    }
                }
            }
            for cy in y0..=y1 {
                for cx in x0..=x1 {
                    let anchor = (cx, cy) == (door_x, door_y);
                    self.put(
                        cx,
                        cy,
                        Booking {
                            id,
                            shape,
                            origin: anchor,
                            door: anchor && has_door,
                            stairs: false,
                            carpet: false,
                        },
                    );
                }
            }
            id += 1;
        }
    }
}

pub struct FloorPlan {
    pub ground: Zones,
    pub top: Zones,
    pub rooms: [Rooms; 3],
}

impl FloorPlan {
    pub fn entrance() -> (i32, i32) {
        ENTRANCE
    }

    pub fn generate(random: &mut MersenneTwister) -> Self {
        let (ex, ey) = ENTRANCE;
        let mut ground = Zones::new();
        ground.fill((ex, ey), (ex + 1, ey + 1), LOBBY);
        ground.fill((ex - 1, ey), (ex - 1, ey + 1), ROOM);
        ground.fill((ex + 2, ey - 2), (ex + 3, ey + 3), SOLID);
        ground.fill((ex + 1, ey - 2), (ex + 1, ey - 1), HALL);
        ground.fill((ex + 1, ey + 2), (ex + 1, ey + 3), HALL);
        ground.put(ex - 1, ey - 1, HALL);
        ground.put(ex - 1, ey + 2, HALL);
        ground.fill((0, 0), (SIZE, 1), SOLID);
        ground.fill((0, SIZE - 2), (SIZE, SIZE), SOLID);
        for (dx, dy, length) in SIDE_HALLS.into_iter().chain(INNER_HALLS) {
            ground.hall(random, (ex + dx, ey + dy), Side::West, length);
        }
        ground.settle();
        let mut rooms = [Rooms::new(), Rooms::new(), Rooms::new()];
        rooms[0].assign(random, &ground);
        rooms[1].assign(random, &ground);
        rooms[0].carpet_strip();
        rooms[1].carpet_strip();
        let mut plan = Self { top: Zones::new(), ground, rooms };
        plan.raise_top(random);
        let top = plan.top.clone();
        plan.rooms[2].assign(random, &top);
        plan
    }

    fn close_top(&mut self) {
        self.top.fill((0, 0), (SIZE, SIZE), SOLID);
    }

    fn raise_top(&mut self, random: &mut MersenneTwister) {
        let candidates: Vec<(i32, i32)> = Zones::positions()
            .filter(|&(x, y)| {
                let booking = self.rooms[1].at(x, y);
                booking.shape == Shape::Double && booking.door
            })
            .collect();
        if candidates.is_empty() {
            self.close_top();
            return;
        }
        let (sx, sy) = candidates[random.next_int_bounded(candidates.len() as i32) as usize];
        let original = self.rooms[1].at(sx, sy);
        self.rooms[1].put(sx, sy, Booking { stairs: true, ..original });
        let partner = self.rooms[1].partner_side(sx, sy, original.id).expect("a double room has a second cell");
        let (tx, ty) = partner.beside((sx, sy));
        for (x, y) in Zones::positions() {
            if !self.ground.is_house(x, y) {
                self.top.put(x, y, SOLID);
            } else if (x, y) == (sx, sy) {
                self.top.put(x, y, LOBBY);
            } else if (x, y) == (tx, ty) {
                self.top.put(x, y, LOBBY);
                self.rooms[2].put(x, y, Booking::CARPET);
            }
        }
        let open: Vec<Side> = Side::ALL
            .into_iter()
            .filter(|side| {
                let (nx, ny) = side.beside((tx, ty));
                self.top.at(nx, ny) == OPEN
            })
            .collect();
        if open.is_empty() {
            self.close_top();
            self.rooms[1].put(sx, sy, original);
            return;
        }
        let heading = open[random.next_int_bounded(open.len() as i32) as usize];
        self.top.hall(random, heading.beside((tx, ty)), heading, THIRD_FLOOR_HALL_LENGTH);
        self.top.settle();
    }

    pub fn zones(&self, floor: usize) -> &Zones {
        if floor == 2 { &self.top } else { &self.ground }
    }
}
