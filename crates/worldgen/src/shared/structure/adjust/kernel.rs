use std::sync::OnceLock;

const SPAN: usize = 24;
const HALF_SPAN: i32 = 12;
const FALLOFF_SCALE: f64 = -0.0625;
const BURY_SCALE: f64 = 0.166_666_666_666_666_66;
const HALF: f64 = 0.5;
const INVERSE_SQRT_MAGIC: u64 = 0x5fe6_eb50_c7b5_37a9;

fn index(a: i32, b: i32, c: i32) -> Option<usize> {
    let inside = |value: i32| (-HALF_SPAN..HALF_SPAN).contains(&value);
    (inside(a) && inside(b) && inside(c)).then(|| (a + HALF_SPAN) as usize * SPAN * SPAN + (b + HALF_SPAN) as usize * SPAN + (c + HALF_SPAN) as usize)
}

fn falloff(dx: i32, dy: i32, dz: i32) -> f32 {
    static TABLE: OnceLock<Vec<f32>> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let mut table = vec![0.0; SPAN * SPAN * SPAN];
        for a in -HALF_SPAN..HALF_SPAN {
            for b in -HALF_SPAN..HALF_SPAN {
                for c in -HALF_SPAN..HALF_SPAN {
                    let height = c as f64 + HALF;
                    let squared = height * height + (b * b) as f64 + (a * a) as f64;
                    table[index(a, b, c).expect("kernel cell")] = std::f64::consts::E.powf(squared * FALLOFF_SCALE) as f32;
                }
            }
        }
        table
    });
    table[index(dz, dx, dy).expect("kernel cell")]
}

fn fast_inverse_sqrt(value: f64) -> f64 {
    let estimate = f64::from_bits(INVERSE_SQRT_MAGIC.wrapping_sub(((value.to_bits() as i64) >> 1) as u64));
    (1.5 - value * HALF * estimate * estimate) * estimate
}

pub fn beard(dx: i32, dy: i32, dz: i32, to_ground: i32) -> f32 {
    if index(dx, dy, dz).is_none() {
        return 0.0;
    }
    let height = to_ground as f64 + HALF;
    let distance_squared = (height * height + (dx * dx) as f64 + (dz * dz) as f64) * HALF;
    (fast_inverse_sqrt(distance_squared) * -height * HALF * falloff(dx, dy, dz) as f64) as f32
}

pub fn bury(a: i32, b: i32, c: i32) -> f32 {
    static TABLE: OnceLock<Vec<f32>> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let mut table = vec![0.0; SPAN * SPAN * SPAN];
        for a in -HALF_SPAN..HALF_SPAN {
            for b in -HALF_SPAN..HALF_SPAN {
                for c in -HALF_SPAN..HALF_SPAN {
                    let halved = |value: i32| (value as f64 * HALF) * (value as f64 * HALF);
                    let distance = (halved(b) + halved(c) + halved(a)).sqrt() * BURY_SCALE;
                    table[index(a, b, c).expect("kernel cell")] = (1.0 - distance).clamp(0.0, 1.0) as f32;
                }
            }
        }
        table
    });
    index(a, b, c).map_or(0.0, |cell| table[cell])
}
