//! The data every plugin's wasmtime `Store` carries.

use std::fmt;
use std::path::Path;
use std::sync::Arc;

use wasmtime::component::ResourceTable;
use wasmtime_wasi::{FsPerms, WasiCtx, WasiCtxView, WasiView};

use crate::bridge::HostBridge;

/// A host function ran while no dispatch was in progress, so there is no world to read.
///
/// In practice this only happens if a plugin finds a way to call an import from outside a call the
/// host made, which the Component Model does not allow. It is here so the error path is an error
/// rather than a dangling pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldAccessError;

impl fmt::Display for WorldAccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("host function called outside a plugin dispatch")
    }
}

impl std::error::Error for WorldAccessError {}

/// An erased `&mut World`, valid only for the duration of one dispatch.
///
/// Yeah, this is the cursed part of the design. The generated host traits need the world, wasmtime
/// hands them `&mut Store<HostState>`, and the store outlives any single call, so the world cannot
/// live in the store as a reference. It is also erased rather than typed because this crate is the
/// ABI and must not know about Bevy: the bridge in `chorus::plugin` is the only place that casts it
/// back. Nothing dereferences it outside a `PluginHost::call`, which sets it on the way in and
/// clears it on the way out.
#[derive(Clone, Copy)]
struct WorldPtr(*mut ());

// SAFETY: the pointer is written and read on the same thread inside one synchronous dispatch, and
// is cleared before that dispatch returns. It is never sent anywhere; the unsafe impls exist only
// because `WasiView: Send` forces `HostState` to be `Send`.
unsafe impl Send for WorldPtr {}
unsafe impl Sync for WorldPtr {}

pub struct HostState {
    world: Option<WorldPtr>,
    name: String,
    bridge: Arc<dyn HostBridge>,
    wasi: WasiCtx,
    table: ResourceTable,
}

impl HostState {
    /// Builds the store data for one plugin.
    ///
    /// The WASI context is deliberately bare: no network, no process environment, no arguments,
    /// and the plugin's own data folder as the only visible directory. `stderr` is inherited so a
    /// guest panic message survives; a panicking plugin is about to be quarantined and the message
    /// is the only clue its author gets.
    pub fn new(data_dir: &Path, name: &str, bridge: Arc<dyn HostBridge>) -> wasmtime::Result<Self> {
        let mut builder = WasiCtx::builder();
        builder.inherit_stderr();
        builder.preopened_dir(data_dir, ".", FsPerms::ReadWrite)?;

        Ok(Self {
            world: None,
            name: name.to_owned(),
            bridge,
            wasi: builder.build(),
            table: ResourceTable::new(),
        })
    }

    /// Store data for a plugin whose name is not known yet, used for the throwaway instance that
    /// reads metadata. It gets no data folder, which is fine: `metadata` must not call imports.
    pub fn probe(bridge: Arc<dyn HostBridge>) -> Self {
        let mut builder = WasiCtx::builder();
        builder.inherit_stderr();

        Self {
            world: None,
            name: String::from("<probe>"),
            bridge,
            wasi: builder.build(),
            table: ResourceTable::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn bridge(&self) -> &Arc<dyn HostBridge> {
        &self.bridge
    }

    pub(crate) fn enter(&mut self, world: *mut ()) {
        self.world = Some(WorldPtr(world));
    }

    pub(crate) fn leave(&mut self) {
        self.world = None;
    }

    /// The world pointer for the dispatch in progress.
    pub fn world_ptr(&self) -> Result<*mut (), WorldAccessError> {
        self.world.map(|ptr| ptr.0).ok_or(WorldAccessError)
    }
}

impl WasiView for HostState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}
