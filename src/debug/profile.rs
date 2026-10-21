use crate::Scheduler;
use crate::command::args::CommandArgs;
use crate::command::context::CommandContext;
use crate::level::level::Level;
use crate::server::ServerState;
use crate::server::profile::{DEFAULT_PROFILE_SECONDS, Snapshot, TickProfile, finish_session, profile_seconds};

pub fn run(context: &mut CommandContext, args: &CommandArgs) -> Result<(), String> {
    let profiling = context.resource::<TickProfile>().is_profiling();
    if args.has("stop") {
        if !profiling {
            return Err("No profile is running".to_owned());
        }
        finish_session(context.world_mut());
        return Ok(());
    }

    if profiling {
        return Err("A profile is already running, use /debug profile stop to end it".to_owned());
    }
    let seconds = args.int("seconds").map_or(DEFAULT_PROFILE_SECONDS, |seconds| profile_seconds(seconds.max(1) as u32));
    let requester = context.sender();
    let tick = context.resource::<ServerState>().tick();
    let snapshot = Snapshot::take(context.resource::<Level>(), context.resource::<Scheduler>());
    context.world_mut().resource_mut::<TickProfile>().start(requester, seconds, snapshot, tick);
    context.reply(format!("Profiling for {seconds} s, the report follows"));
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::command::args::parse;
    use crate::debug::DEBUG_COMMAND;
    use chorus_core::permission::PermissionLevel;
    use std::collections::HashMap;

    fn seconds(command: &crate::command::command_definition::CommandDefinition, line: &str) -> Option<i32> {
        parse(&command.name, &command.overloads, line, &HashMap::new(), PermissionLevel::Operator).unwrap().int("seconds")
    }

    #[test]
    fn profile_start_takes_an_optional_duration() {
        assert_eq!(seconds(&DEBUG_COMMAND, "profile start 45"), Some(45));
        assert_eq!(seconds(&DEBUG_COMMAND, "profile start"), None);
        assert!(
            parse("debug", &DEBUG_COMMAND.overloads, "profile stop", &HashMap::new(), PermissionLevel::Operator)
                .unwrap()
                .has("stop")
        );
    }
}
