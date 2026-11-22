use std::sync::LazyLock;

const TABLE_SIZE: usize = 65536;
const INDEX_MASK: u32 = 0xffff;
const SCALE: f32 = 10430.378;
const COSINE_SHIFT: f32 = 16384.0;

static TABLE: LazyLock<Vec<f32>> = LazyLock::new(|| (0..TABLE_SIZE).map(|index| ((index as f32 / SCALE) as f64).sin() as f32).collect());

fn at(index: f32) -> f32 {
    TABLE[(index as i32 as u32 & INDEX_MASK) as usize]
}

pub fn sin(radians: f32) -> f32 {
    at(radians * SCALE)
}

pub fn cos(radians: f32) -> f32 {
    at(radians * SCALE + COSINE_SHIFT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_is_the_rounded_sine() {
        assert_eq!(sin(0.0), 0.0);
        assert!((sin(std::f32::consts::FRAC_PI_2) - 1.0).abs() < 1e-6);
        assert!((cos(0.0) - 1.0).abs() < 1e-6);
        assert!((sin(-1.0) + 0.841_470_96).abs() < 2e-4);
        assert_eq!(TABLE[7078], ((7078f32 / SCALE) as f64).sin() as f32);
    }
}
