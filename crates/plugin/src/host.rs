//! Loading and running plugin components.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use chorus_core::config::PluginsConfig;
use tracing::{debug, info, warn};
use wasmtime::component::{Component, Linker};
use wasmtime::{Engine, Store};

use crate::Plugin as PluginBindings;
use crate::abi::ABI_VERSION;
use crate::bridge::{HostBridge, NoopBridge};
use crate::metadata::PluginMetadata;
use crate::state::HostState;

/// Epochs a dispatch may span. The ticker bumps the epoch once per configured budget, so two
/// means a call always gets at least one full budget interval even if it starts just before a bump.
const TICK_DEADLINE: u64 = 2;

/// Loading is not the tick loop. Compiling a component and running its `on_load` legitimately take
/// far longer than a tick's budget, so nothing is cut off while the server is still starting. Not
/// `u64::MAX`: wasmtime adds the deadline to the current epoch and panics on overflow.
const LOAD_DEADLINE: u64 = 1_000_000;

/// Whether a plugin is still in the rotation.
///
/// Quarantine is one way: a plugin that trapped or failed to load stays out for the rest of the
/// process. Restarting it would re-run `on-load` against a server that has already moved on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginStatus {
    Running,
    Quarantined(String),
}

pub struct LoadedPlugin {
    pub metadata: PluginMetadata,
    pub state: PluginStatus,
    store: Store<HostState>,
    instance: PluginBindings,
}

impl LoadedPlugin {
    pub fn is_running(&self) -> bool {
        self.state == PluginStatus::Running
    }
}

/// Owns the wasmtime engine and every loaded plugin.
///
/// `new` wires WASI and every `chorus:plugin` import onto the linker, so the only thing a caller
/// has to supply is the [`HostBridge`] the imports forward to. [`Self::linker_mut`] is there for a
/// host that wants to add imports of its own; anything added has to go on before
/// [`Self::load_directory`], which instantiates each component as it loads it.
pub struct PluginHost {
    engine: Engine,
    linker: Linker<HostState>,
    directory: PathBuf,
    fuel: u64,
    bridge: Arc<dyn HostBridge>,
    plugins: Vec<LoadedPlugin>,
    index: HashMap<String, usize>,
    ticker_stop: Arc<AtomicBool>,
    deadline: u64,
}

impl PluginHost {
    pub fn new(config: &PluginsConfig, bridge: Arc<dyn HostBridge>) -> wasmtime::Result<Self> {
        let mut settings = wasmtime::Config::new();
        settings.consume_fuel(true);
        settings.epoch_interruption(true);
        let engine = Engine::new(&settings)?;

        // hmm, epoch interruption only bites if somebody else bumps the epoch, and a runaway
        // plugin is running on this very thread, so the bump has to come from its own thread.
        // Fuel catches pure computation; this catches a call that blocks without burning fuel.
        let ticker_stop = Arc::new(AtomicBool::new(false));
        if let Some(budget) = config.budget.get() {
            spawn_epoch_ticker(&engine, budget, Arc::clone(&ticker_stop));
        }

        let mut linker = Linker::new(&engine);
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
        crate::imports::add_to_linker(&mut linker)?;

        Ok(Self {
            engine,
            linker,
            directory: config.directory.clone(),
            fuel: config.fuel,
            bridge,
            plugins: Vec::new(),
            index: HashMap::new(),
            ticker_stop,
            deadline: TICK_DEADLINE,
        })
    }

    /// Add the generated host imports here before loading anything.
    pub fn linker_mut(&mut self) -> &mut Linker<HostState> {
        &mut self.linker
    }

    pub fn plugins(&self) -> impl Iterator<Item = &LoadedPlugin> {
        self.plugins.iter()
    }

    /// Names of the plugins still in the rotation, in load order, so a dispatch can walk them
    /// without holding a borrow on the host.
    pub fn running_names(&self) -> Vec<String> {
        self.plugins.iter().filter(|plugin| plugin.is_running()).map(|plugin| plugin.metadata.name.clone()).collect()
    }

    pub fn quarantine(&mut self, name: &str, reason: impl Into<String>) {
        let Some(&position) = self.index.get(name) else { return };
        let reason = reason.into();
        warn!(plugin = name, "quarantined: {reason}");
        self.plugins[position].state = PluginStatus::Quarantined(reason);
    }

    /// Loads every `.wasm` file in the configured directory, creating it when it does not exist.
    ///
    /// Returns how many plugins joined the rotation. Nothing here propagates: a plugin that fails
    /// to load is a log line, not a reason for the server not to start.
    pub fn load_directory(&mut self) -> usize {
        let before = self.plugins.len();

        if let Err(err) = fs::create_dir_all(&self.directory) {
            warn!("could not open the plugin directory {}: {err}", self.directory.display());
            return 0;
        }

        let mut paths: Vec<PathBuf> = match fs::read_dir(&self.directory) {
            Ok(entries) => entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("wasm")))
                .collect(),
            Err(err) => {
                warn!("could not read the plugin directory {}: {err}", self.directory.display());
                return 0;
            }
        };
        // load order decides event order, so make it the obvious one rather than the filesystem's
        paths.sort();

        for path in paths {
            let file = path.file_name().unwrap_or_default().to_string_lossy().into_owned();
            match self.load_file(&path) {
                Ok(metadata) => info!("loaded plugin {} {}", metadata.name, metadata.version),
                Err(err) => warn!("skipped {file}: {err}"),
            }
        }

        self.plugins.len() - before
    }

    fn load_file(&mut self, path: &Path) -> wasmtime::Result<PluginMetadata> {
        let component = Component::from_file(&self.engine, path)?;
        let metadata = self.probe_metadata(&component)?;

        match metadata.abi() {
            Some(version) if version.compatible(&ABI_VERSION) => {}
            Some(version) => return Err(incompatible(&version.to_string())),
            None => return Err(incompatible(&metadata.abi_version)),
        }

        if self.index.contains_key(&metadata.name) {
            return Err(wasmtime::Error::msg(format!("a plugin named {} is already loaded", metadata.name)));
        }

        let data_dir = self.directory.join(&metadata.name);
        fs::create_dir_all(&data_dir)?;

        let mut store = Store::new(&self.engine, HostState::new(&data_dir, &metadata.name, Arc::clone(&self.bridge))?);
        store.set_fuel(self.fuel)?;
        store.set_epoch_deadline(LOAD_DEADLINE);
        let instance = PluginBindings::instantiate(&mut store, &component, &self.linker)?;

        self.index.insert(metadata.name.clone(), self.plugins.len());
        self.plugins.push(LoadedPlugin {
            metadata: metadata.clone(),
            state: PluginStatus::Running,
            store,
            instance,
        });

        Ok(metadata)
    }

    /// Reads a component's metadata from a throwaway instance.
    ///
    /// The data folder is named after `metadata.name`, so the real store cannot be built until the
    /// name is known. `metadata` is documented as a pure snapshot that calls no imports, which is
    /// what makes a bridge-less instance enough.
    fn probe_metadata(&self, component: &Component) -> wasmtime::Result<PluginMetadata> {
        let mut store = Store::new(&self.engine, HostState::probe(Arc::new(NoopBridge)));
        store.set_fuel(self.fuel)?;
        store.set_epoch_deadline(LOAD_DEADLINE);
        let instance = PluginBindings::instantiate(&mut store, component, &self.linker)?;
        let reported = instance.chorus_plugin_metadata().call_get(&mut store)?;

        Ok(PluginMetadata {
            name: reported.name,
            version: reported.version,
            authors: reported.authors,
            description: reported.description,
            abi_version: reported.abi_version,
        })
    }

    /// Runs `f` against one plugin with the world pointer in place.
    ///
    /// Returns `None` when the plugin is unknown, quarantined, or trapped. A trap quarantines it,
    /// so a plugin only gets to blow up once.
    pub fn call<R>(&mut self, name: &str, world: *mut (), f: impl FnOnce(&mut Store<HostState>, &PluginBindings) -> wasmtime::Result<R>) -> Option<R> {
        let &position = self.index.get(name)?;
        let fuel = self.fuel;
        let plugin = &mut self.plugins[position];
        if !plugin.is_running() {
            return None;
        }

        if let Err(err) = plugin.store.set_fuel(fuel) {
            self.quarantine(name, format!("could not refill fuel: {err}"));
            return None;
        }
        plugin.store.set_epoch_deadline(self.deadline);

        plugin.store.data_mut().enter(world);
        let result = f(&mut plugin.store, &plugin.instance);
        plugin.store.data_mut().leave();

        match result {
            Ok(value) => Some(value),
            Err(err) => {
                self.quarantine(name, err.to_string());
                None
            }
        }
    }

    /// Calls `on-load` on every plugin that is still running.
    ///
    /// Runs without a tick deadline: a plugin is allowed to do real work on startup.
    pub fn load_all(&mut self, world: *mut ()) {
        self.deadline = LOAD_DEADLINE;
        for name in self.running_names() {
            let outcome = self.call(&name, world, |store, plugin| plugin.call_on_load(store));
            match outcome {
                Some(Ok(())) => debug!(plugin = %name, "on_load returned"),
                Some(Err(err)) => self.quarantine(&name, format!("on_load failed: {err}")),
                None => {}
            }
        }
        self.deadline = TICK_DEADLINE;
    }

    /// Calls `on-unload` on every plugin that is still running, for a clean shutdown.
    pub fn unload_all(&mut self, world: *mut ()) {
        self.deadline = LOAD_DEADLINE;
        for name in self.running_names() {
            if let Some(Err(err)) = self.call(&name, world, |store, plugin| plugin.call_on_unload(store)) {
                warn!(plugin = %name, "on_unload failed: {err}");
            }
        }
        self.deadline = TICK_DEADLINE;
    }
}

impl Drop for PluginHost {
    fn drop(&mut self) {
        self.ticker_stop.store(true, Ordering::Relaxed);
    }
}

fn incompatible(reported: &str) -> wasmtime::Error {
    wasmtime::Error::msg(format!("built against chorus:plugin {reported}, this server speaks {ABI_VERSION}"))
}

/// One thread per host, bumping the engine's epoch so a plugin call that neither returns nor burns
/// fuel still gets cut off.
fn spawn_epoch_ticker(engine: &Engine, interval: Duration, stop: Arc<AtomicBool>) {
    let engine = engine.clone();
    thread::spawn(move || {
        while !stop.load(Ordering::Relaxed) {
            thread::sleep(interval);
            engine.increment_epoch();
        }
    });
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicU32;

    use chorus_core::config::Interval;

    use super::*;
    use crate::state::HostState;

    /// The example plugin, built for `wasm32-wasip2`. The tests that need it are `#[ignore]`d
    /// because building it is a separate cargo invocation against a different target.
    fn fixture() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/hello/target/wasm32-wasip2/release/hello.wasm")
    }

    static NEXT_DIR: AtomicU32 = AtomicU32::new(0);

    fn scratch_dir() -> PathBuf {
        let unique = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("chorus-plugin-test-{}-{unique}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    fn config(directory: PathBuf) -> PluginsConfig {
        PluginsConfig {
            enabled: true,
            directory,
            fuel: 10_000_000,
            // no epoch ticker in tests: it would spawn a thread per host for no benefit
            budget: Interval::DISABLED,
        }
    }

    fn host(directory: PathBuf) -> PluginHost {
        PluginHost::new(&config(directory), Arc::new(NoopBridge)).expect("engine")
    }

    #[test]
    #[ignore]
    fn loads_a_component_and_reads_its_metadata() {
        let dir = scratch_dir();
        fs::copy(fixture(), dir.join("hello.wasm")).expect("fixture built; run the build in examples/plugins/README.md");
        let mut host = host(dir);

        assert_eq!(host.load_directory(), 1);
        assert_eq!(host.plugins().next().unwrap().metadata.name, "hello");
    }

    #[test]
    #[ignore]
    fn skips_a_file_that_is_not_a_component() {
        let dir = scratch_dir();
        fs::copy(fixture(), dir.join("hello.wasm")).expect("fixture");
        fs::write(dir.join("bad.wasm"), b"not wasm at all").expect("write");
        let mut host = host(dir);

        assert_eq!(host.load_directory(), 1);
        assert_eq!(host.plugins().next().unwrap().metadata.name, "hello");
    }

    #[test]
    #[ignore]
    fn refuses_a_duplicate_plugin_name() {
        let dir = scratch_dir();
        fs::copy(fixture(), dir.join("hello.wasm")).expect("fixture");
        fs::copy(fixture(), dir.join("hello-again.wasm")).expect("fixture");
        let mut host = host(dir);

        assert_eq!(host.load_directory(), 1);
    }

    #[test]
    #[ignore]
    fn a_quarantined_plugin_is_not_called() {
        let dir = scratch_dir();
        fs::copy(fixture(), dir.join("hello.wasm")).expect("fixture");
        let mut host = host(dir);
        host.load_directory();

        host.quarantine("hello", "test");

        assert!(host.call("hello", std::ptr::null_mut(), |_, _| Ok(())).is_none());
        assert!(matches!(host.plugins().next().unwrap().state, PluginStatus::Quarantined(_)));
        assert!(host.running_names().is_empty());
    }

    #[test]
    fn a_host_call_without_a_world_is_an_error() {
        let state = HostState::probe(Arc::new(NoopBridge));

        assert!(state.world_ptr().is_err());
    }
}
