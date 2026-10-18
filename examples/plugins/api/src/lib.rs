//! Write a Chorus plugin in Rust.
//!
//! Implement [`Plugin`], hand the type to [`register_plugin!`], build for `wasm32-wasip2`, and
//! drop the `.wasm` in the server's `plugins/` folder. That is the whole workflow; there is no
//! manifest file to write.

pub mod bindings {
    wit_bindgen::generate!({
        path: "../../../crates/plugin/wit",
        world: "plugin",
        pub_export_macro: true,
    });
}

pub use bindings::chorus::plugin::common::{BlockPos, GameMode, LogLevel, PermissionLevel, Position};
pub use bindings::chorus::plugin::event::{BlockBreakData, BlockPlaceData, Event, PlayerChatData, PlayerJoinData, PlayerLeaveData};
pub use bindings::chorus::plugin::player::Player;
pub use bindings::chorus::plugin::{command, logging, player, text};
pub use bindings::exports::chorus::plugin::metadata::PluginMetadata as WitMetadata;
pub use command::{Argument, CommandSender, ConsumedArgs, Definition, Overload, Parameter, ParameterType};

/// The `chorus:plugin` package version this crate was built against. `register_plugin!` reports it
/// for you, so a plugin never has to think about it.
pub const ABI_VERSION: &str = "0.1.0";

/// What your plugin tells the server about itself. `name` has to be unique on the server and names
/// your data folder, so pick something specific.
pub struct Metadata {
    pub name: String,
    pub version: String,
    pub authors: Vec<String>,
    pub description: String,
}

/// Implement this, then call [`register_plugin!`] with your type.
///
/// Every hook but `metadata` has a default, so a plugin that only wants a join message overrides
/// one method. Mutable state goes in a `RefCell` inside your type: plugin code is single threaded.
pub trait Plugin {
    /// Built once, before `on_load`. Keep it cheap and do not call into the host here.
    fn new() -> Self
    where
        Self: Sized;

    fn metadata(&self) -> Metadata;

    /// Register your commands here. Returning `Err` quarantines the plugin.
    fn on_load(&self) -> Result<(), String> {
        Ok(())
    }

    fn on_unload(&self) -> Result<(), String> {
        Ok(())
    }

    /// Return the event you were given, with your changes. Returning a different variant case is
    /// an error the host logs, and your changes are dropped.
    fn on_event(&self, event: Event) -> Event {
        event
    }

    /// `id` is what `command::register` handed back. The `i32` is the result count vanilla reports.
    fn on_command(&self, id: u32, sender: CommandSender, args: ConsumedArgs) -> Result<i32, String> {
        let _ = (id, sender, args);
        Ok(0)
    }
}

/// Wires your [`Plugin`] type up as the component's exports.
///
/// ```ignore
/// chorus_plugin_api::register_plugin!(MyPlugin);
/// ```
#[macro_export]
macro_rules! register_plugin {
    ($ty:ty) => {
        const _: () = {
            struct Component;

            // ok so: components are instantiated once and never moved between threads, so a plain
            // static with interior mutability is enough. OnceLock would need std; this won't.
            fn instance() -> &'static $ty {
                static mut INSTANCE: ::core::option::Option<$ty> = ::core::option::Option::None;
                // SAFETY: wasm components are single threaded and this is the only access path.
                unsafe {
                    let slot = &mut *(&raw mut INSTANCE);
                    if slot.is_none() {
                        *slot = ::core::option::Option::Some(<$ty as $crate::Plugin>::new());
                    }
                    slot.as_ref().unwrap()
                }
            }

            impl $crate::bindings::Guest for Component {
                fn on_load() -> ::core::result::Result<(), ::std::string::String> {
                    $crate::Plugin::on_load(instance())
                }

                fn on_unload() -> ::core::result::Result<(), ::std::string::String> {
                    $crate::Plugin::on_unload(instance())
                }

                fn handle_event(_event_id: u32, event: $crate::Event) -> $crate::Event {
                    $crate::Plugin::on_event(instance(), event)
                }

                fn handle_command(command_id: u32, sender: $crate::CommandSender, args: $crate::ConsumedArgs) -> ::core::result::Result<i32, ::std::string::String> {
                    $crate::Plugin::on_command(instance(), command_id, sender, args)
                }
            }

            impl $crate::bindings::exports::chorus::plugin::metadata::Guest for Component {
                fn get() -> $crate::WitMetadata {
                    let metadata = $crate::Plugin::metadata(instance());
                    $crate::WitMetadata {
                        name: metadata.name,
                        version: metadata.version,
                        authors: metadata.authors,
                        description: metadata.description,
                        abi_version: ::std::string::String::from($crate::ABI_VERSION),
                    }
                }
            }

            $crate::bindings::export!(Component with_types_in $crate::bindings);
        };
    };
}
