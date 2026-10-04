use crate::command::command_definition::CommandDefinition;

macro_rules! commands {
    ($($module:ident::$command:ident),* $(,)?) => {
        $(pub mod $module;)*

        pub const DEFINITIONS: &[&CommandDefinition] = &[$(&$module::$command),*];
    };
}

commands! {
    help::HELP_COMMAND,
    ping::PING_COMMAND,
    debug::DEBUG_COMMAND,
    status::STATUS_COMMAND,
    list::LIST_COMMAND,
    save::SAVE_COMMAND,
    stop::STOP_COMMAND,
    compact::COMPACT_COMMAND,
    generate::GENERATE_COMMAND,

    gamemode::GAMEMODE_COMMAND,
    dimension::DIMENSION_COMMAND,
    tp::TP_COMMAND,

    aimassist::AIMASSIST_COMMAND,
    camera::CAMERA_COMMAND,
    camerashake::CAMERASHAKE_COMMAND,
    daylock::DAYLOCK_COMMAND,
}
