pub struct MathUtils;

impl MathUtils {

    pub fn clamp<T: PartialOrd>(value: T, min: T, max: T) -> T {
        if value < min {
            min
        } else if value > max {
            max
        } else {
            value
        }
    }

    pub fn lerp(start: f64, end: f64, t: f64) -> f64 {
        start + (end - start) * t
    }

    pub fn map(value: f64, from_min: f64, from_max: f64, to_min: f64, to_max: f64) -> f64 {
        (value - from_min) / (from_max - from_min) * (to_max - to_min) + to_min
    }

    pub fn normalize(value: f64, min: f64, max: f64) -> f64 {
        (value - min) / (max - min)
    }

    pub fn distance_2d(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
        ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt()
    }

    pub fn distance_3d(
        x1: f64,
        y1: f64,
        z1: f64,
        x2: f64,
        y2: f64,
        z2: f64,
    ) -> f64 {
        ((x2 - x1).powi(2)
            + (y2 - y1).powi(2)
            + (z2 - z1).powi(2))
            .sqrt()
    }

    pub fn deg_to_rad(deg: f64) -> f64 {
        deg.to_radians()
    }

    pub fn rad_to_deg(rad: f64) -> f64 {
        rad.to_degrees()
    }

    pub fn floor(value: f64) -> i64 {
        value.floor() as i64
    }

    pub fn ceil(value: f64) -> i64 {
        value.ceil() as i64
    }

    pub fn round(value: f64) -> i64 {
        value.round() as i64
    }

    pub fn between<T: PartialOrd>(value: T, min: T, max: T) -> bool {
        value >= min && value <= max
    }

    pub fn random_range(min: f64, max: f64) -> f64 {
        rand::random::<f64>() * (max - min) + min
    }

    pub fn random_int(min: i32, max: i32) -> i32 {
        rand::random::<i32>() % (max - min + 1) + min
    }

    pub fn wrap_angle(angle: f64) -> f64 {
        let mut angle = angle % 360.0;
        if angle < 0.0 {
            angle += 360.0;
        }
        angle
    }

    pub fn sqrt(value: f64) -> f64 {
        value.sqrt()
    }

    pub fn abs(value: f64) -> f64 {
        value.abs()
    }
}