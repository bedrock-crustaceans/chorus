use crate::shared::rng::legacy::Legacy;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::rng::random::RandomSource;
use crate::shared::structure::template::PaletteEntry;

const DEFAULT_STATE_VALUES: [&str; 5] = ["0", "false", "bottom", "none", "y"];

#[derive(Debug, Clone, PartialEq)]
pub enum Matcher {
    Always,
    Block(String),
    RandomBlock(String, f32),
    RandomInList(Vec<String>, f32),
    NotInList(Vec<String>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct LinearPos {
    pub min: f32,
    pub max: f32,
    pub min_d: i32,
    pub max_d: i32,
    pub axis: usize,
}

#[derive(Debug, Clone)]
pub struct Processor {
    pub input: Matcher,
    pub location: Matcher,
    pub position: Option<LinearPos>,
    pub output: Option<u16>,
}

impl LinearPos {
    fn test(&self, (x, y, z): (i32, i32, i32), draw: &mut dyn FnMut() -> f32) -> bool {
        let distance = [x, y, z][self.axis].abs();
        let span = (self.max_d - self.min_d) as f32;
        let t = (distance - self.min_d) as f32 / span;
        let chance = if t < 0.0 {
            self.min
        } else if t > 1.0 {
            self.max
        } else {
            self.min + (self.max - self.min) * t
        };
        draw() <= chance
    }
}

pub struct Sample<'a> {
    pub name: &'a str,
    pub default_state: bool,
}

pub fn is_default_state(states: &str) -> bool {
    states.split(',').filter_map(|pair| pair.split_once('=')).all(|(_, value)| DEFAULT_STATE_VALUES.contains(&value))
}

fn chance_applies(chance: f32, draw: impl FnOnce() -> bool) -> bool {
    chance > 0.0 && (chance >= 1.0 || draw())
}

impl Matcher {
    fn test(&self, subject: &Sample, draw: &mut dyn FnMut() -> f32) -> bool {
        match self {
            Matcher::Always => true,
            Matcher::Block(wanted) => subject.name == wanted,
            Matcher::RandomBlock(wanted, chance) => subject.name == wanted && chance_applies(*chance, || draw() < *chance),
            Matcher::RandomInList(names, chance) => names.iter().any(|candidate| candidate == subject.name) && subject.default_state && chance_applies(*chance, || draw() <= *chance),
            Matcher::NotInList(names) => !names.iter().any(|candidate| candidate == subject.name),
        }
    }
}

pub struct Palette<'a> {
    pub entries: &'a [PaletteEntry],
    pub default_states: &'a [bool],
}

impl Palette<'_> {
    fn subject(&self, index: u16) -> Sample<'_> {
        Sample {
            name: &self.entries[index as usize].name,
            default_state: self.default_states[index as usize],
        }
    }
}

pub fn process(processors: &[Processor], mut block: u16, random: &mut MersenneTwister, palette: &Palette, world: &Sample, from_pivot: (i32, i32, i32)) -> u16 {
    let mut draw = || random.next_float();
    for rule in processors {
        let current = palette.subject(block);
        if rule.input.test(&current, &mut draw)
            && rule.location.test(world, &mut draw)
            && rule.position.as_ref().is_none_or(|position| position.test(from_pivot, &mut draw))
            && let Some(output) = rule.output
        {
            block = output;
        }
    }
    block
}

pub fn process_first(processors: &[Processor], block: u16, seed: i64, palette: &Palette, world: &Sample) -> u16 {
    let mut random = Legacy::new(seed);
    let mut draw = || random.next_float();
    let current = palette.subject(block);
    for rule in processors {
        if rule.input.test(&current, &mut draw) && rule.location.test(world, &mut draw) {
            return rule.output.unwrap_or(block);
        }
    }
    block
}
