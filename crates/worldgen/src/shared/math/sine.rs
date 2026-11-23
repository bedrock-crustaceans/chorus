const TABLE_SIZE: usize = 65536;
const INDEX_MASK: u32 = 0xffff;
const SCALE: f32 = 10430.378;
const COSINE_SHIFT: f32 = 16384.0;
const HALF_PI: f64 = std::f64::consts::FRAC_PI_2;
const SERIES_TERMS: i32 = 14;

const fn sine_series(x: f64) -> f64 {
    let squared = x * x;
    let mut term = x;
    let mut sum = x;
    let mut k = 1;
    while k < SERIES_TERMS {
        term = -term * squared / ((2 * k) * (2 * k + 1)) as f64;
        sum += term;
        k += 1;
    }
    sum
}

const fn cosine_series(x: f64) -> f64 {
    let squared = x * x;
    let mut term = 1.0;
    let mut sum = 1.0;
    let mut k = 1;
    while k < SERIES_TERMS {
        term = -term * squared / ((2 * k - 1) * (2 * k)) as f64;
        sum += term;
        k += 1;
    }
    sum
}

const fn sine(x: f64) -> f64 {
    let quadrants = x / HALF_PI + 0.5;
    let whole = quadrants as i64;
    let nearest = if quadrants < 0.0 { whole - 1 } else { whole };
    let remainder = x - nearest as f64 * HALF_PI;
    match nearest.rem_euclid(4) {
        0 => sine_series(remainder),
        1 => cosine_series(remainder),
        2 => -sine_series(remainder),
        _ => -cosine_series(remainder),
    }
}

const fn build() -> [f32; TABLE_SIZE] {
    let mut table = [0.0; TABLE_SIZE];
    let mut index = 0;
    while index < TABLE_SIZE {
        table[index] = sine((index as f32 / SCALE) as f64) as f32;
        index += 1;
    }
    table
}

static TABLE: [f32; TABLE_SIZE] = build();

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
    fn the_table_is_the_correctly_rounded_sine_of_every_argument() {
        for (index, entry) in TABLE.iter().enumerate() {
            let expected = ((index as f32 / SCALE) as f64).sin() as f32;
            assert_eq!(entry.to_bits(), expected.to_bits(), "entry {index}");
        }
    }
}
