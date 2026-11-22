use super::column::Column;
use super::data::GradientSettings;
use super::palette::Palette;

const GRADIENT_MIN: f32 = -1.0;
const GRADIENT_MAX: f32 = 1.0;

pub struct Segment {
    pub x: i32,
    pub z: i32,
    pub first: i32,
    pub last: i32,
}

pub fn build(palette: &Palette, column: &mut Column, settings: &GradientSettings, segment: &Segment) {
    for y in (segment.first..segment.last).rev() {
        let block = column.get(y);
        if block == palette.air || palette.is_liquid(block) || settings.non_replaceable.contains(&block) {
            continue;
        }
        let value = settings.noise.value(segment.x as f32, y as f32, segment.z as f32).clamp(GRADIENT_MIN, GRADIENT_MAX);
        if let Some(specifier) = settings
            .specifiers
            .iter()
            .find(|specifier| specifier.low < value && value <= specifier.high && specifier.block != palette.air)
        {
            column.set(y, specifier.block);
        }
    }
}
