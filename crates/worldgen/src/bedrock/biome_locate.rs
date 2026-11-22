use chorus_level::generator::dimension::Generator;
use chorus_level::generator::engine::{Delivery, LocateJob, LocateStep, Located};

const STEP: i32 = 32;

pub(super) struct BiomeSearch {
    from: (i32, i32),
    radius: i32,
    sample: Box<dyn Fn(i32, i32) -> Option<i32> + Send + Sync>,
}

impl BiomeSearch {
    pub(super) fn new(from: (i32, i32), radius: i32, sample: Box<dyn Fn(i32, i32) -> Option<i32> + Send + Sync>) -> Self {
        Self {
            from,
            radius: (radius / STEP).max(1),
            sample,
        }
    }
}

impl<G: Generator> LocateJob<G> for BiomeSearch {
    fn step(&mut self, _arrived: Vec<Delivery>) -> LocateStep<G> {
        for ring in 1..=self.radius {
            for dx in -ring..=ring {
                for dz in -ring..=ring {
                    if dx.abs() != ring && dz.abs() != ring {
                        continue;
                    }
                    let x = self.from.0 + dx * STEP;
                    let z = self.from.1 + dz * STEP;
                    if let Some(y) = (self.sample)(x, z) {
                        return LocateStep::Done(Some(Located {
                            x: x.div_euclid(16),
                            z: z.div_euclid(16),
                            y: Some(y),
                            position: Some((x, z)),
                        }));
                    }
                }
            }
        }
        LocateStep::Done(None)
    }
}
