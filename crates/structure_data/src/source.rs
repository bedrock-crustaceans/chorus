use std::error::Error;
use std::sync::Arc;

pub trait PackSource: Send + Sync {
    fn names(&self) -> Vec<String>;
    fn read(&self, name: &str) -> Option<Vec<u8>>;

    fn packed(&self, _name: &str) -> Option<Result<bedrock_vanilla::Entry<'_>, Box<dyn Error>>> {
        None
    }
}

pub type Table = &'static [(&'static str, &'static [u8])];

pub struct TableSource {
    tables: Vec<Table>,
}

impl TableSource {
    pub fn new(tables: &[Table]) -> Self {
        Self { tables: tables.to_vec() }
    }
}

impl PackSource for TableSource {
    fn names(&self) -> Vec<String> {
        self.tables.iter().flat_map(|table| table.iter().map(|(name, _)| (*name).to_owned())).collect()
    }

    fn read(&self, name: &str) -> Option<Vec<u8>> {
        self.tables.iter().find_map(|table| {
            let index = table.binary_search_by_key(&name, |(entry, _)| entry).ok()?;
            Some(table[index].1.to_vec())
        })
    }
}

pub struct ChainSource {
    first: Arc<dyn PackSource>,
    second: Arc<dyn PackSource>,
}

impl ChainSource {
    pub fn new(first: Arc<dyn PackSource>, second: Arc<dyn PackSource>) -> Self {
        Self { first, second }
    }
}

impl PackSource for ChainSource {
    fn names(&self) -> Vec<String> {
        let mut names = self.first.names();
        names.extend(self.second.names());
        names
    }

    fn read(&self, name: &str) -> Option<Vec<u8>> {
        self.first.read(name).or_else(|| self.second.read(name))
    }

    fn packed(&self, name: &str) -> Option<Result<bedrock_vanilla::Entry<'_>, Box<dyn Error>>> {
        self.first.packed(name).or_else(|| self.second.packed(name))
    }
}

pub fn list(source: &dyn PackSource, folder: &str, suffix: &str, nested: bool) -> Vec<String> {
    let prefix = format!("{folder}/");
    let mut names: Vec<String> = source
        .names()
        .into_iter()
        .filter(|name| name.strip_prefix(&prefix).is_some_and(|rest| name.ends_with(suffix) && (nested || !rest.contains('/'))))
        .collect();
    names.sort_by(|a, b| a.split('/').cmp(b.split('/')));
    names
}

pub fn read_text(source: &dyn PackSource, name: &str) -> Result<String, Box<dyn Error>> {
    let bytes = source.read(name).ok_or_else(|| format!("missing entry {name}"))?;
    Ok(String::from_utf8(bytes).map_err(|error| format!("{name}: {error}"))?)
}
