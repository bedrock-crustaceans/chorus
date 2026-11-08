use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum NetworkCompression {
    #[default]
    Zlib,
    Snappy,
    None,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
#[serde(untagged)]
pub enum LevelSeed {
    Number(i64),
    Text(String),
}

impl Default for LevelSeed {
    fn default() -> Self {
        Self::Number(0)
    }
}

impl LevelSeed {
    pub fn value(&self) -> i64 {
        self.parse_with(Self::parse_text)
    }

    pub fn parse_with(&self, parse: impl FnOnce(&str) -> i64) -> i64 {
        match self {
            Self::Number(seed) => *seed,
            Self::Text(text) => parse(text),
        }
    }

    fn parse_text(text: &str) -> i64 {
        let text = text.trim();
        if text.is_empty() {
            return rand::random();
        }
        text.parse()
            .unwrap_or_else(|_| text.encode_utf16().fold(0i32, |hash, unit| hash.wrapping_mul(31).wrapping_add(unit as i32)) as i64)
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(untagged)]
pub enum Threads {
    Count(usize),
    Mode(ThreadMode),
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ThreadMode {
    Auto,
}

impl Default for Threads {
    fn default() -> Self {
        Self::Mode(ThreadMode::Auto)
    }
}

impl Threads {
    pub fn count(self) -> usize {
        match self {
            Self::Count(count) => count,
            Self::Mode(ThreadMode::Auto) => std::thread::available_parallelism().map_or(1, |count| count.get()),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Interval(Option<Duration>);

impl Interval {
    pub const DISABLED: Self = Self(None);

    pub const fn every(duration: Duration) -> Self {
        Self(Some(duration))
    }

    pub fn get(self) -> Option<Duration> {
        self.0.filter(|duration| !duration.is_zero())
    }
}

impl Serialize for Interval {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.get() {
            Some(duration) => serializer.serialize_str(&humantime::format_duration(duration).to_string()),
            None => serializer.serialize_bool(false),
        }
    }
}

impl<'de> Deserialize<'de> for Interval {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Toggle(bool),
            Seconds(u64),
            Text(String),
        }
        match Raw::deserialize(deserializer)? {
            Raw::Toggle(false) => Ok(Self::DISABLED),
            Raw::Toggle(true) => Err(serde::de::Error::custom("expected a duration such as \"5m\" or false, not true")),
            Raw::Seconds(seconds) => Ok(Self::every(Duration::from_secs(seconds))),
            Raw::Text(text) => humantime::parse_duration(&text).map(Self::every).map_err(serde::de::Error::custom),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum ChunkSaving {
    #[default]
    Modified,
    All,
}
