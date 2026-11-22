use super::beard::embankment_kernel;
use crate::shared::structure::BoundingBox;

const BOX_GROWTH: i32 = 2;
const COLUMN_REACH: i32 = 11;
const HEIGHT_REACH_BELOW: i32 = 12;
const HEIGHT_REACH_ABOVE: i32 = 11;
const SIZE_PADDING: i32 = 5;
const MARGIN_BELOW: f32 = 1.0;
const MARGIN_ABOVE: f32 = 5.0;
const FULL_SCALE: f32 = 1.5;

#[derive(Debug, Clone, Copy)]
pub struct Shaver {
    bounds: BoundingBox,
    ground: i32,
    middle: (i32, i32),
    radius: f32,
}

impl Shaver {
    pub fn new(piece: BoundingBox, ground_offset: i32) -> Self {
        let bounds = BoundingBox::new(piece.x0 - BOX_GROWTH, piece.y0, piece.z0 - BOX_GROWTH, piece.x1 + BOX_GROWTH, piece.y1, piece.z1 + BOX_GROWTH);
        let (size_x, size_z) = (piece.x1 - piece.x0 + SIZE_PADDING, piece.z1 - piece.z0 + SIZE_PADDING);
        Self {
            bounds,
            ground: piece.y0 + ground_offset,
            middle: (size_x / 2 + bounds.x0, size_z / 2 + bounds.z0),
            radius: (size_x as f32 * 0.5).max(size_z as f32 * 0.5),
        }
    }

    pub fn reaches_column(&self, x: i32, z: i32) -> bool {
        let (dx, dz) = self.distance_outside(x, z);
        dx <= COLUMN_REACH && dz <= COLUMN_REACH
    }

    pub fn heights(&self) -> (i32, i32) {
        (self.ground.min(self.bounds.y1) - HEIGHT_REACH_BELOW, self.ground.max(self.bounds.y1) + HEIGHT_REACH_ABOVE)
    }

    pub fn contribution(&self, x: i32, y: i32, z: i32) -> f32 {
        let above = y > self.ground;
        let clamped = if y > self.bounds.y1 {
            self.bounds.y1
        } else if above {
            y
        } else {
            self.ground
        };
        let (dx, dz) = self.distance_outside(x, z);
        let (offset_x, offset_y, offset_z) = ((x - self.middle.0) as f32, (y - clamped) as f32, (z - self.middle.1) as f32);
        let distance = (offset_x * offset_x + offset_y * offset_y + offset_z * offset_z).sqrt();
        let reach = if above { MARGIN_ABOVE } else { MARGIN_BELOW } + self.radius;
        let scale = if 0.0 < reach && distance <= reach { FULL_SCALE - distance / reach } else { 0.0 };
        embankment_kernel(dx, y - clamped, dz) * scale
    }

    fn distance_outside(&self, x: i32, z: i32) -> (i32, i32) {
        let bounds = &self.bounds;
        ((bounds.x0 - x).max(x - bounds.x1).max(0), (bounds.z0 - z).max(z - bounds.z1).max(0))
    }
}
