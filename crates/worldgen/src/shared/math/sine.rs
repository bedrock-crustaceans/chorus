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
