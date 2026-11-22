use chorus_level::dimension_definition::DimensionDefinition;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundingBox {
    pub x0: i32,
    pub y0: i32,
    pub z0: i32,
    pub x1: i32,
    pub y1: i32,
    pub z1: i32,
}

impl BoundingBox {
    pub const fn new(x0: i32, y0: i32, z0: i32, x1: i32, y1: i32, z1: i32) -> Self {
        Self { x0, y0, z0, x1, y1, z1 }
    }

    pub fn chunk(chunk_x: i32, chunk_z: i32, definition: DimensionDefinition) -> Self {
        Self::new(chunk_x * 16, definition.min_y, chunk_z * 16, chunk_x * 16 + 15, definition.max_y(), chunk_z * 16 + 15)
    }

    pub fn intersects(&self, other: &BoundingBox) -> bool {
        self.x1 >= other.x0 && self.x0 <= other.x1 && self.z1 >= other.z0 && self.z0 <= other.z1 && self.y1 >= other.y0 && self.y0 <= other.y1
    }

    pub fn contains(&self, x: i32, y: i32, z: i32) -> bool {
        (self.x0..=self.x1).contains(&x) && (self.y0..=self.y1).contains(&y) && (self.z0..=self.z1).contains(&z)
    }

    pub fn union(&self, other: &BoundingBox) -> Self {
        Self::new(
            self.x0.min(other.x0),
            self.y0.min(other.y0),
            self.z0.min(other.z0),
            self.x1.max(other.x1),
            self.y1.max(other.y1),
            self.z1.max(other.z1),
        )
    }

    pub fn shifted_y(&self, dy: i32) -> Self {
        Self::new(self.x0, self.y0 + dy, self.z0, self.x1, self.y1 + dy, self.z1)
    }
}

pub(crate) const CHUNK_CORNERS: [(i32, i32); 4] = [(0, 0), (15, 0), (15, 15), (0, 15)];
