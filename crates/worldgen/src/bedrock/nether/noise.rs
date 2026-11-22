use crate::shared::noise::legacy::SimplexNoise;

pub fn adjustment(octaves: &[SimplexNoise], x: i32, z: i32, scale: f32) -> f32 {
    let position = [x as f32, 0.0, z as f32];
    let mut total = 0.0f32;
    let mut octave_scale = 1.0f32;
    for octave in octaves {
        let at = position.map(|value| value * scale * octave_scale);
        total += octave.value3(at[0], at[1], at[2]) / octave_scale;
        octave_scale *= 0.5;
    }
    total * (1.0 / 7.0)
}

pub fn depth(octaves: &[SimplexNoise], x: i32, z: i32, frequency: f32) -> f32 {
    let mut total = 0.0f32;
    let mut scale = 1.0f32;
    for octave in octaves {
        let [x_offset, z_offset] = octave.origin;
        let value = octave.value(x as f32 * scale * frequency + x_offset, z as f32 * scale * frequency + z_offset);
        total += value * (0.55 / scale);
        scale *= 0.5;
    }
    total
}
