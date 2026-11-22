use serde_json::Value;

use super::blocks::{Output, output_state};
use super::documents::bare;
use super::fault::{Fault, float, items, member, signed, text, unsigned};
use super::processors::{LinearPos, Matcher};

const AXES: [&str; 3] = ["x", "y", "z"];

pub struct Rule {
    pub input: Matcher,
    pub location: Matcher,
    pub position: Option<LinearPos>,
    pub output: Option<Output>,
}

pub enum Step {
    Rules(Vec<Rule>),
    Capped { limit: usize, rules: Vec<Rule> },
}

impl Step {
    pub fn rules(&self) -> &[Rule] {
        match self {
            Self::Rules(rules) | Self::Capped { rules, .. } => rules,
        }
    }
}

pub fn parse_processor_list(document: &Value) -> Result<Vec<Step>, Fault> {
    let mut steps = Vec::new();
    for processor in items(document, "processors")? {
        match text(processor, "processor_type")? {
            "minecraft:rule" => steps.push(Step::Rules(parse_rules(processor)?)),
            "minecraft:capped" => steps.push(Step::Capped {
                limit: unsigned(processor, "limit")? as usize,
                rules: parse_rules(member(processor, "delegate")?)?,
            }),
            "minecraft:protected_blocks" => {}
            other => return Err(Fault::unknown("processor_type", other)),
        }
    }
    Ok(steps)
}

fn parse_rules(parent: &Value) -> Result<Vec<Rule>, Fault> {
    items(parent, "rules")?
        .iter()
        .map(|rule| {
            let output = member(rule, "output_state")?;
            Ok(Rule {
                input: parse_matcher(member(rule, "input_predicate")?)?,
                location: rule.get("location_predicate").map_or(Ok(Matcher::Always), parse_matcher)?,
                position: rule.get("position_predicate").map(parse_position).transpose()?,
                output: output_state(text(output, "name")?, output.get("states").and_then(Value::as_object)),
            })
        })
        .collect()
}

fn parse_matcher(predicate: &Value) -> Result<Matcher, Fault> {
    let block = || text(predicate, "block").map(|name| bare(name).to_string());
    let blocks = || {
        items(predicate, "blocks")?
            .iter()
            .map(|block| block.as_str().map(|name| bare(name).to_string()).ok_or(Fault::wrong_type("blocks", "a list of strings")))
            .collect::<Result<Vec<_>, _>>()
    };
    let probability = || float(predicate, "probability").map(|chance| chance as f32);
    Ok(match text(predicate, "predicate_type")? {
        "minecraft:always_true" => Matcher::Always,
        "minecraft:block_match" => Matcher::Block(block()?),
        "minecraft:random_block_match" => Matcher::RandomBlock(block()?, probability()?),
        "bedrock:always_true_except" => Matcher::NotInList(blocks()?),
        "bedrock:true_if_found" => Matcher::RandomInList(blocks()?, probability()?),
        other => return Err(Fault::unknown("predicate_type", other)),
    })
}

fn parse_position(predicate: &Value) -> Result<LinearPos, Fault> {
    let axis = text(predicate, "axis")?;
    Ok(LinearPos {
        min: float(predicate, "min_chance")? as f32,
        max: float(predicate, "max_chance")? as f32,
        min_d: signed(predicate, "min_dist")? as i32,
        max_d: signed(predicate, "max_dist")? as i32,
        axis: AXES.iter().position(|name| *name == axis).ok_or_else(|| Fault::unknown("axis", axis))?,
    })
}
