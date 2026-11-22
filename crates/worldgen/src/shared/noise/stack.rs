use super::perlin::PerlinNoise;

struct Layer {
    noise: PerlinNoise,
    frequency: f64,
    amplitude: f32,
}

#[derive(Default)]
pub struct NoiseStack {
    layers: Vec<Layer>,
}

impl NoiseStack {
    pub(super) fn add(&mut self, noise: PerlinNoise, frequency: f64, amplitude: f32) {
        self.layers.push(Layer { noise, frequency, amplitude });
    }

    pub fn get(&self, x: f64, y: f64, z: f64) -> f32 {
        let mut value = 0.0f32;
        for layer in &self.layers {
            value += layer.amplitude * layer.noise.get(x * layer.frequency, y * layer.frequency, z * layer.frequency);
        }
        value
    }
}
