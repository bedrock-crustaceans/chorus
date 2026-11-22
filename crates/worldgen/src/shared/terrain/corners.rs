use super::CELL_WIDTH;

pub(super) const CORNER_SPAN: i32 = CELLS + 1;

pub(super) const CELLS: i32 = 16 / CELL_WIDTH;

pub struct CornerColumns {
    corners: Vec<Corner>,
    levels: usize,
}

impl CornerColumns {
    pub const COLUMNS: i32 = CELLS;

    pub(crate) fn from_corners(corners: Vec<Corner>, levels: usize) -> Self {
        debug_assert_eq!(corners.len(), CELLS as usize * CELLS as usize * levels);
        Self { corners, levels }
    }

    pub(super) fn column(&self, cx: i32, cz: i32) -> &[Corner] {
        let start = (cz * CELLS + cx) as usize * self.levels;
        &self.corners[start..start + self.levels]
    }
}

#[derive(Clone, Copy, Default)]
pub struct Corner {
    pub(crate) main: f32,
    pub(crate) noodle: f32,
    pub(crate) noodle_thickness: f32,
    pub(crate) noodle_ridge_a: f32,
    pub(crate) noodle_ridge_b: f32,
    pub(crate) vein_toggle: f32,
    pub(crate) vein_a: f32,
    pub(crate) vein_b: f32,
}

#[derive(Clone)]
pub struct NoiseChunk {
    corners: Vec<Corner>,
    size_y: i32,
}

impl NoiseChunk {
    pub fn assemble([own, east, south, south_east]: [&CornerColumns; 4]) -> Self {
        let levels = own.levels;
        let mut corners = Vec::with_capacity((CORNER_SPAN * CORNER_SPAN) as usize * levels);
        for cz in 0..CORNER_SPAN {
            for cx in 0..CORNER_SPAN {
                let column = match (cx == CELLS, cz == CELLS) {
                    (false, false) => own.column(cx, cz),
                    (true, false) => east.column(0, cz),
                    (false, true) => south.column(cx, 0),
                    (true, true) => south_east.column(0, 0),
                };
                corners.extend_from_slice(column);
            }
        }
        Self { corners, size_y: levels as i32 }
    }

    pub(crate) fn corner(&self, cx: i32, cy: i32, cz: i32) -> &Corner {
        &self.corners[((cz * 5 + cx) * self.size_y + cy) as usize]
    }
}
