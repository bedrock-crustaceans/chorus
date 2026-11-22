use super::compass::{Pos, Side, go, spin};
use super::floor_plan::{Booking, HALL, Shape};
use super::layout::{Layout, Mirror};
use crate::shared::structure::place::Rotation;

const WALL_REACH: i32 = 8;
const SOUTH_WALL_DEPTH: i32 = 7;
const NORTH_WALL_BACK: i32 = 1;
const WALL_SLIDE: i32 = 7;

#[derive(Clone, Copy)]
enum Wing {
    Ground,
    Upper,
}

#[derive(Clone, Copy)]
enum Doorway {
    Side,
    Front,
    Secret,
}

fn single_corner(facing: Option<Side>) -> (Rotation, (i32, i32)) {
    match facing {
        Some(Side::East) | None => (Rotation::None, (1, 0)),
        Some(Side::South) => (Rotation::Clockwise90, (7, 0)),
        Some(Side::West) => (Rotation::Clockwise180, (7, 6)),
        Some(Side::North) => (Rotation::CounterClockwise90, (1, 6)),
    }
}

fn double_placement(door: Option<Side>, partner: Option<Side>) -> Option<(Doorway, (i32, i32), i32, Mirror)> {
    use Mirror::{FrontBack, LeftRight, None as Plain};
    use Side::{East, North, South, West};
    let side = |east, south, turns, mirror| Some((Doorway::Side, (east, south), turns, mirror));
    let front = |east, south, turns| Some((Doorway::Front, (east, south), turns, Plain));
    let secret = |east, south, turns| Some((Doorway::Secret, (east, south), turns, Plain));
    match (door, partner) {
        (Some(East), Some(South)) => side(1, 0, 0, Plain),
        (Some(East), Some(North)) => side(1, 6, 0, LeftRight),
        (Some(West), Some(North)) => side(7, 6, 2, Plain),
        (Some(West), Some(South)) => side(7, 0, 0, FrontBack),
        (Some(South), Some(East)) => side(1, 0, 1, LeftRight),
        (Some(South), Some(West)) => side(7, 0, 1, Plain),
        (Some(North), Some(West)) => side(7, 6, 1, FrontBack),
        (Some(North), Some(East)) => side(1, 6, 3, Plain),
        (Some(South), Some(North)) => front(1, -8, 0),
        (Some(North), Some(South)) => front(7, 14, 2),
        (Some(West), Some(East)) => front(15, 0, 1),
        (Some(East), Some(West)) => front(-7, 6, 3),
        (None, Some(East)) => secret(15, 0, 1),
        (None, Some(South)) => secret(1, 0, 0),
        _ => None,
    }
}

fn quad_placement(door: Side, partner: Side) -> ((i32, i32), i32, Mirror) {
    use Mirror::{FrontBack, LeftRight, None as Plain};
    use Side::{East, North, South, West};
    match (door, partner) {
        (East, South) => ((-7, 0), 0, Plain),
        (East, North) => ((-7, 6), 0, LeftRight),
        (North, East) => ((1, 14), 3, Plain),
        (North, West) => ((7, 14), 3, LeftRight),
        (South, West) => ((7, -8), 1, Plain),
        (South, East) => ((1, -8), 1, LeftRight),
        (West, North) => ((15, 6), 2, Plain),
        (West, South) => ((15, 0), 0, FrontBack),
        _ => ((0, 0), 0, Plain),
    }
}

impl Layout<'_> {
    fn numbered(&mut self, prefix: &str, count: i32) -> String {
        format!("{prefix}{}", self.random.next_int_bounded(count) + 1)
    }

    fn single_name(&mut self, wing: Wing) -> String {
        match wing {
            Wing::Ground => self.numbered("1x1_a", 5),
            Wing::Upper => self.numbered("1x1_b", 5),
        }
    }

    fn double_name(&mut self, wing: Wing, doorway: Doorway, stairs: bool) -> String {
        match (wing, doorway) {
            (Wing::Ground, Doorway::Side) => self.numbered("1x2_a", 9),
            (Wing::Ground, Doorway::Front) => self.numbered("1x2_b", 5),
            (Wing::Ground, Doorway::Secret) => self.numbered("1x2_s", 2),
            (Wing::Upper, Doorway::Side) if stairs => "1x2_c_stairs".to_string(),
            (Wing::Upper, Doorway::Side) => self.numbered("1x2_c", 4),
            (Wing::Upper, Doorway::Front) if stairs => "1x2_d_stairs".to_string(),
            (Wing::Upper, Doorway::Front) => self.numbered("1x2_d", 5),
            (Wing::Upper, Doorway::Secret) => self.numbered("1x2_se", 1),
        }
    }

    fn quad_name(&mut self, wing: Wing) -> String {
        match wing {
            Wing::Ground => self.numbered("2x2_a", 4),
            Wing::Upper => self.numbered("2x2_b", 5),
        }
    }

    pub fn chamber(&mut self, base: Pos, rotation: Rotation, floor: usize, (x, y): (i32, i32), booking: Booking, stairwell: bool) {
        let plan = self.plan;
        let zones = plan.zones(floor);
        let rooms = &plan.rooms[floor];
        let id = booking.id;
        let hall_sides: Vec<Side> = if booking.door {
            Side::ALL
                .into_iter()
                .filter(|side| {
                    let (nx, ny) = side.beside((x, y));
                    zones.at(nx, ny) == HALL
                })
                .collect()
        } else {
            Vec::new()
        };
        let door = if !hall_sides.is_empty() {
            Some(Some(hall_sides[self.random.next_int_bounded(hall_sides.len() as i32) as usize]))
        } else if booking.origin {
            Some(None)
        } else {
            None
        };
        let corner = self.cell_origin(base, rotation, x, y, -1);
        let (wall, door_wall) = if floor == 0 { ("indoors_wall", "indoors_door") } else { ("indoors_wall_2", "indoors_door_2") };
        let pick = |side: Side| if door == Some(Some(side)) { door_wall } else { wall };
        let partitions = |side: Side| {
            let (nx, ny) = side.beside((x, y));
            zones.is_house(nx, ny) && !rooms.same_room(nx, ny, id)
        };
        let hall_beyond = |side: Side| zones.at(side.beside((x, y)).0, side.beside((x, y)).1) == HALL;
        if partitions(Side::West) {
            self.push(pick(Side::West), corner, rotation, Mirror::None);
        }
        if hall_beyond(Side::East) && !stairwell {
            self.push(pick(Side::East), go(corner, rotation, Side::East, WALL_REACH), rotation, Mirror::None);
        }
        if partitions(Side::South) {
            let at = go(go(corner, rotation, Side::South, SOUTH_WALL_DEPTH), rotation, Side::East, WALL_SLIDE);
            self.push(pick(Side::South), at, spin(rotation, 1), Mirror::None);
        }
        if hall_beyond(Side::North) && !stairwell {
            let at = go(go(corner, rotation, Side::North, NORTH_WALL_BACK), rotation, Side::East, WALL_SLIDE);
            self.push(pick(Side::North), at, spin(rotation, 1), Mirror::None);
        }
        let wing = if floor == 0 { Wing::Ground } else { Wing::Upper };
        match (booking.shape, door) {
            (Shape::Single, door) => self.single_room(corner, rotation, door.flatten(), wing),
            (Shape::Double, Some(door)) => {
                let partner = rooms.partner_side(x, y, id);
                self.double_room(corner, rotation, partner, door, wing, booking.stairs);
            }
            (Shape::Quad, Some(Some(door))) => {
                let mut partner = door.right();
                let (px, py) = partner.beside((x, y));
                if !rooms.same_room(px, py, id) {
                    partner = partner.behind();
                }
                self.quad_room(corner, rotation, partner, door, wing);
            }
            (Shape::Quad, Some(None)) => self.push("2x2_s1", go(corner, rotation, Side::East, 1), rotation, Mirror::None),
            _ => {}
        }
    }

    fn single_room(&mut self, corner: Pos, rotation: Rotation, door: Option<Side>, wing: Wing) {
        let mut name = self.single_name(wing);
        if door.is_none() {
            name = self.numbered("1x1_as", 4);
        }
        let (turned, (dx, dz)) = single_corner(door);
        let (ox, _, oz) = rotation.apply((dx, 0, dz));
        self.push(name, (corner.0 + ox, corner.1, corner.2 + oz), spin(turned, rotation as i32), Mirror::None);
    }

    fn double_room(&mut self, corner: Pos, rotation: Rotation, partner: Option<Side>, door: Option<Side>, wing: Wing, stairs: bool) {
        let Some((doorway, (east, south), turns, mirror)) = double_placement(door, partner) else { return };
        let name = self.double_name(wing, doorway, stairs);
        let at = go(go(corner, rotation, Side::East, east), rotation, Side::South, south);
        self.push(name, at, spin(rotation, turns), mirror);
    }

    fn quad_room(&mut self, corner: Pos, rotation: Rotation, partner: Side, door: Side, wing: Wing) {
        let ((east, south), turns, mirror) = quad_placement(door, partner);
        let name = self.quad_name(wing);
        let at = go(go(corner, rotation, Side::East, east), rotation, Side::South, south);
        self.push(name, at, spin(rotation, turns), mirror);
    }
}
