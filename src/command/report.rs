use crate::command::style::CommandStyle;
use crate::format::{Tone, decimal, megabytes_parts, thousands};
use std::time::Duration;

const SECONDS_PER_MINUTE: u64 = 60;
const SECONDS_PER_HOUR: u64 = 60 * SECONDS_PER_MINUTE;
const SECONDS_PER_DAY: u64 = 24 * SECONDS_PER_HOUR;
const MICROS_PER_MILLI: f64 = 1_000.;
const MICROS_PER_SECOND: f64 = 1_000_000.;
const NESTED_TITLE_DEPTH: usize = 2;

#[derive(Clone, Debug)]
struct Segment {
    number: String,
    unit: String,
    spaced: bool,
    divided: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Value {
    segments: Vec<Segment>,
    tone: Option<Tone>,
    note: Option<String>,
}

impl Value {
    pub fn new(number: impl ToString) -> Self {
        Self {
            segments: vec![Segment {
                number: number.to_string(),
                unit: String::new(),
                spaced: false,
                divided: false,
            }],
            ..Self::default()
        }
    }

    pub fn count(value: usize) -> Self {
        Self::new(thousands(value))
    }

    pub fn ratio(value: usize, max: usize) -> Self {
        Self::count(value).suffix(format!("/{}", thousands(max)))
    }

    pub fn bytes(value: u64) -> Self {
        let (amount, unit) = megabytes_parts(value);
        Self::new(amount).unit(unit)
    }

    pub fn percent(value: f64) -> Self {
        Self::new(decimal(value, 1)).suffix("%")
    }

    pub fn duration(value: Duration) -> Self {
        Self {
            segments: duration_segments(value),
            ..Self::default()
        }
    }

    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        if let Some(last) = self.segments.last_mut() {
            last.unit = unit.into();
            last.spaced = true;
        }
        self
    }

    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        if let Some(last) = self.segments.last_mut() {
            last.unit = suffix.into();
            last.spaced = false;
        }
        self
    }

    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = Some(tone);
        self
    }

    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    pub fn and(mut self, other: Value) -> Self {
        let first = self.segments.len();
        self.segments.extend(other.segments);
        if let Some(segment) = self.segments.get_mut(first) {
            segment.divided = true;
        }
        self
    }

    fn render(&self, style: CommandStyle) -> String {
        let mut text = String::new();
        for (index, segment) in self.segments.iter().enumerate() {
            let joint = match (index, segment.divided, style) {
                (0, _, _) => "",
                (_, true, CommandStyle::Chorus) => " §8| ",
                (_, true, CommandStyle::Vanilla) => " | ",
                _ => " ",
            };
            text.push_str(joint);
            text.push_str(&self.segment(segment, style));
        }
        if let Some(note) = &self.note {
            match style {
                CommandStyle::Chorus => text.push_str(&format!(" §8- §7§o{note}§r")),
                CommandStyle::Vanilla => text.push_str(&format!(" ({note})")),
            }
        }
        text
    }

    fn segment(&self, segment: &Segment, style: CommandStyle) -> String {
        let gap = if segment.spaced { " " } else { "" };
        match style {
            CommandStyle::Chorus => {
                let color = self.tone.map_or("§f", Tone::color);
                if segment.unit.is_empty() {
                    format!("{color}{}", segment.number)
                } else {
                    format!("{color}{}§7{gap}{}", segment.number, segment.unit)
                }
            }
            CommandStyle::Vanilla => format!("{}{gap}{}", segment.number, segment.unit),
        }
    }
}

impl From<&str> for Value {
    fn from(text: &str) -> Self {
        Self::new(text)
    }
}

impl From<String> for Value {
    fn from(text: String) -> Self {
        Self::new(text)
    }
}

fn duration_segments(value: Duration) -> Vec<Segment> {
    let segment = |number: String, unit: &str, spaced: bool| Segment {
        number,
        unit: unit.to_owned(),
        spaced,
        divided: false,
    };
    let micros = value.as_micros();
    if micros < MICROS_PER_MILLI as u128 {
        return vec![segment(micros.to_string(), "us", true)];
    }
    if micros < MICROS_PER_SECOND as u128 {
        return vec![segment(decimal(micros as f64 / MICROS_PER_MILLI, 1), "ms", true)];
    }
    let seconds = value.as_secs();
    if seconds < SECONDS_PER_MINUTE {
        return vec![segment(decimal(micros as f64 / MICROS_PER_SECOND, 2), "s", true)];
    }
    let parts = [
        (seconds / SECONDS_PER_DAY, "d"),
        (seconds % SECONDS_PER_DAY / SECONDS_PER_HOUR, "h"),
        (seconds % SECONDS_PER_HOUR / SECONDS_PER_MINUTE, "m"),
        (seconds % SECONDS_PER_MINUTE, "s"),
    ];
    let first = parts.iter().position(|(amount, _)| *amount > 0).unwrap_or(parts.len() - 1);
    let shown = if parts[first].1 == "d" { &parts[first..first + 3] } else { &parts[first..] };
    shown.iter().map(|(amount, unit)| segment(amount.to_string(), unit, false)).collect()
}

pub fn duration_text(value: Duration) -> String {
    duration_segments(value)
        .iter()
        .map(|segment| format!("{}{}{}", segment.number, if segment.spaced { " " } else { "" }, segment.unit))
        .collect::<Vec<_>>()
        .join(" ")
}

pub struct Report {
    style: CommandStyle,
    depth: usize,
    lines: Vec<String>,
}

impl Report {
    pub fn new(style: CommandStyle) -> Self {
        Self { style, depth: 0, lines: Vec::new() }
    }

    pub fn section(&mut self, title: &str, body: impl FnOnce(&mut Self)) {
        let depth = self.depth;
        self.lines.push(match self.style {
            CommandStyle::Chorus => {
                let indent = " ".repeat(depth * 2);
                let color = if depth < NESTED_TITLE_DEPTH { "§b" } else { "§f" };
                let weight = if depth == 0 { "§l" } else { "" };
                format!("{indent}§8[{color}{weight}{title}§r§8]")
            }
            CommandStyle::Vanilla if depth == 0 => format!("§2--- {title} ---"),
            CommandStyle::Vanilla => format!("{}§e{title}", " ".repeat(depth)),
        });
        self.depth += 1;
        body(self);
        self.depth -= 1;
    }

    pub fn row(&mut self, label: &str, value: impl Into<Value>) {
        let depth = self.depth.saturating_sub(1);
        let value = value.into().render(self.style);
        self.lines.push(match self.style {
            CommandStyle::Chorus => format!("{}§8- §f{label}§8: {value}", " ".repeat(depth * 2 + 1)),
            CommandStyle::Vanilla => format!("{}- {label}: {value}", " ".repeat(depth)),
        });
    }

    pub fn hint(&mut self, text: impl AsRef<str>) {
        self.lines.push(match self.style {
            CommandStyle::Chorus => format!(" §7§o{}§r", text.as_ref()),
            CommandStyle::Vanilla => format!("§7{}", text.as_ref()),
        });
    }

    pub fn into_lines(self) -> Vec<String> {
        self.lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chorus() -> Report {
        Report::new(CommandStyle::Chorus)
    }

    #[test]
    fn durations_use_ascii_units_that_the_game_font_can_draw() {
        assert_eq!(duration_text(Duration::from_micros(250)), "250 us");
        assert_eq!(duration_text(Duration::from_micros(1_500)), "1.5 ms");
        assert_eq!(duration_text(Duration::from_millis(2_500)), "2.5 s");
        assert!(duration_text(Duration::from_micros(40)).is_ascii());
    }

    #[test]
    fn long_durations_attach_each_unit_to_its_number() {
        assert_eq!(duration_text(Duration::from_secs(65)), "1m 5s");
        assert_eq!(duration_text(Duration::from_secs(2 * 3_600 + 14 * 60 + 3)), "2h 14m 3s");
        assert_eq!(duration_text(Duration::from_secs(3 * 86_400 + 4 * 3_600 + 5 * 60 + 6)), "3d 4h 5m");
    }

    #[test]
    fn chorus_rows_color_numbers_and_dim_units_and_notes() {
        let mut report = chorus();
        report.section("Tick", |report| {
            report.row("Last", Value::new("12.4").unit("ms").note("25% of 50 ms").tone(Tone::Good));
        });
        assert_eq!(report.into_lines(), ["§8[§b§lTick§r§8]", " §8- §fLast§8: §a12.4§7 ms §8- §7§o25% of 50 ms§r"]);
    }

    #[test]
    fn nested_sections_indent_and_lose_emphasis() {
        let mut report = chorus();
        report.section("Chunks", |report| {
            report.row("Loaded", Value::count(6_125));
            report.section("Overworld", |report| {
                report.row("Radius", Value::new(8).unit("chunks"));
                report.section("Queues", |report| report.row("Sending", Value::count(12)));
            });
        });
        assert_eq!(
            report.into_lines(),
            [
                "§8[§b§lChunks§r§8]",
                " §8- §fLoaded§8: §f6,125",
                "  §8[§bOverworld§r§8]",
                "   §8- §fRadius§8: §f8§7 chunks",
                "    §8[§fQueues§r§8]",
                "     §8- §fSending§8: §f12",
            ]
        );
    }

    #[test]
    fn divided_values_join_with_a_dark_bar() {
        let mut report = chorus();
        report.row("Task", Value::new(12).unit("runs").and(Value::duration(Duration::from_micros(2_100))));
        assert_eq!(report.into_lines(), [" §8- §fTask§8: §f12§7 runs §8| §f2.1§7 ms"]);
    }

    #[test]
    fn ratios_attach_the_maximum_to_the_value() {
        let mut report = chorus();
        report.row("Players", Value::ratio(3, 20));
        assert_eq!(report.into_lines(), [" §8- §fPlayers§8: §f3§7/20"]);
    }

    #[test]
    fn vanilla_reports_have_no_formatting_beyond_titles() {
        let mut report = Report::new(CommandStyle::Vanilla);
        report.section("Status", |report| report.row("TPS", Value::new("20").tone(Tone::Good).note("avg 19.9")));
        assert_eq!(report.into_lines(), ["§2--- Status ---", "- TPS: 20 (avg 19.9)"]);
    }
}
