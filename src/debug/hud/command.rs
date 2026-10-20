use crate::command::args::CommandArgs;
use crate::command::context::CommandContext;
use crate::command::parameter::CommandEnum;
use crate::debug::hud::page::Page;
use crate::debug::hud::widget::Widget;
use crate::debug::hud::{DEFAULT_INTERVAL_TICKS, DebugHud, seconds_to_ticks, ticks_to_seconds};
use crate::server::ServerState;
use crate::values;

const MAX_ROTATE_SECONDS: f32 = 600.0;
const MAX_INTERVAL_TICKS: i32 = 1200;

pub const HUD_ACTION: CommandEnum = CommandEnum::new("DebugHudAction", values!["on", "off", "reset", "status"]);
pub const HUD_SWITCH: CommandEnum = CommandEnum::new("DebugHudSwitch", values!["on", "off"]);
pub const HUD_PAGE: CommandEnum = CommandEnum::new("DebugHudPage", values!["generation", "sending", "performance", "memory"]);

pub fn run(context: &mut CommandContext, args: &CommandArgs) -> Result<(), String> {
    let now = context.resource::<ServerState>().tick();
    let mut hud = context.get_mut::<DebugHud>().ok_or("must be sent by player!")?;
    let message = apply(&mut hud, args, now)?;
    context.reply(message);
    Ok(())
}

fn apply(hud: &mut DebugHud, args: &CommandArgs, now: i64) -> Result<String, String> {
    if args.has("interval") {
        return set_interval(hud, args.int("ticks").ok_or("missing ticks")?);
    }
    if args.has("actionbar") {
        return actionbar(hud, args);
    }
    if args.has("bossbar") {
        return bossbar(hud, args, now);
    }
    match args.string("action") {
        Some("on") => {
            hud.enable(now);
            Ok("Debug HUD enabled".to_owned())
        }
        Some("off") => {
            hud.disable();
            Ok("Debug HUD disabled".to_owned())
        }
        Some("reset") => {
            hud.reset();
            Ok("Debug HUD reset to defaults".to_owned())
        }
        _ => Ok(describe(hud)),
    }
}

fn set_interval(hud: &mut DebugHud, ticks: i32) -> Result<String, String> {
    if !(1..=MAX_INTERVAL_TICKS).contains(&ticks) {
        return Err(format!("the interval must be between 1 and {MAX_INTERVAL_TICKS} ticks"));
    }
    hud.interval_ticks = ticks as u32;
    Ok(format!("Debug HUD updates every {ticks} ticks"))
}

fn actionbar(hud: &mut DebugHud, args: &CommandArgs) -> Result<String, String> {
    if args.has("widgets") {
        hud.actionbar.widgets = Widget::parse_list(args.string("list").unwrap_or_default())?;
        return Ok(format!("Action bar shows {}", widget_names(hud)));
    }
    if args.has("separator") {
        let text = args.string("text").unwrap_or_default().replace('&', "§");
        hud.actionbar.separator = format!(" {text} ");
        return Ok(format!("Action bar separator is now \"{text}§r\""));
    }
    hud.actionbar.enabled = args.string("switch") == Some("on");
    Ok(format!("Action bar {}", on_off(hud.actionbar.enabled)))
}

fn bossbar(hud: &mut DebugHud, args: &CommandArgs, now: i64) -> Result<String, String> {
    if args.has("page") {
        let page = args.string("name").and_then(Page::parse).ok_or("unknown page")?;
        hud.bossbar.pinned = Some(page);
        hud.bossbar.enabled = true;
        return Ok(format!("Boss bar pinned to {}", page.name()));
    }
    if args.has("rotate") {
        let seconds = args.float("seconds").ok_or("missing seconds")?;
        if !(seconds > 0.0 && seconds <= MAX_ROTATE_SECONDS) {
            return Err(format!("the rotation time must be above 0 and at most {MAX_ROTATE_SECONDS} seconds"));
        }
        hud.bossbar.rotate_ticks = seconds_to_ticks(seconds);
        hud.bossbar.pinned = None;
        hud.enable_bossbar(now);
        return Ok(format!("Boss bar rotates every {} seconds", ticks_to_seconds(hud.bossbar.rotate_ticks)));
    }
    if args.has("pages") {
        hud.bossbar.pages = Page::parse_list(args.string("list").unwrap_or_default())?;
        hud.restart_rotation(now);
        return Ok(format!("Boss bar rotates through {}", page_names(&hud.bossbar.pages)));
    }
    if args.string("switch") == Some("on") {
        hud.bossbar.pinned = None;
        hud.enable_bossbar(now);
    } else {
        hud.bossbar.enabled = false;
    }
    Ok(format!("Boss bar {}", on_off(hud.bossbar.enabled)))
}

fn describe(hud: &DebugHud) -> String {
    let boss = match (hud.bossbar.enabled, hud.bossbar.pinned) {
        (false, _) => "off".to_owned(),
        (true, Some(page)) => format!("pinned to {}", page.name()),
        (true, None) => format!("rotating every {} s through {}", ticks_to_seconds(hud.bossbar.rotate_ticks), page_names(&hud.bossbar.pages)),
    };
    format!(
        "Debug HUD updates every {} ticks (default {DEFAULT_INTERVAL_TICKS})\nAction bar {}: {}\nBoss bar {boss}\nWidgets: {}\nPages: {}",
        hud.interval_ticks,
        on_off(hud.actionbar.enabled),
        widget_names(hud),
        Widget::names().join(", "),
        Page::names().join(", ")
    )
}

fn widget_names(hud: &DebugHud) -> String {
    hud.actionbar.widgets.iter().map(|widget| widget.name()).collect::<Vec<_>>().join(", ")
}

fn page_names(pages: &[Page]) -> String {
    pages.iter().map(|page| page.name()).collect::<Vec<_>>().join(", ")
}

fn on_off(enabled: bool) -> &'static str {
    if enabled { "on" } else { "off" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::args::parse;
    use crate::debug::DEBUG_COMMAND;
    use chorus_core::permission::PermissionLevel;
    use std::collections::HashMap;

    fn run_line(hud: &mut DebugHud, line: &str) -> Result<String, String> {
        let args = parse("debug", &DEBUG_COMMAND.overloads, line, &HashMap::new(), PermissionLevel::Operator).map_err(|error| error.message)?;
        apply(hud, &args, 0)
    }

    #[test]
    fn interval_is_bounded() {
        let mut hud = DebugHud::default();
        assert!(set_interval(&mut hud, 0).is_err());
        assert!(set_interval(&mut hud, MAX_INTERVAL_TICKS + 1).is_err());
        assert!(set_interval(&mut hud, 40).is_ok());
        assert_eq!(hud.interval_ticks, 40);
    }

    #[test]
    fn description_lists_state() {
        let mut hud = DebugHud::default();
        assert!(describe(&hud).contains("Boss bar off"));
        hud.enable(0);
        let description = describe(&hud);
        assert!(description.contains("Action bar on"));
        assert!(description.contains("rotating every 5 s"));
    }

    #[test]
    fn hud_overloads_parse_and_apply() {
        let mut hud = DebugHud::default();
        run_line(&mut hud, "hud on").unwrap();
        assert!(hud.actionbar.enabled && hud.bossbar.enabled);
        run_line(&mut hud, "hud actionbar widgets tps, mspt memory").unwrap();
        assert_eq!(hud.actionbar.widgets, vec![Widget::Tps, Widget::Mspt, Widget::Memory]);
        run_line(&mut hud, "hud actionbar separator ,").unwrap();
        assert_eq!(hud.actionbar.separator, " , ");
        run_line(&mut hud, "hud actionbar off").unwrap();
        assert!(!hud.actionbar.enabled);
        run_line(&mut hud, "hud bossbar page memory").unwrap();
        assert_eq!(hud.bossbar.pinned, Some(Page::Memory));
        run_line(&mut hud, "hud bossbar rotate 2.5").unwrap();
        assert_eq!((hud.bossbar.pinned, hud.bossbar.rotate_ticks), (None, 50));
        run_line(&mut hud, "hud bossbar pages sending generation").unwrap();
        assert_eq!(hud.bossbar.pages, vec![Page::Sending, Page::Generation]);
        run_line(&mut hud, "hud bossbar off").unwrap();
        assert!(!hud.bossbar.enabled);
        run_line(&mut hud, "hud interval 40").unwrap();
        assert_eq!(hud.interval_ticks, 40);
        run_line(&mut hud, "hud reset").unwrap();
        assert!(!hud.is_active() && hud.interval_ticks == DEFAULT_INTERVAL_TICKS);
        assert!(run_line(&mut hud, "hud").unwrap().contains("Boss bar off"));
    }

    #[test]
    fn invalid_hud_input_is_rejected() {
        let mut hud = DebugHud::default();
        assert!(run_line(&mut hud, "hud actionbar widgets nonsense").is_err());
        assert!(run_line(&mut hud, "hud bossbar rotate 0").is_err());
        assert!(run_line(&mut hud, "hud bossbar page nowhere").is_err());
    }
}
