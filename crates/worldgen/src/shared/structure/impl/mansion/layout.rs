use super::compass::{Pos, Side, go, lift, spin};
use super::floor_plan::{FloorPlan, HALL, LOBBY, ROOM, Zones};
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::place::Rotation;

const GROUND_FLOOR_RISE: i32 = 8;
const THIRD_FLOOR_RISE: i32 = 3;
const CELL: i32 = 8;
const FRONT_OFFSET: i32 = 16;
const ENTRANCE_BACK: i32 = 9;
const TOP_WALL_RISE: i32 = 19;
const LOWER_ROOF_RISE: i32 = 16;
const UPPER_ROOF_RISE: i32 = 27;
const CARPET_RISE: i32 = 1;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mirror {
    None,
    LeftRight,
    FrontBack,
}

#[derive(Clone, Debug)]
pub struct Spec {
    pub template: String,
    pub at: Pos,
    pub rotation: Rotation,
    pub mirror: Mirror,
}

struct Wall {
    at: Pos,
    rotation: Rotation,
    template: &'static str,
}

pub struct Layout<'a> {
    pub plan: &'a FloorPlan,
    pub random: &'a mut MersenneTwister,
    pub specs: Vec<Spec>,
    pub anchor: (i32, i32),
}

impl<'a> Layout<'a> {
    pub fn build(plan: &'a FloorPlan, random: &'a mut MersenneTwister, origin: Pos, rotation: Rotation) -> Vec<Spec> {
        let (ex, ey) = FloorPlan::entrance();
        let mut layout = Self {
            plan,
            random,
            specs: Vec::new(),
            anchor: (ex + 1, ey + 1),
        };
        layout.shell(origin, rotation);
        layout.roofs(origin, rotation);
        for floor in 0..3 {
            layout.floor(origin, rotation, floor);
        }
        layout.specs
    }

    pub fn push(&mut self, template: impl Into<String>, at: Pos, rotation: Rotation, mirror: Mirror) {
        self.specs.push(Spec {
            template: template.into(),
            at,
            rotation,
            mirror,
        });
    }

    pub fn cell_origin(&self, floor_origin: Pos, rotation: Rotation, x: i32, y: i32, east: i32) -> Pos {
        let south = go(floor_origin, rotation, Side::South, CELL + (y - self.anchor.1) * CELL);
        go(south, rotation, Side::East, east + (x - self.anchor.0) * CELL)
    }

    fn shell(&mut self, origin: Pos, rotation: Rotation) {
        let (ex, ey) = FloorPlan::entrance();
        let mut lower = Wall {
            at: origin,
            rotation,
            template: "wall_flat",
        };
        self.push("entrance", go(lower.at, rotation, Side::West, ENTRANCE_BACK), rotation, Mirror::None);
        lower.at = go(lower.at, rotation, Side::South, FRONT_OFFSET);
        let mut upper = Wall {
            at: lift(lower.at, GROUND_FLOOR_RISE),
            rotation,
            template: "wall_window",
        };
        let plan = self.plan;
        let ground = &plan.ground;
        let start = self.anchor;
        let finish = (ex + 1, ey);
        self.trace(&mut lower, ground, start, finish);
        self.trace(&mut upper, ground, start, finish);
        let top = &plan.top;
        let mut crown = Wall {
            at: lift(lower.at, TOP_WALL_RISE),
            rotation,
            template: "wall_window",
        };
        let first = (0..Zones::size()).flat_map(|y| (0..Zones::size()).rev().map(move |x| (x, y))).find(|&(x, y)| top.is_house(x, y));
        if let Some((x, y)) = first {
            crown.at = go(crown.at, rotation, Side::South, CELL + (y - start.1) * CELL);
            crown.at = go(crown.at, rotation, Side::East, (x - start.0) * CELL);
            self.straight(&mut crown);
            self.trace(&mut crown, top, (x, y), (x, y));
        }
    }

    fn trace(&mut self, wall: &mut Wall, zones: &Zones, start: (i32, i32), finish: (i32, i32)) {
        let (mut x, mut y, mut heading) = (start.0, start.1, Side::South);
        loop {
            let (ahead_x, ahead_y) = heading.beside((x, y));
            let (left_x, left_y) = heading.left().beside((ahead_x, ahead_y));
            if !zones.is_house(ahead_x, ahead_y) {
                self.corner(wall);
                heading = heading.right();
                if (x, y) != finish || heading != Side::South {
                    self.straight(wall);
                }
            } else if zones.is_house(left_x, left_y) {
                wall.at = go(wall.at, wall.rotation, Side::South, 6);
                wall.at = go(wall.at, wall.rotation, Side::East, CELL);
                wall.rotation = spin(wall.rotation, 3);
                (x, y) = (ahead_x, ahead_y);
                heading = heading.left();
            } else {
                (x, y) = (ahead_x, ahead_y);
                if (x, y) != finish || heading != Side::South {
                    self.straight(wall);
                }
            }
            if (x, y) == finish && heading == Side::South {
                break;
            }
        }
    }

    fn corner(&mut self, wall: &mut Wall) {
        wall.at = go(wall.at, wall.rotation, Side::South, -1);
        self.push("wall_corner", wall.at, wall.rotation, Mirror::None);
        wall.at = go(wall.at, wall.rotation, Side::South, -7);
        wall.at = go(wall.at, wall.rotation, Side::West, -6);
        wall.rotation = spin(wall.rotation, 1);
    }

    fn straight(&mut self, wall: &mut Wall) {
        self.push(wall.template, go(wall.at, wall.rotation, Side::East, 7), wall.rotation, Mirror::None);
        wall.at = go(wall.at, wall.rotation, Side::South, CELL);
    }

    fn floor(&mut self, origin: Pos, rotation: Rotation, floor: usize) {
        let base = lift(origin, GROUND_FLOOR_RISE * floor as i32 + if floor == 2 { THIRD_FLOOR_RISE } else { 0 });
        self.halls(base, rotation, floor);
        self.chambers(base, rotation, floor);
    }

    fn halls(&mut self, base: Pos, rotation: Rotation, floor: usize) {
        let plan = self.plan;
        let zones = plan.zones(floor);
        let rooms = &plan.rooms[floor];
        let suffix = if floor == 0 { "" } else { "_2" };
        for (x, y) in Zones::positions() {
            if zones.at(x, y) != HALL {
                continue;
            }
            let at = self.cell_origin(base, rotation, x, y, 0);
            let walk = |from: Pos, side: Side, n: i32| go(from, rotation, side, n);
            let joined = |a: i32, b: i32| zones.at(a, b) == HALL || rooms.at(a, b).carpet;
            self.specs.push(Spec {
                template: format!("corridor_floor{suffix}"),
                at,
                rotation,
                mirror: Mirror::None,
            });
            if joined(x, y - 1) {
                self.push("carpet_north", lift(walk(at, Side::East, 1), CARPET_RISE), rotation, Mirror::None);
            }
            if joined(x + 1, y) {
                self.push("carpet_east", lift(walk(walk(at, Side::South, 1), Side::East, 5), CARPET_RISE), rotation, Mirror::None);
            }
            if joined(x, y + 1) {
                self.push(format!("carpet_south{suffix}"), walk(walk(at, Side::South, 5), Side::West, 1), rotation, Mirror::None);
            }
            if joined(x - 1, y) {
                self.push(format!("carpet_west{suffix}"), walk(walk(at, Side::West, 1), Side::North, 1), rotation, Mirror::None);
            }
        }
    }

    fn chambers(&mut self, base: Pos, rotation: Rotation, floor: usize) {
        let plan = self.plan;
        let zones = plan.zones(floor);
        for (x, y) in Zones::positions() {
            let lobby = floor == 2 && zones.at(x, y) == LOBBY;
            if zones.at(x, y) != ROOM && !lobby {
                continue;
            }
            let booking = plan.rooms[floor].at(x, y);
            self.chamber(base, rotation, floor, (x, y), booking, lobby && booking.carpet);
        }
    }

    fn roofs(&mut self, origin: Pos, rotation: Rotation) {
        let plan = self.plan;
        let (ground, top) = (&plan.ground, &plan.top);
        self.roof(lift(origin, LOWER_ROOF_RISE), rotation, ground, Some(top));
        self.roof(lift(origin, UPPER_ROOF_RISE), rotation, top, None);
    }
}
