use crate::bedrock::overworld::decoration::placed::Placed;
use crate::shared::chunk::region::Region;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::Probed;
use glam::IVec3;
use serde_json::Value;
use std::sync::Arc;

const SEARCH_AXIS_KEY: &str = "search_axis";
const VOLUME_KEY: &str = "search_volume";
const REQUIRED_KEY: &str = "required_successes";
const AXES: [char; 3] = ['x', 'y', 'z'];

#[derive(Clone, Copy, Debug)]
struct Sweep {
    axis: usize,
    ascending: bool,
    low: i32,
    high: i32,
}

#[derive(Debug)]
pub struct Search {
    feature: Arc<Placed>,
    sweeps: [Sweep; 3],
    required: i32,
}

fn loop_order(axis: &str) -> Option<[(char, bool); 3]> {
    Some(match axis {
        "-x" => [('x', false), ('y', true), ('z', false)],
        "+x" => [('x', true), ('y', true), ('z', true)],
        "-y" => [('y', false), ('z', true), ('x', false)],
        "+y" => [('y', true), ('z', true), ('x', true)],
        "-z" => [('z', false), ('y', true), ('x', true)],
        "+z" => [('z', true), ('y', true), ('x', false)],
        _ => return None,
    })
}

impl Search {
    pub fn parse(body: &'static Value, feature: Arc<Placed>) -> Option<Self> {
        let order = loop_order(body.get(SEARCH_AXIS_KEY)?.as_str()?)?;
        let volume = body.get(VOLUME_KEY)?;
        let bound = |side: &str, axis: usize| volume.get(side)?.get(axis)?.as_i64().map(|value| value as i32);
        let mut sweeps = [Sweep {
            axis: 0,
            ascending: true,
            low: 0,
            high: 0,
        }; 3];
        for (slot, (name, ascending)) in order.into_iter().enumerate() {
            let axis = AXES.iter().position(|&candidate| candidate == name)?;
            sweeps[slot] = Sweep {
                axis,
                ascending,
                low: bound("min", axis)?,
                high: bound("max", axis)?,
            };
        }
        Some(Self {
            feature,
            sweeps,
            required: body.get(REQUIRED_KEY).and_then(Value::as_i64).unwrap_or(1) as i32,
        })
    }

    pub fn place(&self, region: &mut Region, random: &mut MersenneTwister, at: IVec3) -> Probed<Option<IVec3>> {
        let mut successes = 0;
        let [outer, middle, inner] = self.sweeps;
        for a in outer.values() {
            for b in middle.values() {
                for c in inner.values() {
                    let mut pos = at.to_array();
                    pos[outer.axis] += a;
                    pos[middle.axis] += b;
                    pos[inner.axis] += c;
                    if let Some(done) = self.feature.place_at(region, random, IVec3::from_array(pos))? {
                        successes += 1;
                        if successes == self.required {
                            return Ok(Some(done));
                        }
                    }
                }
            }
        }
        Ok(None)
    }
}

#[derive(Debug)]
pub struct Sequence {
    pub features: Vec<Arc<Placed>>,
}

impl Sequence {
    pub fn place(&self, region: &mut Region, random: &mut MersenneTwister, at: IVec3) -> Probed<Option<IVec3>> {
        let mut placed = None;
        for feature in &self.features {
            match feature.place_at(region, random, placed.unwrap_or(at))? {
                Some(next) => placed = Some(next),
                None => return Ok(None),
            }
        }
        Ok(placed)
    }
}

impl Sweep {
    fn values(self) -> Vec<i32> {
        if self.ascending {
            (self.low..=self.high).collect()
        } else {
            (self.low..=self.high).rev().collect()
        }
    }
}
