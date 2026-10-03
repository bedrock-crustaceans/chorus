use super::noise::lerp;

#[derive(Clone, Copy)]
pub enum Coordinate {
    Continents,
    Erosion,
    Weirdness,
    Ridges,
}

#[derive(Clone, Copy)]
pub struct SplineInput {
    pub continents: f32,
    pub erosion: f32,
    pub weirdness: f32,
    pub ridges: f32,
}

impl SplineInput {
    fn get(&self, coordinate: Coordinate) -> f32 {
        match coordinate {
            Coordinate::Continents => self.continents,
            Coordinate::Erosion => self.erosion,
            Coordinate::Weirdness => self.weirdness,
            Coordinate::Ridges => self.ridges,
        }
    }
}

#[derive(Clone)]
pub enum Spline {
    Constant(f32),
    Multipoint {
        coordinate: Coordinate,
        locations: Vec<f32>,
        values: Vec<Spline>,
        derivatives: Vec<f32>,
    },
}

impl From<f32> for Spline {
    fn from(value: f32) -> Self {
        Self::Constant(value)
    }
}

pub struct Builder {
    coordinate: Coordinate,
    locations: Vec<f32>,
    values: Vec<Spline>,
    derivatives: Vec<f32>,
}

impl Builder {
    pub fn point(self, location: f32, value: impl Into<Spline>) -> Self {
        self.point_with_derivative(location, value, 0.0)
    }

    pub fn point_with_derivative(mut self, location: f32, value: impl Into<Spline>, derivative: f32) -> Self {
        self.locations.push(location);
        self.values.push(value.into());
        self.derivatives.push(derivative);
        self
    }

    pub fn build(self) -> Spline {
        Spline::Multipoint {
            coordinate: self.coordinate,
            locations: self.locations,
            values: self.values,
            derivatives: self.derivatives,
        }
    }
}

fn builder(coordinate: Coordinate) -> Builder {
    Builder {
        coordinate,
        locations: Vec::new(),
        values: Vec::new(),
        derivatives: Vec::new(),
    }
}

impl Spline {
    pub fn sample(&self, input: &SplineInput) -> f32 {
        match self {
            Self::Constant(value) => *value,
            Self::Multipoint {
                coordinate,
                locations,
                values,
                derivatives,
            } => {
                let x = input.get(*coordinate);
                let start = locations.partition_point(|&location| location <= x) as i32 - 1;
                let last = locations.len() - 1;
                let extend = |value: f32, index: usize| {
                    let derivative = derivatives[index];
                    if derivative == 0.0 { value } else { value + derivative * (x - locations[index]) }
                };
                if start < 0 {
                    return extend(values[0].sample(input), 0);
                }
                let start = start as usize;
                if start == last {
                    return extend(values[last].sample(input), last);
                }
                let (x1, x2) = (locations[start], locations[start + 1]);
                let t = (x - x1) / (x2 - x1);
                let y1 = values[start].sample(input);
                let y2 = values[start + 1].sample(input);
                let a = derivatives[start] * (x2 - x1) - (y2 - y1);
                let b = -derivatives[start + 1] * (x2 - x1) + (y2 - y1);
                lerp(t, y1, y2) + t * (1.0 - t) * lerp(t, a, b)
            }
        }
    }
}

pub fn peaks_and_valleys(weirdness: f32) -> f32 {
    -((weirdness.abs() - 0.6666667).abs() - 0.33333334) * 3.0
}

pub fn overworld_offset() -> Spline {
    let beach = erosion_offset(-0.15, 0.0, 0.0, 0.1, 0.0, -0.03, false, false);
    let low = erosion_offset(-0.1, 0.03, 0.1, 0.1, 0.01, -0.03, false, false);
    let mid = erosion_offset(-0.1, 0.03, 0.1, 0.7, 0.01, -0.03, true, true);
    let high = erosion_offset(-0.05, 0.03, 0.1, 1.0, 0.01, 0.01, true, true);
    builder(Coordinate::Continents)
        .point(-1.1, 0.044)
        .point(-1.02, -0.2222)
        .point(-0.51, -0.2222)
        .point(-0.44, -0.12)
        .point(-0.18, -0.12)
        .point(-0.16, beach.clone())
        .point(-0.15, beach)
        .point(-0.1, low)
        .point(0.25, mid)
        .point(1.0, high)
        .build()
}

pub fn overworld_factor() -> Spline {
    builder(Coordinate::Continents)
        .point(-0.19, 3.95)
        .point(-0.15, erosion_factor(6.25, true))
        .point(-0.1, erosion_factor(5.47, true))
        .point(0.03, erosion_factor(5.08, true))
        .point(0.06, erosion_factor(4.69, false))
        .build()
}

pub fn overworld_jaggedness() -> Spline {
    builder(Coordinate::Continents)
        .point(-0.11, 0.0)
        .point(0.03, erosion_jaggedness(1.0, 0.5, 0.0, 0.0))
        .point(0.65, erosion_jaggedness(1.0, 1.0, 1.0, 0.0))
        .build()
}

fn erosion_jaggedness(peak_at_erosion_0: f32, peak_at_erosion_1: f32, high_at_erosion_0: f32, high_at_erosion_1: f32) -> Spline {
    let at_erosion_0 = ridge_jaggedness(peak_at_erosion_0, high_at_erosion_0);
    let at_erosion_1 = ridge_jaggedness(peak_at_erosion_1, high_at_erosion_1);
    builder(Coordinate::Erosion)
        .point(-1.0, at_erosion_0)
        .point(-0.78, at_erosion_1.clone())
        .point(-0.5775, at_erosion_1)
        .point(-0.375, 0.0)
        .build()
}

fn ridge_jaggedness(at_peak_ridge: f32, at_high_ridge: f32) -> Spline {
    let high_slice_start = peaks_and_valleys(0.4);
    let high_slice_end = peaks_and_valleys(0.56666666);
    let high_slice_middle = (high_slice_start + high_slice_end) / 2.0;
    let ridge = builder(Coordinate::Ridges).point(high_slice_start, 0.0);
    let ridge = if at_high_ridge > 0.0 {
        ridge.point(high_slice_middle, weirdness_jaggedness(at_high_ridge))
    } else {
        ridge.point(high_slice_middle, 0.0)
    };
    let ridge = if at_peak_ridge > 0.0 {
        ridge.point(1.0, weirdness_jaggedness(at_peak_ridge))
    } else {
        ridge.point(1.0, 0.0)
    };
    ridge.build()
}

fn weirdness_jaggedness(factor: f32) -> Spline {
    builder(Coordinate::Weirdness).point(-0.01, 0.63 * factor).point(0.01, 0.3 * factor).build()
}

fn erosion_factor(base_value: f32, shattered_terrain: bool) -> Spline {
    let base = builder(Coordinate::Weirdness).point(-0.2, 6.3).point(0.2, base_value).build();
    let erosion = builder(Coordinate::Erosion)
        .point(-0.6, base.clone())
        .point(-0.5, builder(Coordinate::Weirdness).point(-0.05, 6.3).point(0.05, 2.67).build())
        .point(-0.35, base.clone())
        .point(-0.25, base.clone())
        .point(-0.1, builder(Coordinate::Weirdness).point(-0.05, 2.67).point(0.05, 6.3).build())
        .point(0.03, base.clone());
    if shattered_terrain {
        let weirdness_shattered = builder(Coordinate::Weirdness).point(0.0, base_value).point(0.1, 0.625).build();
        let ridges_shattered = builder(Coordinate::Ridges).point(-0.9, base_value).point(-0.69, weirdness_shattered).build();
        erosion
            .point(0.35, base_value)
            .point(0.45, ridges_shattered.clone())
            .point(0.55, ridges_shattered)
            .point(0.62, base_value)
            .build()
    } else {
        let extreme_hills = builder(Coordinate::Ridges).point(-0.7, base.clone()).point(-0.15, 1.37).build();
        let peaks_only = builder(Coordinate::Ridges).point(0.45, base).point(0.7, 1.56).build();
        erosion
            .point(0.05, peaks_only.clone())
            .point(0.4, peaks_only)
            .point(0.45, extreme_hills.clone())
            .point(0.55, extreme_hills)
            .point(0.58, base_value)
            .build()
    }
}

fn slope(y1: f32, y2: f32, x1: f32, x2: f32) -> f32 {
    (y2 - y1) / (x2 - x1)
}

fn mountain_continentalness(ridge: f32, modulation: f32, allow_rivers_below: f32) -> f32 {
    let ridge_slope = 1.0 - (1.0 - modulation) * 0.5;
    let ridge_intersect = 0.5 * (1.0 - modulation);
    let adjusted = (ridge + 1.17) * 0.46082947;
    let continentalness = adjusted * ridge_slope - ridge_intersect;
    if ridge < allow_rivers_below { continentalness.max(-0.2222) } else { continentalness.max(0.0) }
}

fn mountain_ridge_zero_point(modulation: f32) -> f32 {
    let ridge_slope = 1.0 - (1.0 - modulation) * 0.5;
    let ridge_intersect = 0.5 * (1.0 - modulation);
    ridge_intersect / (0.46082947 * ridge_slope) - 1.17
}

fn mountain_ridge(modulation: f32, saddle: bool) -> Spline {
    let spline = builder(Coordinate::Ridges);
    let min_c = mountain_continentalness(-1.0, modulation, -0.7);
    let max_c = mountain_continentalness(1.0, modulation, -0.7);
    let zero = mountain_ridge_zero_point(modulation);
    if -0.65 < zero && zero < 1.0 {
        let after_river = mountain_continentalness(-0.65, modulation, -0.7);
        let before_river = mountain_continentalness(-0.75, modulation, -0.7);
        let min_derivative = slope(min_c, before_river, -1.0, -0.75);
        let zero_c = mountain_continentalness(zero, modulation, -0.7);
        let max_derivative = slope(zero_c, max_c, zero, 1.0);
        spline
            .point_with_derivative(-1.0, min_c, min_derivative)
            .point(-0.75, before_river)
            .point(-0.65, after_river)
            .point(zero - 0.01, zero_c)
            .point_with_derivative(zero, zero_c, max_derivative)
            .point_with_derivative(1.0, max_c, max_derivative)
            .build()
    } else {
        let simple = slope(min_c, max_c, -1.0, 1.0);
        let spline = if saddle {
            spline.point(-1.0, 0.2f32.max(min_c)).point_with_derivative(0.0, lerp(0.5, min_c, max_c), simple)
        } else {
            spline.point_with_derivative(-1.0, min_c, simple)
        };
        spline.point_with_derivative(1.0, max_c, simple).build()
    }
}

#[allow(clippy::too_many_arguments)]
fn erosion_offset(low_valley: f32, hill: f32, tall_hill: f32, mountain_factor: f32, plain: f32, swamp: f32, extreme_hills: bool, saddle: bool) -> Spline {
    let very_low = mountain_ridge(lerp(mountain_factor, 0.6, 1.5), saddle);
    let low = mountain_ridge(lerp(mountain_factor, 0.6, 1.0), saddle);
    let mountains = mountain_ridge(mountain_factor, saddle);
    let wide_plateau = ridge_spline(
        low_valley - 0.15,
        0.5 * mountain_factor,
        lerp(0.5, 0.5, 0.5) * mountain_factor,
        0.5 * mountain_factor,
        0.6 * mountain_factor,
        0.5,
    );
    let narrow_plateau = ridge_spline(low_valley, plain * mountain_factor, hill * mountain_factor, 0.5 * mountain_factor, 0.6 * mountain_factor, 0.5);
    let plains = ridge_spline(low_valley, plain, plain, hill, tall_hill, 0.5);
    let plains_far_inland = ridge_spline(low_valley, plain, plain, hill, tall_hill, 0.5);
    let hills = builder(Coordinate::Ridges).point(-1.0, low_valley).point(-0.4, plains.clone()).point(0.0, tall_hill + 0.07).build();
    let swamps = ridge_spline(-0.02, swamp, swamp, hill, tall_hill, 0.0);
    let erosion = builder(Coordinate::Erosion)
        .point(-0.85, very_low)
        .point(-0.7, low)
        .point(-0.4, mountains)
        .point(-0.35, wide_plateau)
        .point(-0.1, narrow_plateau)
        .point(0.2, plains);
    let erosion = if extreme_hills {
        erosion
            .point(0.4, plains_far_inland.clone())
            .point(0.45, hills.clone())
            .point(0.55, hills)
            .point(0.58, plains_far_inland)
    } else {
        erosion
    };
    erosion.point(0.7, swamps).build()
}

fn ridge_spline(valley: f32, low: f32, mid: f32, high: f32, peaks: f32, min_valley_steepness: f32) -> Spline {
    let d1 = (0.5 * (low - valley)).max(min_valley_steepness);
    let d2 = 5.0 * (mid - low);
    builder(Coordinate::Ridges)
        .point_with_derivative(-1.0, valley, d1)
        .point_with_derivative(-0.4, low, d1.min(d2))
        .point_with_derivative(0.0, mid, d2)
        .point_with_derivative(0.4, high, 2.0 * (high - mid))
        .point_with_derivative(1.0, peaks, 0.7 * (peaks - high))
        .build()
}
