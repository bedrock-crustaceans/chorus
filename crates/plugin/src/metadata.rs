//! What a plugin says about itself.

use crate::abi::AbiVersion;

/// A plugin's own description of itself, read once at load time.
///
/// `name` is the identity: it has to be unique across loaded plugins, it names the data folder,
/// and it tags the plugin's log lines. There is no manifest file, so this is all the host knows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub authors: Vec<String>,
    pub description: String,
    /// The `chorus:plugin` package version the plugin was built against, verbatim as reported.
    /// Kept as the raw string so an unparseable value can still be logged back at the author.
    pub abi_version: String,
}

impl PluginMetadata {
    /// The reported ABI version, or `None` when it is not a `major.minor.patch` triple.
    pub fn abi(&self) -> Option<AbiVersion> {
        AbiVersion::parse(&self.abi_version)
    }
}
