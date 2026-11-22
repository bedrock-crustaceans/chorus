use std::sync::OnceLock;

use crate::shared::structure::BoundingBox;

const SPAN: usize = 24;
const HALF_SPAN: i32 = 12;
const BOX_GROWTH: i32 = 2;
const COLUMN_REACH: i32 = 11;
const HEIGHT_REACH_BELOW: i32 = 12;
const HEIGHT_REACH_ABOVE: i32 = 11;
const WEIGHT: f32 = 0.8;
const FALLOFF_SCALE: f32 = 0.0625;
const DISTANCE_SCALE: f32 = 0.5;
const INVERSE_SQRT_MAGIC: u32 = 0x5f37_59df;

#[derive(Debug, Clone, Copy)]
pub struct Embankment {
    bounds: BoundingBox,
    ground: i32,
}

impl Embankment {
    pub fn new(piece: BoundingBox, ground_offset: i32) -> Self {
        let bounds = BoundingBox::new(piece.x0 - BOX_GROWTH, piece.y0, piece.z0 - BOX_GROWTH, piece.x1 + BOX_GROWTH, piece.y1, piece.z1 + BOX_GROWTH);
        Self {
            bounds,
            ground: piece.y0 + ground_offset,
        }
    }

    pub fn reaches_column(&self, x: i32, z: i32) -> bool {
        let (dx, dz) = self.distance_outside(x, z);
        dx <= COLUMN_REACH && dz <= COLUMN_REACH
    }

    pub fn heights(&self) -> (i32, i32) {
        (self.ground - HEIGHT_REACH_BELOW, self.ground + HEIGHT_REACH_ABOVE)
    }

    pub fn contribution(&self, x: i32, y: i32, z: i32) -> f32 {
        let (dx, dz) = self.distance_outside(x, z);
        embankment_kernel(dx, y - self.ground, dz) * WEIGHT
    }

    fn distance_outside(&self, x: i32, z: i32) -> (i32, i32) {
        let bounds = &self.bounds;
        ((bounds.x0 - x).max(x - bounds.x1).max(0), (bounds.z0 - z).max(z - bounds.z1).max(0))
    }
}

fn index(a: i32, b: i32, c: i32) -> Option<usize> {
    let inside = |value: i32| (-HALF_SPAN..HALF_SPAN).contains(&value);
    (inside(a) && inside(b) && inside(c)).then(|| (a + HALF_SPAN) as usize * SPAN * SPAN + (b + HALF_SPAN) as usize * SPAN + (c + HALF_SPAN) as usize)
}

fn fast_inverse_sqrt(value: f32) -> f32 {
    let estimate = f32::from_bits(INVERSE_SQRT_MAGIC.wrapping_sub(((value.to_bits() as i32) >> 1) as u32));
    (1.5 - value * 0.5 * estimate * estimate) * estimate
}

fn kernel() -> &'static [f32] {
    static TABLE: OnceLock<Vec<f32>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = vec![0.0; SPAN * SPAN * SPAN];
        for a in -HALF_SPAN..HALF_SPAN {
            for b in -HALF_SPAN..HALF_SPAN {
                let planar = (b * b + a * a) as f32;
                for c in -HALF_SPAN..HALF_SPAN {
                    let height = (c as f64 + 0.5) as f32;
                    let squared = height * height;
                    let falloff = std::f32::consts::E.powf(-(squared * FALLOFF_SCALE + planar * FALLOFF_SCALE));
                    let distance_squared = squared * DISTANCE_SCALE + planar * DISTANCE_SCALE;
                    table[index(a, b, c).expect("kernel cell")] = -height * fast_inverse_sqrt(distance_squared) * DISTANCE_SCALE * falloff;
                }
            }
        }
        table
    })
}

pub(super) fn embankment_kernel(dx: i32, dy: i32, dz: i32) -> f32 {
    index(dz, dx, dy).map_or(0.0, |cell| kernel()[cell])
}
