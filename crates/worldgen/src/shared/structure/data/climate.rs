use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub type ClimatePoint = [(f64, f64); 6];

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BiomeClimate(pub BTreeMap<String, Vec<ClimatePoint>>);

impl BiomeClimate {
    pub fn parse(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_climate_parses() {
        let climate = BiomeClimate::parse("{\"ocean\": [[[-1.0, 1.0], [-1.0, 1.0], [-1.2, -1.05], [-1.0, 1.0], [0.0, 0.0], [-1.0, 1.0]]]}").expect("parses");
        assert_eq!(climate.0["ocean"][0][2], (-1.2, -1.05));
    }
}
