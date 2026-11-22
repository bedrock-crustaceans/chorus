use glam::IVec3;

mod anemone;
mod crust;
mod hang;
mod kinds;
mod reef;

pub use crust::CoralCrust;
#[cfg(test)]
pub(super) use crust::template_count;
pub use kinds::Coral;

const CORAL_COLORS: usize = 5;

const FAN_DIRECTIONS: usize = 2;

const DOWN: usize = 0;

const NORTH: usize = 2;

const SOUTH: usize = 3;

const WEST: usize = 4;

const EAST: usize = 5;

const FACES: [IVec3; 6] = [IVec3::NEG_Y, IVec3::Y, IVec3::NEG_Z, IVec3::Z, IVec3::NEG_X, IVec3::X];

const WALL_DIRECTION: [i32; 4] = [2, 3, 0, 1];

const CLOCKWISE_TURN: [usize; 4] = [EAST, WEST, NORTH, SOUTH];

const COUNTER_TURN: [usize; 3] = [WEST, EAST, SOUTH];

const ROTATION_COUNT: i32 = 4;

fn turn(face: usize, clockwise: bool) -> usize {
    if clockwise {
        CLOCKWISE_TURN[face - NORTH]
    } else {
        COUNTER_TURN.get(face - NORTH).copied().unwrap_or(NORTH)
    }
}
