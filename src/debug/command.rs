use super::hud::command::{HUD_ACTION, HUD_PAGE, HUD_SWITCH};
use crate::command::command_definition::CommandDefinition;
use crate::command::parameter::{ArgumentType, CommandEnum, CommandParameter};
use crate::entity::components::actor_id::ActorId;
use crate::network::session::Session;
use crate::player::forms::PendingForms;
use crate::player::gamemode::{FlySpeed, Gamemode};
use crate::values;
use bedrock::form::elems::button::Button;
use bedrock::form::forms::Form;
use bedrock::form::forms::simple::SimpleForm;
use chorus_core::permission::PermissionLevel;
use tracing::info;

#[derive(Clone, Copy, PartialEq, Eq)]
enum FlyAxis {
    Horizontal,
    Vertical,
    Both,
}

impl FlyAxis {
    fn label(self) -> &'static str {
        match self {
            Self::Horizontal => "horizontal",
            Self::Vertical => "vertical",
            Self::Both => "horizontal and vertical",
        }
    }
}

const AXIS: CommandEnum = CommandEnum::new("DebugFlyAxis", values!["horizontal", "vertical", "both"]);

const FEATURE: CommandEnum = CommandEnum::new("DebugFeature", values!["unhandled", "form", "flyspeed"]);

pub static DEBUG_COMMAND: CommandDefinition = CommandDefinition::new("debug", "Used for debugging", |context, args| {
    if args.has("hud") {
        return super::hud::command::run(context, args);
    }
    if args.has("profile") {
        return super::profile::run(context, args);
    }
    match args.string("feature") {
        Some("unhandled") => {
            let Some(session) = context.get::<Session>() else {
                return Err("must be sent by player!".to_owned());
            };
            let report = format!("Unhandled Packets: {:#?}", session.unhandled_packets);
            context.reply(report);
        }
        Some("form") => {
            let name = context.sender_name().to_owned();
            let Some((mut session, mut forms)) = context.components_mut::<(&mut Session, &mut PendingForms)>() else {
                return Err("must be sent by player!".to_owned());
            };

            forms.send(
                &mut session,
                Form::Simple(SimpleForm {
                    body: format!("Hello {}!", name),
                    buttons: vec![
                        Button { text: "Hey!".to_owned(), image: None },
                        Button {
                            text: "Fuck you".to_owned(),
                            image: None,
                        },
                    ],
                    title: "Simple Form".to_owned(),
                }),
                move || {
                    info!("{} responded!", name);
                },
            );
        }
        Some("flyspeed") => {
            let multiplier = args.float("multiplier");
            if multiplier.is_some_and(|multiplier| multiplier < 0.0) {
                return Err("the fly speed multiplier can't be negative".to_owned());
            }
            let axis = match args.string("axis") {
                None | Some("horizontal") => FlyAxis::Horizontal,
                Some("vertical") => FlyAxis::Vertical,
                Some("both") => FlyAxis::Both,
                Some(other) => return Err(format!("\"{other}\" is not a fly speed axis.")),
            };
            let permission = context.permission_level();
            let Some((mut session, gamemode, actor, mut fly_speed)) = context.components_mut::<(&mut Session, &Gamemode, &ActorId, &mut FlySpeed)>() else {
                return Err("must be sent by player!".to_owned());
            };
            let Some(multiplier) = multiplier else {
                let (horizontal, vertical) = (fly_speed.horizontal, fly_speed.vertical);
                context.reply(format!("Fly speed is {horizontal}x horizontal and {vertical}x vertical"));
                return Ok(());
            };

            if axis != FlyAxis::Vertical {
                fly_speed.horizontal = multiplier;
            }
            if axis != FlyAxis::Horizontal {
                fly_speed.vertical = multiplier;
            }
            session.send(gamemode.abilities_packet(actor, permission, None, *fly_speed));
            context.reply(format!("Set {} fly speed to {multiplier}x", axis.label()));
        }
        _ => {}
    }
    Ok(())
})
.style(crate::command::style::CommandStyle::Chorus)
.flags(crate::command::command_definition::CommandFlags::HIGHLIGHTED)
.permission(PermissionLevel::Operator)
.overloads(crate::overloads![
    [
        CommandParameter::enumeration("feature", FEATURE),
        CommandParameter::new("multiplier", ArgumentType::Float).optional(),
        CommandParameter::enumeration("axis", AXIS).optional()
    ],
    [CommandParameter::literal("profile"), CommandParameter::literal("start"), CommandParameter::new("seconds", ArgumentType::Int).optional()],
    [CommandParameter::literal("profile"), CommandParameter::literal("stop")],
    [CommandParameter::literal("hud"), CommandParameter::enumeration("action", HUD_ACTION).optional()],
    [
        CommandParameter::literal("hud"),
        CommandParameter::literal("interval"),
        CommandParameter::new("ticks", ArgumentType::Int)
    ],
    [
        CommandParameter::literal("hud"),
        CommandParameter::literal("actionbar"),
        CommandParameter::enumeration("switch", HUD_SWITCH)
    ],
    [
        CommandParameter::literal("hud"),
        CommandParameter::literal("actionbar"),
        CommandParameter::literal("widgets"),
        CommandParameter::new("list", ArgumentType::RawText)
    ],
    [
        CommandParameter::literal("hud"),
        CommandParameter::literal("actionbar"),
        CommandParameter::literal("separator"),
        CommandParameter::new("text", ArgumentType::RawText)
    ],
    [
        CommandParameter::literal("hud"),
        CommandParameter::literal("bossbar"),
        CommandParameter::enumeration("switch", HUD_SWITCH)
    ],
    [
        CommandParameter::literal("hud"),
        CommandParameter::literal("bossbar"),
        CommandParameter::literal("page"),
        CommandParameter::enumeration("name", HUD_PAGE)
    ],
    [
        CommandParameter::literal("hud"),
        CommandParameter::literal("bossbar"),
        CommandParameter::literal("rotate"),
        CommandParameter::new("seconds", ArgumentType::Float)
    ],
    [
        CommandParameter::literal("hud"),
        CommandParameter::literal("bossbar"),
        CommandParameter::literal("pages"),
        CommandParameter::new("list", ArgumentType::RawText)
    ],
]);
