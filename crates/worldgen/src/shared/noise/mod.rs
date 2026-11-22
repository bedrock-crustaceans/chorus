pub mod legacy;

mod helpers;
mod normal;
mod perlin;
mod simplex;
mod stack;

pub use helpers::lerp;
pub use normal::NormalNoise;
pub use simplex::SimplexNoise;
pub use stack::NoiseStack;
