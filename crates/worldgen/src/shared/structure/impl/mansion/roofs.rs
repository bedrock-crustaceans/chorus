use super::compass::{Pos, Side, go, spin};
use super::floor_plan::Zones;
use super::layout::{Layout, Mirror};
use crate::shared::structure::place::Rotation;

const ROOF_RISE: i32 = 3;

impl Layout<'_> {
    pub fn roof(&mut self, origin: Pos, rotation: Rotation, zones: &Zones, above: Option<&Zones>) {
        let walk = |from: Pos, side: Side, n: i32| go(from, rotation, side, n);
        let house = |x: i32, y: i32| zones.is_house(x, y);
        let covered = |x: i32, y: i32| above.is_some_and(|upper| upper.is_house(x, y));
        let exposed: Vec<(i32, i32)> = Zones::positions().filter(|&(x, y)| house(x, y) && !covered(x, y)).collect();
        for &(x, y) in &exposed {
            let at = self.cell_origin(origin, rotation, x, y, 0);
            self.push("roof", (at.0, at.1 + ROOF_RISE, at.2), rotation, Mirror::None);
            if !house(x + 1, y) {
                self.push("roof_front", walk(at, Side::East, 6), rotation, Mirror::None);
            }
            if !house(x - 1, y) {
                self.push("roof_front", walk(at, Side::South, 7), spin(rotation, 2), Mirror::None);
            }
            if !house(x, y - 1) {
                self.push("roof_front", walk(at, Side::West, 1), spin(rotation, 3), Mirror::None);
            }
            if !house(x, y + 1) {
                self.push("roof_front", walk(walk(at, Side::East, 6), Side::South, 6), spin(rotation, 1), Mirror::None);
            }
        }
        if above.is_some() {
            for (x, y) in Zones::positions().filter(|&(x, y)| house(x, y) && covered(x, y)) {
                let at = self.cell_origin(origin, rotation, x, y, 0);
                self.parapet(at, rotation, zones, (x, y));
            }
        }
        for &(x, y) in &exposed {
            let at = self.cell_origin(origin, rotation, x, y, 0);
            if !house(x + 1, y) {
                let edge = walk(at, Side::East, 6);
                if !house(x, y + 1) {
                    self.push("roof_corner", walk(edge, Side::South, 6), rotation, Mirror::None);
                } else if house(x + 1, y + 1) {
                    self.push("roof_inner_corner", walk(edge, Side::South, 5), rotation, Mirror::None);
                }
                if !house(x, y - 1) {
                    self.push("roof_corner", edge, spin(rotation, 3), Mirror::None);
                } else if house(x + 1, y - 1) {
                    self.push("roof_inner_corner", walk(walk(at, Side::East, 9), Side::North, 2), spin(rotation, 1), Mirror::None);
                }
            }
            if !house(x - 1, y) {
                if !house(x, y + 1) {
                    self.push("roof_corner", walk(at, Side::South, 6), spin(rotation, 1), Mirror::None);
                } else if house(x - 1, y + 1) {
                    self.push("roof_inner_corner", walk(walk(at, Side::South, 8), Side::West, 3), spin(rotation, 3), Mirror::None);
                }
                if !house(x, y - 1) {
                    self.push("roof_corner", at, spin(rotation, 2), Mirror::None);
                } else if house(x - 1, y - 1) {
                    self.push("roof_inner_corner", walk(at, Side::South, 1), spin(rotation, 2), Mirror::None);
                }
            }
        }
    }

    fn parapet(&mut self, at: Pos, rotation: Rotation, zones: &Zones, (x, y): (i32, i32)) {
        let walk = |from: Pos, side: Side, n: i32| go(from, rotation, side, n);
        let house = |a: i32, b: i32| zones.is_house(a, b);
        if !house(x + 1, y) {
            self.push("small_wall", walk(at, Side::East, 7), rotation, Mirror::None);
        }
        if !house(x - 1, y) {
            self.push("small_wall", walk(walk(at, Side::West, 1), Side::South, 6), spin(rotation, 2), Mirror::None);
        }
        if !house(x, y - 1) {
            self.push("small_wall", walk(at, Side::North, 1), spin(rotation, 3), Mirror::None);
        }
        if !house(x, y + 1) {
            self.push("small_wall", walk(walk(at, Side::East, 6), Side::South, 7), spin(rotation, 1), Mirror::None);
        }
        if !house(x + 1, y) {
            if !house(x, y - 1) {
                self.push("small_wall_corner", walk(walk(at, Side::East, 7), Side::North, 2), rotation, Mirror::None);
            }
            if !house(x, y + 1) {
                self.push("small_wall_corner", walk(walk(at, Side::East, 8), Side::South, 7), spin(rotation, 1), Mirror::None);
            }
        }
        if !house(x - 1, y) {
            if !house(x, y - 1) {
                self.push("small_wall_corner", walk(walk(at, Side::West, 2), Side::North, 1), spin(rotation, 3), Mirror::None);
            }
            if !house(x, y + 1) {
                self.push("small_wall_corner", walk(walk(at, Side::West, 1), Side::South, 8), spin(rotation, 2), Mirror::None);
            }
        }
    }
}
