use super::generator::OverworldGenerator;

const FULL: (i64, i64) = (-10000, 10000);
const INLAND_CONTINENTALNESS: (i64, i64) = (-1099, 10000);
const WEIRDNESS_BELOW: (i64, i64) = (-10000, -1599);
const WEIRDNESS_ABOVE: (i64, i64) = (1599, 10000);
const TARGET_WEIRDNESS: [(i64, i64); 2] = [WEIRDNESS_BELOW, WEIRDNESS_ABOVE];
const QUANTISE: f64 = 10000.0;
const COARSE_STEP: i32 = 512;
const COARSE_LIMIT: i32 = 2048;
const FINE_STEP: i32 = 32;
const FINE_LIMIT: i32 = 512;
const FITNESS_SHIFT: u32 = 22;
const FULL_TURN: f32 = 6.2831855;

fn distance(value: i64, (low, high): (i64, i64)) -> i64 {
    if value < low {
        low - value
    } else if value > high {
        value - high
    } else {
        0
    }
}

fn requantised(value: i64) -> i64 {
    ((value as f64 / QUANTISE) as f32 as f64 * QUANTISE) as i64
}

struct Search<'a> {
    generator: &'a OverworldGenerator,
}

impl Search<'_> {
    fn climate_fitness(&self, x: i32, z: i32) -> i64 {
        let [temperature, humidity, continentalness, erosion, _, weirdness] = self.generator.terrain.climate_column(x, z).point(0).map(requantised);
        TARGET_WEIRDNESS
            .iter()
            .map(|&target| {
                [(temperature, FULL), (humidity, FULL), (continentalness, INLAND_CONTINENTALNESS), (erosion, FULL), (weirdness, target)]
                    .into_iter()
                    .map(|(value, range)| distance(value, range).pow(2))
                    .sum::<i64>()
            })
            .min()
            .unwrap_or(i64::MAX)
    }

    fn score(&self, x: i32, z: i32) -> i64 {
        (self.climate_fitness(x, z) << FITNESS_SHIFT) + (x as i64 * x as i64 + z as i64 * z as i64)
    }

    fn ring(&self, centre: (i32, i32), step: i32, limit: i32, best: &mut ((i32, i32), i64)) {
        let (mut radius, mut angle) = (step, 0.0f32);
        while radius <= limit {
            let reach = radius as f32;
            let x = (angle.sin() * reach) as i32 + centre.0;
            let z = (angle.cos() * reach) as i32 + centre.1;
            let score = self.score(x, z);
            if score < best.1 {
                *best = ((x, z), score);
            }
            let next = step as f32 / reach + angle;
            if next <= FULL_TURN {
                angle = next;
            } else {
                angle = 0.0;
                radius += step;
            }
        }
    }
}

impl OverworldGenerator {
    pub fn spawn_column(&self) -> (i32, i32) {
        let search = Search { generator: self };
        let mut best = ((0, 0), search.climate_fitness(0, 0) << FITNESS_SHIFT);
        search.ring((0, 0), COARSE_STEP, COARSE_LIMIT, &mut best);
        let coarse = best.0;
        search.ring(coarse, FINE_STEP, FINE_LIMIT, &mut best);
        best.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_value_inside_a_range_is_at_no_distance() {
        assert_eq!(distance(0, FULL), 0);
        assert_eq!(distance(-1099, INLAND_CONTINENTALNESS), 0);
        assert_eq!(distance(-1100, INLAND_CONTINENTALNESS), 1);
        assert_eq!(distance(-1200, INLAND_CONTINENTALNESS), 101);
        assert_eq!(distance(-1000, WEIRDNESS_BELOW), 599);
        assert_eq!(distance(1600, WEIRDNESS_ABOVE), 0);
        assert_eq!(distance(0, WEIRDNESS_ABOVE), 1599);
    }
}
