const TPS_CRITICAL: f64 = 12.;
const TPS_WARNING: f64 = 17.;
const USAGE_WARNING: f64 = 50.;
const USAGE_CRITICAL: f64 = 80.;
const BYTES_PER_KIBIBYTE: f64 = 1024.;
const KIBIBYTES_PER_GIBIBYTE: f64 = 1024.;
const GROUP_SIZE: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Good,
    Warning,
    Critical,
}

impl Tone {
    pub fn color(self) -> &'static str {
        match self {
            Self::Good => "§a",
            Self::Warning => "§6",
            Self::Critical => "§c",
        }
    }
}

pub fn usage_tone(percent: f64) -> Tone {
    if percent < USAGE_WARNING {
        Tone::Good
    } else if percent < USAGE_CRITICAL {
        Tone::Warning
    } else {
        Tone::Critical
    }
}

pub fn tps_tone(value: f64) -> Tone {
    if value < TPS_CRITICAL {
        Tone::Critical
    } else if value < TPS_WARNING {
        Tone::Warning
    } else {
        Tone::Good
    }
}

pub fn usage(percent: f64) -> String {
    format!("{}{}%§f", usage_tone(percent).color(), decimal(percent, 1))
}

pub fn milliseconds(value: f64) -> String {
    format!("{} ms", decimal(value, 2))
}

pub fn tps(value: f64) -> String {
    format!("{}{}§f", tps_tone(value).color(), decimal(value, 1))
}

pub fn decimal(value: f64, places: usize) -> String {
    let formatted = format!("{value:.places$}");
    if formatted.contains('.') {
        formatted.trim_end_matches('0').trim_end_matches('.').to_owned()
    } else {
        formatted
    }
}

pub fn megabytes(bytes: u64) -> String {
    let (amount, unit) = megabytes_parts(bytes);
    format!("{amount} {unit}")
}

pub fn megabytes_parts(bytes: u64) -> (String, &'static str) {
    let megabytes = bytes as f64 / BYTES_PER_KIBIBYTE / BYTES_PER_KIBIBYTE;
    if megabytes >= KIBIBYTES_PER_GIBIBYTE {
        (decimal(megabytes / KIBIBYTES_PER_GIBIBYTE, 2), "GB")
    } else {
        (decimal(megabytes, 1), "MB")
    }
}

pub fn percent(value: usize, total: usize) -> String {
    let percent = if total == 0 { 100. } else { value as f64 * 100. / total as f64 };
    format!("{}%", decimal(percent, 0))
}

pub fn thousands(value: usize) -> String {
    let digits = value.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / GROUP_SIZE);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(GROUP_SIZE) {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_trims_trailing_zeros() {
        assert_eq!(decimal(20.0, 1), "20");
        assert_eq!(decimal(19.50, 2), "19.5");
        assert_eq!(decimal(0.125, 2), "0.12");
    }

    #[test]
    fn thousands_groups_digits() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1_000), "1,000");
        assert_eq!(thousands(1_234_567), "1,234,567");
    }

    #[test]
    fn megabytes_switches_to_gigabytes() {
        assert_eq!(megabytes(512 * 1024 * 1024), "512 MB");
        assert_eq!(megabytes(2048 * 1024 * 1024), "2 GB");
    }

    #[test]
    fn tps_color_follows_thresholds() {
        assert!(tps(20.0).starts_with("§a"));
        assert!(tps(15.0).starts_with("§6"));
        assert!(tps(5.0).starts_with("§c"));
    }

    #[test]
    fn usage_color_follows_thresholds() {
        assert!(usage(10.0).starts_with("§a"));
        assert!(usage(60.0).starts_with("§6"));
        assert!(usage(95.0).starts_with("§c"));
    }
}
