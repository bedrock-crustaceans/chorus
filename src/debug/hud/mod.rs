use crate::debug::hud::page::{BossFrame, Page, rotation_index};
use crate::debug::hud::sample::{Memory, PlayerSample, Sample, ServerSample};
use crate::debug::hud::widget::{Widget, render_line};
use crate::entity::components::actor_id::ActorId;
use crate::entity::components::transform::Transform;
use crate::level::generator::dimension::Dimension;
use crate::level::level::Level;
use crate::network::session::Session;
use crate::network::session::state::SessionState;
use crate::player::Player;
use crate::player::chunk_view::ChunkView;
use crate::schedule::GameSet;
use crate::server::{ServerMetrics, ServerState};
use crate::utils::process::process_stats;
use crate::world::chunks::ChunkSerializer;
use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use chorus_core::schedule::{TICK_RATE, Tick};

pub mod command;
pub mod packets;
pub mod page;
pub mod sample;
pub mod widget;

pub const DEFAULT_INTERVAL_TICKS: u32 = 10;
pub const DEFAULT_ROTATE_SECONDS: f32 = 5.0;
pub const DEFAULT_SEPARATOR: &str = " §8| ";
const ACTION_BAR_KEEPALIVE_TICKS: i64 = 20;
const MEMORY_SAMPLE_TICKS: u32 = 20;

pub fn seconds_to_ticks(seconds: f32) -> u32 {
    (seconds * TICK_RATE as f32).round().max(1.0) as u32
}

pub fn ticks_to_seconds(ticks: u32) -> f32 {
    ticks as f32 / TICK_RATE as f32
}

fn is_due(last: Option<i64>, now: i64, interval_ticks: u32) -> bool {
    last.is_none_or(|last| now - last >= i64::from(interval_ticks.max(1)))
}

pub struct ActionBar {
    pub enabled: bool,
    pub widgets: Vec<Widget>,
    pub separator: String,
}

impl Default for ActionBar {
    fn default() -> Self {
        Self {
            enabled: false,
            widgets: Widget::DEFAULTS.to_vec(),
            separator: DEFAULT_SEPARATOR.to_owned(),
        }
    }
}

pub struct BossBar {
    pub enabled: bool,
    pub pinned: Option<Page>,
    pub pages: Vec<Page>,
    pub rotate_ticks: u32,
}

impl Default for BossBar {
    fn default() -> Self {
        Self {
            enabled: false,
            pinned: None,
            pages: Page::ALL.to_vec(),
            rotate_ticks: seconds_to_ticks(DEFAULT_ROTATE_SECONDS),
        }
    }
}

impl BossBar {
    pub fn active_page(&self, rotating_for_ticks: i64) -> Option<Page> {
        if !self.enabled {
            return None;
        }
        self.pinned.or_else(|| self.pages.get(rotation_index(rotating_for_ticks, self.rotate_ticks, self.pages.len())).copied())
    }

    fn shows(&self, page: Page) -> bool {
        self.enabled && self.pinned.map_or_else(|| self.pages.contains(&page), |pinned| pinned == page)
    }
}

struct ShownBar {
    frame: BossFrame,
    page: Page,
    dimension_changes: i32,
}

#[derive(Default)]
struct HudState {
    action_checked: Option<i64>,
    action_sent: Option<i64>,
    action_text: String,
    boss_checked: Option<i64>,
    boss: Option<ShownBar>,
    rotation_started: i64,
}

#[derive(Component)]
pub struct DebugHud {
    pub actionbar: ActionBar,
    pub bossbar: BossBar,
    pub interval_ticks: u32,
    state: HudState,
}

impl Default for DebugHud {
    fn default() -> Self {
        Self {
            actionbar: ActionBar::default(),
            bossbar: BossBar::default(),
            interval_ticks: DEFAULT_INTERVAL_TICKS,
            state: HudState::default(),
        }
    }
}

impl DebugHud {
    pub fn enable(&mut self, now: i64) {
        self.actionbar.enabled = true;
        self.enable_bossbar(now);
    }

    pub fn disable(&mut self) {
        self.actionbar.enabled = false;
        self.bossbar.enabled = false;
    }

    pub fn enable_bossbar(&mut self, now: i64) {
        self.bossbar.enabled = true;
        self.restart_rotation(now);
    }

    pub fn restart_rotation(&mut self, now: i64) {
        self.state.rotation_started = now;
    }

    pub fn reset(&mut self) {
        self.actionbar = ActionBar::default();
        self.bossbar = BossBar::default();
        self.interval_ticks = DEFAULT_INTERVAL_TICKS;
        self.state.action_checked = None;
        self.state.boss_checked = None;
    }

    pub fn is_active(&self) -> bool {
        self.actionbar.enabled || self.bossbar.enabled
    }

    fn needs_memory(&self) -> bool {
        self.actionbar.enabled && self.actionbar.widgets.contains(&Widget::Memory) || self.bossbar.shows(Page::Memory)
    }
}

#[derive(Default)]
struct MemoryCache {
    sampled_at: Option<i64>,
    memory: Option<Memory>,
}

impl MemoryCache {
    fn get(&mut self, now: i64) -> Option<Memory> {
        if is_due(self.sampled_at, now, MEMORY_SAMPLE_TICKS) {
            self.sampled_at = Some(now);
            self.memory = process_stats().map(|stats| Memory {
                resident_bytes: stats.resident_bytes,
                virtual_bytes: stats.virtual_bytes,
            });
        }
        self.memory
    }
}

fn update_huds(
    state: Res<ServerState>,
    metrics: Res<ServerMetrics>,
    level: Res<Level>,
    serializer: Res<ChunkSerializer>,
    players: Query<(), With<Player>>,
    mut huds: Query<(&mut DebugHud, &mut Session, &Transform, &ChunkView, &ActorId)>,
    mut memory_cache: Local<MemoryCache>,
) {
    let now = state.tick();
    let mut server_sample = None;

    for (mut hud, mut session, transform, view, actor) in &mut huds {
        let hud = &mut *hud;
        if session.get_state() != SessionState::Play {
            continue;
        }

        if !hud.actionbar.enabled && hud.state.action_sent.take().is_some() {
            hud.state.action_text.clear();
            session.send(packets::action_bar(""));
        }
        let active_page = hud.bossbar.active_page(now - hud.state.rotation_started);
        if active_page.is_none()
            && let Some(shown) = hud.state.boss.take()
        {
            session.send(packets::boss_bar_remove(actor.unique_id, &shown.frame));
        }

        let action_due = hud.actionbar.enabled && is_due(hud.state.action_checked, now, hud.interval_ticks);
        let page_changed = active_page.is_some_and(|page| hud.state.boss.as_ref().is_none_or(|shown| shown.page != page));
        let boss_due = active_page.is_some() && (page_changed || is_due(hud.state.boss_checked, now, hud.interval_ticks));
        if !action_due && !boss_due {
            continue;
        }

        let server = *server_sample.get_or_insert_with(|| ServerSample {
            tps: metrics.tps(),
            mspt: metrics.mspt(),
            tick_usage: metrics.tick_usage(),
            memory: None,
            players: players.iter().count(),
        });
        let dimension = level.dimension(view.dimension);
        let sample = Sample {
            server: ServerSample {
                memory: if hud.needs_memory() { memory_cache.get(now) } else { None },
                ..server
            },
            player: PlayerSample {
                position: transform.position,
                rotation: transform.rotation,
                view: serializer.view_stats(view, dimension),
                loaded_chunks: dimension.map_or(0, Dimension::chunk_count),
            },
        };

        if action_due {
            hud.state.action_checked = Some(now);
            let text = render_line(&hud.actionbar.widgets, &hud.actionbar.separator, &sample);
            let stale = hud.state.action_sent.is_none_or(|sent| now - sent >= ACTION_BAR_KEEPALIVE_TICKS);
            if stale || text != hud.state.action_text {
                session.send(packets::action_bar(&text));
                hud.state.action_sent = Some(now);
                hud.state.action_text = text;
            }
        }

        if let (true, Some(page)) = (boss_due, active_page) {
            hud.state.boss_checked = Some(now);
            let frame = page.frame(&sample);
            match hud.state.boss.take() {
                Some(shown) if shown.dimension_changes == view.dimension_changes => {
                    for packet in packets::boss_bar_updates(actor.unique_id, &frame, frame.changes_from(&shown.frame)) {
                        session.send(packet);
                    }
                }
                _ => session.send(packets::boss_bar_add(actor.unique_id, &frame)),
            }
            hud.state.boss = Some(ShownBar {
                frame,
                page,
                dimension_changes: view.dimension_changes,
            });
        }
    }
}

pub struct DebugHudPlugin;

impl Plugin for DebugHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Tick, update_huds.in_set(GameSet::Broadcast));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_check_is_always_due() {
        assert!(is_due(None, 0, 10));
    }

    #[test]
    fn due_once_interval_has_elapsed() {
        assert!(!is_due(Some(100), 109, 10));
        assert!(is_due(Some(100), 110, 10));
    }

    #[test]
    fn zero_interval_behaves_as_every_tick() {
        assert!(is_due(Some(5), 6, 0));
        assert!(!is_due(Some(5), 5, 0));
    }

    #[test]
    fn seconds_convert_to_ticks_and_back() {
        assert_eq!(seconds_to_ticks(5.0), 100);
        assert_eq!(seconds_to_ticks(0.0), 1);
        assert_eq!(ticks_to_seconds(40), 2.0);
    }

    #[test]
    fn disabled_bossbar_has_no_page() {
        assert_eq!(BossBar::default().active_page(0), None);
    }

    #[test]
    fn rotating_bossbar_cycles_pages_on_schedule() {
        let bossbar = BossBar {
            enabled: true,
            rotate_ticks: 100,
            ..BossBar::default()
        };
        assert_eq!(bossbar.active_page(0), Some(Page::Generation));
        assert_eq!(bossbar.active_page(100), Some(Page::Sending));
        assert_eq!(bossbar.active_page(450), Some(Page::Generation));
    }

    #[test]
    fn pinned_bossbar_ignores_rotation() {
        let bossbar = BossBar {
            enabled: true,
            pinned: Some(Page::Memory),
            ..BossBar::default()
        };
        assert_eq!(bossbar.active_page(0), Some(Page::Memory));
        assert_eq!(bossbar.active_page(10_000), Some(Page::Memory));
    }

    #[test]
    fn enabling_shows_actionbar_and_rotating_bossbar() {
        let mut hud = DebugHud::default();
        assert!(!hud.is_active());
        hud.enable(40);
        assert!(hud.actionbar.enabled && hud.bossbar.enabled && hud.bossbar.pinned.is_none());
        assert_eq!(hud.bossbar.active_page(40 - hud.state.rotation_started), Some(Page::Generation));
    }

    #[test]
    fn reset_restores_defaults_but_keeps_shown_bar_state() {
        let mut hud = DebugHud::default();
        hud.enable(0);
        hud.interval_ticks = 99;
        hud.actionbar.separator = "x".to_owned();
        hud.reset();
        assert!(!hud.is_active());
        assert_eq!(hud.interval_ticks, DEFAULT_INTERVAL_TICKS);
        assert_eq!(hud.actionbar.separator, DEFAULT_SEPARATOR);
    }

    #[test]
    fn memory_is_only_needed_when_shown() {
        let mut hud = DebugHud::default();
        assert!(!hud.needs_memory());
        hud.bossbar.enabled = true;
        assert!(hud.needs_memory());
        hud.bossbar.pinned = Some(Page::Generation);
        assert!(!hud.needs_memory());
        hud.actionbar.enabled = true;
        hud.actionbar.widgets.push(Widget::Memory);
        assert!(hud.needs_memory());
    }
}
