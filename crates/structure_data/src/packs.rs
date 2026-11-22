pub const STRUCTURES_FOLDER: &str = "structures";
pub(crate) const STRUCTURES_PREFIX: &str = "structures/";
const LISTED_EXTENSION: &str = ".bin";
pub(crate) const GRID_EXTENSION: &str = ".mcstructure";

pub(crate) fn family_of(name: &str) -> &str {
    name.split('/').next().expect("template family")
}

pub(crate) fn template_key(entry: &str) -> Option<&str> {
    let rest = entry.strip_prefix(STRUCTURES_FOLDER)?.strip_prefix('/')?;
    rest.strip_suffix(LISTED_EXTENSION).or_else(|| rest.strip_suffix(GRID_EXTENSION))
}
