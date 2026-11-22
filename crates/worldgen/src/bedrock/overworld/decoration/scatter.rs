use super::expression::{Expression, Host, Variables};
use crate::shared::rng::mersenne::MersenneTwister;
use glam::IVec3;
use serde_json::Value;
use std::convert::Infallible;

const ITERATIONS: &str = "iterations";
const CHANCE: &str = "scatter_chance";
const NUMERATOR: &str = "numerator";
const DENOMINATOR: &str = "denominator";
const EVAL_ORDER: &str = "coordinate_eval_order";
const DISTRIBUTION: &str = "distribution";
const EXTENT: &str = "extent";
const STEP_SIZE: &str = "step_size";
const GRID_OFFSET: &str = "grid_offset";
const FULL_PERCENT: f32 = 100.0;
const DEFAULT_ITERATIONS: f32 = 0.0;
const NEGATIVE_ITERATIONS_REPLACEMENT: f32 = 1.0;
const DEFAULT_EVAL_ORDER: [usize; 3] = [0, 2, 1];
const AXIS_NAMES: [&str; 3] = ["x", "y", "z"];

thread_local! {
    static LEFT_ORIGIN: std::cell::Cell<Option<IVec3>> = const { std::cell::Cell::new(None) };
    static WORLD_BOUND: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub fn reset_origin() {
    LEFT_ORIGIN.set(None);
}

pub fn reset_world() {
    WORLD_BOUND.set(false);
}

pub fn bind_world() -> bool {
    WORLD_BOUND.replace(true)
}

pub fn leave_origin(origin: IVec3) {
    LEFT_ORIGIN.set(Some(origin));
}

pub fn current_origin(corner: IVec3) -> [f32; 3] {
    LEFT_ORIGIN.get().unwrap_or(corner).as_vec3().to_array()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Distribution {
    SingleValued,
    Uniform,
    Gaussian,
    InverseGaussian,
    FixedGrid,
    JitteredGrid,
    Triangle,
}

impl Distribution {
    fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "single_valued" => Self::SingleValued,
            "uniform" => Self::Uniform,
            "gaussian" => Self::Gaussian,
            "inverse_gaussian" => Self::InverseGaussian,
            "fixed_grid" => Self::FixedGrid,
            "jittered_grid" => Self::JitteredGrid,
            "triangle" => Self::Triangle,
            _ => return None,
        })
    }
}

#[derive(Debug)]
pub enum Chance {
    Always,
    Fraction { numerator: i32, denominator: i32 },
    Percent(Expression),
}

#[derive(Debug)]
pub enum Coordinate {
    Fixed(Expression),
    Sampled {
        distribution: Distribution,
        min: Expression,
        max: Expression,
        step: Expression,
        grid_offset: Expression,
    },
}

#[derive(Debug)]
pub struct Scatter {
    chance: Chance,
    iterations: Expression,
    order: [usize; 3],
    coordinates: [Coordinate; 3],
}

fn expression(value: &Value) -> Expression {
    match value {
        Value::Number(number) => Expression::Number(number.as_f64().unwrap_or_default() as f32),
        Value::String(text) => Expression::parse(text),
        other => Expression::Unsupported(other.to_string()),
    }
}

fn eval_order(name: Option<&str>) -> [usize; 3] {
    match name {
        Some("xzy") => [0, 2, 1],
        Some("yxz") => [1, 0, 2],
        Some("yzx") => [1, 2, 0],
        Some("zxy") => [2, 0, 1],
        Some("zyx") => [2, 1, 0],
        Some("xyz") => [0, 1, 2],
        _ => DEFAULT_EVAL_ORDER,
    }
}

fn coordinate(value: &Value) -> Coordinate {
    let Some(map) = value.as_object() else { return Coordinate::Fixed(expression(value)) };
    let distribution = map.get(DISTRIBUTION).and_then(Value::as_str).and_then(Distribution::parse);
    let extent = map.get(EXTENT).and_then(Value::as_array).filter(|extent| extent.len() == 2);
    match (distribution, extent) {
        (Some(distribution), Some(extent)) => Coordinate::Sampled {
            distribution,
            min: expression(&extent[0]),
            max: expression(&extent[1]),
            step: map.get(STEP_SIZE).map_or(Expression::Number(1.0), expression),
            grid_offset: map.get(GRID_OFFSET).map_or(Expression::Number(0.0), expression),
        },
        _ => Coordinate::Fixed(Expression::Unsupported(value.to_string())),
    }
}

impl Scatter {
    pub fn parse(distribution: &Value) -> Self {
        let distribution = distribution.get(DISTRIBUTION).filter(|value| value.is_object()).unwrap_or(distribution);
        let chance = match distribution.get(CHANCE) {
            Some(Value::Object(map)) => {
                let numerator = map.get(NUMERATOR).and_then(Value::as_i64).unwrap_or_default() as i32;
                let denominator = map.get(DENOMINATOR).and_then(Value::as_i64).unwrap_or_default() as i32;
                Chance::Fraction {
                    numerator,
                    denominator: if denominator > numerator { denominator } else { 1 },
                }
            }
            Some(Value::Number(number)) if number.as_f64().is_some_and(|percent| percent <= 0.0 || percent > FULL_PERCENT as f64) => Chance::Always,
            Some(other) => Chance::Percent(expression(other)),
            None => Chance::Always,
        };
        let iterations = match distribution.get(ITERATIONS).map(expression) {
            None => Expression::Number(DEFAULT_ITERATIONS),
            Some(Expression::Number(count)) if count < 0.0 => Expression::Number(NEGATIVE_ITERATIONS_REPLACEMENT),
            Some(other) => other,
        };
        Self {
            chance,
            iterations,
            order: eval_order(distribution.get(EVAL_ORDER).and_then(Value::as_str)),
            coordinates: AXIS_NAMES.map(|axis| coordinate(&distribution[axis])),
        }
    }

    pub fn is_supported(&self) -> bool {
        let mut found = std::collections::BTreeMap::new();
        self.collect_unsupported(&mut found);
        found.is_empty()
    }

    pub fn uses_random(&self) -> bool {
        let mut expressions = vec![&self.iterations];
        if let Chance::Percent(chance) = &self.chance {
            expressions.push(chance);
        }
        for coordinate in &self.coordinates {
            match coordinate {
                Coordinate::Fixed(value) => expressions.push(value),
                Coordinate::Sampled { min, max, step, grid_offset, .. } => expressions.extend([min, max, step, grid_offset]),
            }
        }
        expressions.into_iter().any(Expression::uses_random)
    }

    pub fn is_inert(&self) -> bool {
        self.iterations.constant().is_some_and(|iterations| iterations.round() <= 0.0)
    }

    pub fn collect_unsupported(&self, into: &mut std::collections::BTreeMap<String, usize>) {
        if let Chance::Percent(chance) = &self.chance {
            chance.collect_unsupported(into);
        }
        self.iterations.collect_unsupported(into);
        for coordinate in &self.coordinates {
            match coordinate {
                Coordinate::Fixed(value) => value.collect_unsupported(into),
                Coordinate::Sampled { min, max, step, grid_offset, .. } => [min, max, step, grid_offset].iter().for_each(|value| value.collect_unsupported(into)),
            }
        }
    }

    pub fn run<W: Host>(&self, draws: &mut Draws, variables: Variables, origin: IVec3, world: &mut W, mut visit: impl FnMut(&mut W, &mut MersenneTwister, IVec3)) {
        let _: Result<(), Infallible> = self.try_run(draws, variables, origin, world, |world, random, position| {
            visit(world, random, position);
            Ok(())
        });
    }

    pub fn try_run<W: Host, E>(
        &self,
        draws: &mut Draws,
        mut variables: Variables,
        origin: IVec3,
        world: &mut W,
        mut visit: impl FnMut(&mut W, &mut MersenneTwister, IVec3) -> Result<(), E>,
    ) -> Result<(), E> {
        if !self.passes_chance(draws, &variables, world) {
            return Ok(());
        }
        let iterations = draws.evaluate(&self.iterations, &variables, world).round() as i32;
        let origin = origin.to_array();
        for iteration in (0..iterations).rev() {
            let mut position = [0i32; 3];
            let mut grid_index = iteration;
            for &axis in &self.order {
                position[axis] = origin[axis] + self.coordinates[axis].sample(draws, &variables, world, &mut grid_index);
                variables.world[axis] = position[axis] as f32;
            }
            visit(world, draws.positions, IVec3::from_array(position))?;
        }
        Ok(())
    }

    fn passes_chance(&self, draws: &mut Draws, variables: &Variables, world: &mut dyn Host) -> bool {
        match &self.chance {
            Chance::Always => true,
            Chance::Fraction { numerator, denominator } => *numerator < 1 || *denominator < 1 || numerator == denominator || draws.positions.next_int_bounded(*denominator) < *numerator,
            Chance::Percent(expression) => {
                let percent = draws.evaluate(expression, variables, world);
                if percent >= FULL_PERCENT {
                    true
                } else if percent <= 0.0 {
                    false
                } else {
                    draws.positions.next_float() * FULL_PERCENT < percent
                }
            }
        }
    }
}

pub struct Draws<'a> {
    pub positions: &'a mut MersenneTwister,
    pub molang: Option<&'a mut MersenneTwister>,
}

impl Draws<'_> {
    pub fn evaluate(&mut self, expression: &Expression, variables: &Variables, world: &mut dyn Host) -> f32 {
        let random = match &mut self.molang {
            Some(molang) => &mut **molang,
            None => &mut *self.positions,
        };
        expression.evaluate(variables, world, random)
    }

    fn bound(&mut self, expression: &Expression, variables: &Variables, world: &mut dyn Host) -> i32 {
        self.evaluate(expression, variables, world).round() as i32
    }
}

impl Coordinate {
    fn sample(&self, draws: &mut Draws, variables: &Variables, world: &mut dyn Host, grid_index: &mut i32) -> i32 {
        match self {
            Self::Fixed(value) => draws.bound(value, variables, world),
            Self::Sampled {
                distribution,
                min,
                max,
                step,
                grid_offset,
            } => {
                let (min, max) = (draws.bound(min, variables, world), draws.bound(max, variables, world));
                let (step, grid_offset) = (draws.evaluate(step, variables, world) as i32, draws.evaluate(grid_offset, variables, world) as i32);
                distribution.sample(draws.positions, min, max, step, grid_offset, grid_index)
            }
        }
    }
}

impl Distribution {
    pub fn sample(self, random: &mut MersenneTwister, min: i32, max: i32, step: i32, grid_offset: i32, grid_index: &mut i32) -> i32 {
        let span = max.wrapping_sub(min);
        match self {
            Self::SingleValued => min,
            Self::Uniform => {
                if max > min {
                    min + random.next_int_bounded(span)
                } else {
                    min
                }
            }
            Self::Gaussian => min + span / 2 + bell_offset(random, span),
            Self::InverseGaussian => {
                let offset = bell_offset(random, span);
                match offset {
                    0 if random.next_boolean() => max,
                    0 => min,
                    offset if offset < 0 => min - offset,
                    offset => max - offset,
                }
            }
            Self::FixedGrid => grid_value(min, span, step, grid_offset, grid_index),
            Self::JitteredGrid => {
                let value = grid_value(min, span, step, grid_offset, grid_index);
                if step > 1 { value.wrapping_add(random.next_int_bounded(step)) } else { value }
            }
            Self::Triangle => {
                let first = random.next_int_bounded(span / 2 + 1);
                min + first + random.next_int_bounded(span - span / 2 + 1)
            }
        }
    }
}

fn bell_offset(random: &mut MersenneTwister, span: i32) -> i32 {
    let half = span / 2;
    if span + 1 > 2 && half > 0 {
        let first = (random.next_u32() % half as u32) as i32;
        first - (random.next_u32() % half as u32) as i32
    } else {
        0
    }
}

fn grid_value(min: i32, span: i32, step: i32, grid_offset: i32, grid_index: &mut i32) -> i32 {
    let width = span + 1;
    let value = min + grid_offset + grid_index.wrapping_mul(step);
    *grid_index = if width == 0 { 0 } else { (grid_index.wrapping_mul(step) + grid_offset) / width };
    if width == 0 { value } else { value % width }
}
