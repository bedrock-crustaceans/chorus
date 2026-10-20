use crate::debug::hud::sample::{Sample, fraction};
use crate::format::{decimal, megabytes, thousands, tps, usage};

const PERCENT: f32 = 100.0;
const PROGRESS_EPSILON: f32 = 0.005;
const USAGE_WARNING: f32 = 0.5;
const USAGE_CRITICAL: f32 = 0.8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum BossColor {
    Pink = 0,
    Blue = 1,
    Red = 2,
    Green = 3,
    Yellow = 4,
    Purple = 5,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Generation,
    Sending,
    Performance,
    Memory,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BossFrame {
    pub title: String,
    pub progress: f32,
    pub color: BossColor,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameChanges {
    pub title: bool,
    pub progress: bool,
    pub color: bool,
}

impl FrameChanges {
    pub fn any(self) -> bool {
        self.title || self.progress || self.color
    }
}

impl BossFrame {
    pub fn changes_from(&self, previous: &BossFrame) -> FrameChanges {
        FrameChanges {
            title: self.title != previous.title,
            progress: (self.progress - previous.progress).abs() >= PROGRESS_EPSILON,
            color: self.color != previous.color,
        }
    }
}

impl Page {
    pub const ALL: [Page; 4] = [Self::Generation, Self::Sending, Self::Performance, Self::Memory];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Generation => "generation",
            Self::Sending => "sending",
            Self::Performance => "performance",
            Self::Memory => "memory",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|page| page.name().eq_ignore_ascii_case(name))
    }

    pub fn parse_list(text: &str) -> Result<Vec<Self>, String> {
        let mut pages = Vec::new();
        for name in text.split(|character: char| character == ',' || character.is_whitespace()).filter(|name| !name.is_empty()) {
            let page = Self::parse(name).ok_or_else(|| format!("\"{name}\" is not a page, expected one of: {}", Self::names().join(", ")))?;
            if !pages.contains(&page) {
                pages.push(page);
            }
        }
        if pages.is_empty() {
            return Err("give at least one page".to_owned());
        }
        Ok(pages)
    }

    pub fn names() -> Vec<&'static str> {
        Self::ALL.iter().map(|page| page.name()).collect()
    }

    pub fn frame(self, sample: &Sample) -> BossFrame {
        let (server, view) = (&sample.server, &sample.player.view);
        match self {
            Self::Generation => {
                let progress = fraction(view.generated, view.wanted);
                BossFrame {
                    title: format!(
                        "§lChunks generated§r {}/{} §7({}%)",
                        thousands(view.generated),
                        thousands(view.wanted),
                        decimal((progress * PERCENT) as f64, 0)
                    ),
                    progress,
                    color: completion_color(progress, BossColor::Blue),
                }
            }
            Self::Sending => {
                let progress = fraction(view.sent, view.wanted);
                BossFrame {
                    title: format!(
                        "§lChunks sent§r {}/{} §7({}%) §8queued §7{}",
                        thousands(view.sent),
                        thousands(view.wanted),
                        decimal((progress * PERCENT) as f64, 0),
                        thousands(view.queued)
                    ),
                    progress,
                    color: completion_color(progress, BossColor::Purple),
                }
            }
            Self::Performance => {
                let progress = (server.tick_usage as f32 / PERCENT).clamp(0.0, 1.0);
                BossFrame {
                    title: format!("§lTPS§r {} §7| tick {} §7| {} ms", tps(server.tps), usage(server.tick_usage), decimal(server.mspt, 1)),
                    progress,
                    color: load_color(progress),
                }
            }
            Self::Memory => {
                let (title, progress) = match server.memory {
                    Some(memory) => (
                        format!("§lMemory§r {} §7of {} virtual", megabytes(memory.resident_bytes), megabytes(memory.virtual_bytes)),
                        fraction(memory.resident_bytes as usize, memory.virtual_bytes as usize),
                    ),
                    None => ("§lMemory§r §7unavailable".to_owned(), 0.0),
                };
                BossFrame {
                    title,
                    progress,
                    color: BossColor::Yellow,
                }
            }
        }
    }
}

fn completion_color(progress: f32, working: BossColor) -> BossColor {
    if progress >= 1.0 { BossColor::Green } else { working }
}

fn load_color(progress: f32) -> BossColor {
    if progress < USAGE_WARNING {
        BossColor::Green
    } else if progress < USAGE_CRITICAL {
        BossColor::Yellow
    } else {
        BossColor::Red
    }
}

pub fn rotation_index(elapsed_ticks: i64, period_ticks: u32, count: usize) -> usize {
    if count == 0 {
        return 0;
    }
    let period = i64::from(period_ticks.max(1));
    (elapsed_ticks.max(0) / period) as usize % count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debug::hud::sample::{Memory, PlayerSample, ServerSample};
    use crate::world::chunks::ViewStats;

    fn sample(generated: usize, sent: usize, wanted: usize) -> Sample {
        Sample {
            server: ServerSample {
                tps: 20.0,
                mspt: 10.0,
                tick_usage: 20.0,
                memory: Some(Memory {
                    resident_bytes: 512 * 1024 * 1024,
                    virtual_bytes: 1024 * 1024 * 1024,
                }),
                players: 1,
            },
            player: PlayerSample {
                view: ViewStats {
                    wanted,
                    generated,
                    sent,
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    #[test]
    fn generation_progress_is_generated_over_wanted() {
        let frame = Page::Generation.frame(&sample(750, 0, 1000));
        assert_eq!(frame.progress, 0.75);
        assert_eq!(frame.color, BossColor::Blue);
        assert!(frame.title.contains("750/1,000"));
    }

    #[test]
    fn sending_progress_is_sent_over_wanted_and_turns_green_when_done() {
        let working = Page::Sending.frame(&sample(1000, 250, 1000));
        assert_eq!(working.progress, 0.25);
        assert_eq!(working.color, BossColor::Purple);
        let done = Page::Sending.frame(&sample(1000, 1000, 1000));
        assert_eq!(done.progress, 1.0);
        assert_eq!(done.color, BossColor::Green);
    }

    #[test]
    fn empty_view_counts_as_complete() {
        assert_eq!(Page::Generation.frame(&sample(0, 0, 0)).progress, 1.0);
    }

    #[test]
    fn performance_progress_follows_tick_usage_and_color_follows_load() {
        let mut busy = sample(0, 0, 0);
        busy.server.tick_usage = 90.0;
        let frame = Page::Performance.frame(&busy);
        assert_eq!(frame.progress, 0.9);
        assert_eq!(frame.color, BossColor::Red);
        busy.server.tick_usage = 250.0;
        assert_eq!(Page::Performance.frame(&busy).progress, 1.0);
    }

    #[test]
    fn memory_progress_is_resident_share_of_virtual() {
        assert_eq!(Page::Memory.frame(&sample(0, 0, 0)).progress, 0.5);
        let mut unavailable = sample(0, 0, 0);
        unavailable.server.memory = None;
        assert_eq!(Page::Memory.frame(&unavailable).progress, 0.0);
    }

    #[test]
    fn rotation_advances_each_period_and_wraps() {
        assert_eq!(rotation_index(0, 100, 4), 0);
        assert_eq!(rotation_index(99, 100, 4), 0);
        assert_eq!(rotation_index(100, 100, 4), 1);
        assert_eq!(rotation_index(399, 100, 4), 3);
        assert_eq!(rotation_index(400, 100, 4), 0);
    }

    #[test]
    fn rotation_tolerates_degenerate_input() {
        assert_eq!(rotation_index(50, 0, 3), 2);
        assert_eq!(rotation_index(-10, 100, 3), 0);
        assert_eq!(rotation_index(10, 100, 0), 0);
    }

    #[test]
    fn frame_changes_ignore_tiny_progress_noise() {
        let base = Page::Generation.frame(&sample(500, 0, 1000));
        let noise = BossFrame {
            progress: base.progress + 0.001,
            ..base.clone()
        };
        assert!(!noise.changes_from(&base).any());
        let moved = BossFrame {
            progress: base.progress + 0.1,
            ..base.clone()
        };
        assert_eq!(moved.changes_from(&base), FrameChanges { progress: true, ..Default::default() });
    }

    #[test]
    fn frame_changes_report_title_and_color() {
        let base = Page::Generation.frame(&sample(500, 0, 1000));
        let other = Page::Performance.frame(&sample(500, 0, 1000));
        let changes = other.changes_from(&base);
        assert!(changes.title && changes.color);
    }

    #[test]
    fn parses_page_lists() {
        assert_eq!(Page::parse_list("memory,generation").unwrap(), vec![Page::Memory, Page::Generation]);
        assert!(Page::parse_list("nothing").is_err());
        assert!(Page::parse_list("").is_err());
    }
}
