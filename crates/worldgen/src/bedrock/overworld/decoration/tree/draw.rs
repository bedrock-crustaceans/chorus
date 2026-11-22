use crate::bedrock::overworld::decoration::ore::Draw;
use serde_json::Value;

const RANGE_MIN: &str = "range_min";
const RANGE_MAX: &str = "range_max";
const SHORT_MIN: &str = "min";
const SHORT_MAX: &str = "max";
const NUMERATOR: &str = "numerator";
const DENOMINATOR: &str = "denominator";
const FULL_PERCENT: f32 = 100.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Range {
    pub min: i32,
    pub max: i32,
}

impl Range {
    pub const fn new(min: i32, max: i32) -> Self {
        Self { min, max }
    }

    pub const fn constant(value: i32) -> Self {
        Self::new(value, value)
    }

    pub fn parse(value: Option<&Value>, default: Self) -> Self {
        match value {
            Some(Value::Number(number)) => Self::constant(number.as_f64().unwrap_or_default() as i32),
            Some(Value::Array(pair)) if pair.len() == 2 => Self::new(pair[0].as_i64().unwrap_or_default() as i32, pair[1].as_i64().unwrap_or_default() as i32),
            Some(Value::Object(map)) => {
                let read = |long: &str, short: &str| map.get(long).or_else(|| map.get(short)).and_then(Value::as_i64);
                match (read(RANGE_MIN, SHORT_MIN), read(RANGE_MAX, SHORT_MAX)) {
                    (Some(min), Some(max)) => Self::new(min as i32, max as i32),
                    _ => default,
                }
            }
            _ => default,
        }
    }

    pub fn draw<D: Draw>(self, random: &mut D) -> i32 {
        if self.min >= self.max - 1 { self.min } else { self.min + random.below(self.max - self.min) }
    }

    pub fn draw_inclusive<D: Draw>(self, random: &mut D) -> i32 {
        if self.min >= self.max { self.min } else { self.min + random.below(self.max - self.min + 1) }
    }

    pub fn draw_inclusive_always<D: Draw>(self, random: &mut D) -> i32 {
        if self.min > self.max { self.min } else { self.min + random.below(self.max - self.min + 1) }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Odds {
    Percent(f32),
    Fraction(u32, u32),
}

impl Odds {
    pub const NEVER: Self = Self::Percent(0.0);

    pub fn parse(value: Option<&Value>) -> Self {
        match value {
            Some(Value::Object(map)) => Self::Fraction(
                map.get(NUMERATOR).and_then(Value::as_u64).unwrap_or_default() as u32,
                map.get(DENOMINATOR).and_then(Value::as_u64).unwrap_or_default() as u32,
            ),
            Some(Value::Number(number)) => Self::Percent(number.as_f64().unwrap_or_default() as f32),
            _ => Self::NEVER,
        }
    }

    pub fn roll<D: Draw>(self, random: &mut D) -> bool {
        match self {
            Self::Fraction(_, 0) => false,
            Self::Fraction(numerator, denominator) if numerator == denominator => true,
            Self::Fraction(numerator, denominator) => random.word() % denominator < numerator,
            Self::Percent(percent) if percent <= 0.0 => false,
            Self::Percent(percent) if percent >= FULL_PERCENT => true,
            Self::Percent(percent) => random.float() * FULL_PERCENT < percent,
        }
    }
}

pub fn pick_weighted<T: Copy, D: Draw>(entries: &[(T, i32)], random: &mut D) -> T {
    let sum: i32 = entries.iter().map(|&(_, weight)| weight).sum();
    let mut left = if sum != 0 { random.below(sum) } else { 0 };
    for &(value, weight) in entries {
        left -= weight;
        if left < 0 {
            return value;
        }
    }
    entries[entries.len() - 1].0
}
