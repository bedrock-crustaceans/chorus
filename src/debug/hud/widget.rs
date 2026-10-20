use crate::debug::hud::sample::Sample;
use crate::format::{decimal, megabytes, milliseconds, thousands, tps, usage};
use crate::player::PLAYER_EYE_HEIGHT;

const DEGREES_PER_TURN: f32 = 360.0;
const DEGREES_PER_COMPASS_STEP: f32 = 45.0;
const COMPASS: [&str; 8] = ["S", "SW", "W", "NW", "N", "NE", "E", "SE"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Widget {
    Tps,
    Mspt,
    Usage,
    Memory,
    Players,
    Generated,
    Sent,
    Position,
    Chunk,
    Facing,
    Loaded,
}

impl Widget {
    pub const ALL: [Widget; 11] = [
        Self::Tps,
        Self::Mspt,
        Self::Usage,
        Self::Memory,
        Self::Players,
        Self::Generated,
        Self::Sent,
        Self::Position,
        Self::Chunk,
        Self::Facing,
        Self::Loaded,
    ];

    pub const DEFAULTS: [Widget; 4] = [Self::Tps, Self::Usage, Self::Generated, Self::Sent];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Tps => "tps",
            Self::Mspt => "mspt",
            Self::Usage => "usage",
            Self::Memory => "memory",
            Self::Players => "players",
            Self::Generated => "generated",
            Self::Sent => "sent",
            Self::Position => "position",
            Self::Chunk => "chunk",
            Self::Facing => "facing",
            Self::Loaded => "loaded",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|widget| widget.name().eq_ignore_ascii_case(name))
    }

    pub fn parse_list(text: &str) -> Result<Vec<Self>, String> {
        let mut widgets = Vec::new();
        for name in text.split(|character: char| character == ',' || character.is_whitespace()).filter(|name| !name.is_empty()) {
            let widget = Self::parse(name).ok_or_else(|| format!("\"{name}\" is not a widget, expected one of: {}", Self::names().join(", ")))?;
            if !widgets.contains(&widget) {
                widgets.push(widget);
            }
        }
        if widgets.is_empty() {
            return Err("give at least one widget".to_owned());
        }
        Ok(widgets)
    }

    pub fn names() -> Vec<&'static str> {
        Self::ALL.iter().map(|widget| widget.name()).collect()
    }

    pub fn render(self, sample: &Sample) -> String {
        let (server, player) = (&sample.server, &sample.player);
        match self {
            Self::Tps => format!("§7TPS {}", tps(server.tps)),
            Self::Mspt => format!("§7MSPT §f{}", milliseconds(server.mspt)),
            Self::Usage => format!("§7Tick {}", usage(server.tick_usage)),
            Self::Memory => format!("§7Mem §f{}", server.memory.map_or_else(|| "-".to_owned(), |memory| megabytes(memory.resident_bytes))),
            Self::Players => format!("§7Players §f{}", server.players),
            Self::Generated => format!(
                "§7Gen §f{}§8/§f{} §8r§f{}",
                thousands(player.view.generated),
                thousands(player.view.wanted),
                decimal(player.view.generated_radius as f64, 1)
            ),
            Self::Sent => format!(
                "§7Sent §f{}§8/§f{} §8r§f{}",
                thousands(player.view.sent),
                thousands(player.view.wanted),
                decimal(player.view.sent_radius as f64, 1)
            ),
            Self::Position => {
                let feet = player.position.y - PLAYER_EYE_HEIGHT;
                format!("§7XYZ §f{} {} {}", decimal(player.position.x as f64, 1), decimal(feet as f64, 1), decimal(player.position.z as f64, 1))
            }
            Self::Chunk => format!("§7Chunk §f{} {}", player.chunk().0, player.chunk().1),
            Self::Facing => format!(
                "§7Facing §f{} §8(§7{}§8)",
                compass(player.rotation.y),
                decimal(player.rotation.y.rem_euclid(DEGREES_PER_TURN) as f64, 0)
            ),
            Self::Loaded => format!("§7Loaded §f{}", thousands(player.loaded_chunks)),
        }
    }
}

pub fn compass(yaw: f32) -> &'static str {
    let normalized = yaw.rem_euclid(DEGREES_PER_TURN);
    let step = (normalized / DEGREES_PER_COMPASS_STEP).round() as usize % COMPASS.len();
    COMPASS[step]
}

pub fn render_line(widgets: &[Widget], separator: &str, sample: &Sample) -> String {
    widgets.iter().map(|widget| widget.render(sample)).collect::<Vec<_>>().join(separator)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debug::hud::sample::{Memory, PlayerSample, ServerSample};
    use crate::world::chunks::ViewStats;
    use glam::{Vec2, Vec3};

    fn sample() -> Sample {
        Sample {
            server: ServerSample {
                tps: 20.0,
                mspt: 12.5,
                tick_usage: 25.0,
                memory: Some(Memory {
                    resident_bytes: 256 * 1024 * 1024,
                    virtual_bytes: 1024 * 1024 * 1024,
                }),
                players: 3,
            },
            player: PlayerSample {
                position: Vec3::new(10.5, 64.0 + PLAYER_EYE_HEIGHT, -3.5),
                rotation: Vec2::new(0.0, 90.0),
                view: ViewStats {
                    radius: 24.0,
                    generated_radius: 21.5,
                    sent_radius: 7.25,
                    wanted: 1500,
                    generated: 1200,
                    sent: 300,
                    ..Default::default()
                },
                loaded_chunks: 2048,
            },
        }
    }

    #[test]
    fn parses_names_case_insensitively() {
        assert_eq!(Widget::parse("TPS"), Some(Widget::Tps));
        assert_eq!(Widget::parse("nope"), None);
    }

    #[test]
    fn parse_list_accepts_commas_and_spaces_and_dedupes() {
        assert_eq!(Widget::parse_list("tps, mspt tps").unwrap(), vec![Widget::Tps, Widget::Mspt]);
    }

    #[test]
    fn parse_list_rejects_unknown_and_empty() {
        assert!(Widget::parse_list("tps,bogus").is_err());
        assert!(Widget::parse_list(" , ").is_err());
    }

    #[test]
    fn every_widget_name_round_trips() {
        for widget in Widget::ALL {
            assert_eq!(Widget::parse(widget.name()), Some(widget));
        }
    }

    #[test]
    fn renders_chunk_counts_with_grouping() {
        assert_eq!(Widget::Generated.render(&sample()), "§7Gen §f1,200§8/§f1,500 §8r§f21.5");
        assert_eq!(Widget::Sent.render(&sample()), "§7Sent §f300§8/§f1,500 §8r§f7.2");
    }

    #[test]
    fn renders_position_at_feet() {
        assert_eq!(Widget::Position.render(&sample()), "§7XYZ §f10.5 64 -3.5");
    }

    #[test]
    fn renders_chunk_coordinates() {
        assert_eq!(Widget::Chunk.render(&sample()), "§7Chunk §f0 -1");
    }

    #[test]
    fn memory_without_stats_renders_dash() {
        let mut sample = sample();
        sample.server.memory = None;
        assert_eq!(Widget::Memory.render(&sample), "§7Mem §f-");
    }

    #[test]
    fn compass_matches_bedrock_yaw() {
        assert_eq!(compass(0.0), "S");
        assert_eq!(compass(90.0), "W");
        assert_eq!(compass(180.0), "N");
        assert_eq!(compass(-90.0), "E");
        assert_eq!(compass(359.0), "S");
    }

    #[test]
    fn line_joins_widgets_with_separator() {
        let line = render_line(&[Widget::Players, Widget::Loaded], " | ", &sample());
        assert_eq!(line, "§7Players §f3 | §7Loaded §f2,048");
    }
}
