use std::collections::{BTreeSet, HashMap};
use std::error::Error;
use std::fmt;
use std::sync::Arc;

use super::packs::{STRUCTURES_PREFIX, family_of, template_key};
use super::source::{PackSource, TableSource};

#[derive(Clone)]
pub struct Pack {
    pub id: String,
    pub source: Arc<dyn PackSource>,
}

impl Pack {
    pub fn new(id: impl Into<String>, source: Arc<dyn PackSource>) -> Self {
        Self { id: id.into(), source }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateIdentifier {
    pub identifier: String,
    pub first: String,
    pub second: String,
}

impl fmt::Display for DuplicateIdentifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} is defined by both pack {} and pack {}", self.identifier, self.first, self.second)
    }
}

impl Error for DuplicateIdentifier {}

#[derive(Clone, Default)]
pub struct PackSet {
    packs: Vec<Pack>,
    owners: HashMap<String, usize>,
}

impl PackSet {
    pub const VANILLA: &'static str = "vanilla";

    pub fn new() -> Self {
        Self::default()
    }

    pub fn vanilla() -> Self {
        Self::new()
            .with(Pack::new(Self::VANILLA, Arc::new(VanillaSource::new())))
            .expect("the vanilla tables define every name once")
    }

    pub fn add(&mut self, pack: Pack) -> Result<(), DuplicateIdentifier> {
        let index = self.packs.len();
        let mut claimed = Vec::new();
        for name in pack.source.names() {
            if let Some(&owner) = self.owners.get(&name) {
                return Err(DuplicateIdentifier {
                    identifier: name,
                    first: self.packs[owner].id.clone(),
                    second: pack.id,
                });
            }
            claimed.push(name);
        }
        self.owners.extend(claimed.into_iter().map(|name| (name, index)));
        self.packs.push(pack);
        Ok(())
    }

    pub fn with(mut self, pack: Pack) -> Result<Self, DuplicateIdentifier> {
        self.add(pack)?;
        Ok(self)
    }

    pub fn packs(&self) -> &[Pack] {
        &self.packs
    }

    pub fn owner(&self, name: &str) -> Option<&str> {
        self.owners.get(name).map(|&index| self.packs[index].id.as_str())
    }

    pub fn structure_families(&self) -> BTreeSet<String> {
        self.template_names().iter().map(|name| family_of(name).to_owned()).collect()
    }

    pub fn template_entries(&self) -> Result<Vec<(String, String)>, DuplicateIdentifier> {
        let mut found: Vec<(String, String)> = self.owners.keys().filter_map(|entry| Some((template_key(entry)?.to_owned(), entry.clone()))).collect();
        found.sort();
        for pair in found.windows(2) {
            if pair[0].0 == pair[1].0 {
                return Err(DuplicateIdentifier {
                    identifier: pair[0].0.clone(),
                    first: self.packs[self.owners[&pair[0].1]].id.clone(),
                    second: self.packs[self.owners[&pair[1].1]].id.clone(),
                });
            }
        }
        Ok(found)
    }

    pub fn template_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.owners.keys().filter_map(|entry| template_key(entry).map(str::to_owned)).collect();
        names.sort();
        names
    }
}

struct VanillaSource {
    tables: TableSource,
}

impl VanillaSource {
    fn new() -> Self {
        Self {
            tables: TableSource::new(&[
                bedrock_vanilla::biomes(),
                bedrock_vanilla::climate(),
                bedrock_vanilla::feature_rules(),
                bedrock_vanilla::features(),
                bedrock_vanilla::jigsaw(),
                bedrock_vanilla::worldgen(),
            ]),
        }
    }
}

impl PackSource for VanillaSource {
    fn names(&self) -> Vec<String> {
        let mut names = self.tables.names();
        names.extend(bedrock_vanilla::structures().names().iter().map(|name| format!("{STRUCTURES_PREFIX}{name}")));
        names
    }

    fn read(&self, name: &str) -> Option<Vec<u8>> {
        match name.strip_prefix(STRUCTURES_PREFIX) {
            Some(template) => bedrock_vanilla::structures().get(template)?.ok()?.to_bytes().ok(),
            None => self.tables.read(name),
        }
    }

    fn packed(&self, name: &str) -> Option<Result<bedrock_vanilla::Entry<'_>, Box<dyn Error>>> {
        let template = name.strip_prefix(STRUCTURES_PREFIX)?;
        Some(bedrock_vanilla::structures().get(template)?.map_err(Into::into))
    }
}

impl PackSource for PackSet {
    fn names(&self) -> Vec<String> {
        self.packs.iter().flat_map(|pack| pack.source.names()).collect()
    }

    fn packed(&self, name: &str) -> Option<Result<bedrock_vanilla::Entry<'_>, Box<dyn Error>>> {
        let owner = *self.owners.get(name)?;
        self.packs[owner].source.packed(name)
    }

    fn read(&self, name: &str) -> Option<Vec<u8>> {
        let owner = *self.owners.get(name)?;
        self.packs[owner].source.read(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::Table;

    static FIRST: Table = &[("structures/alpha/one.bin", b"1"), ("worldgen/template_pools/shared.json", b"{}")];
    static SECOND: Table = &[("structures/beta/readme.txt", b"x"), ("structures/beta/two.mcstructure", b"2")];
    static CLASH: Table = &[("worldgen/template_pools/shared.json", b"{}")];

    fn pack(id: &str, table: Table) -> Pack {
        Pack::new(id, Arc::new(TableSource::new(&[table])))
    }

    #[test]
    fn a_name_defined_by_two_packs_is_an_error_naming_both() {
        let mut set = PackSet::new().with(pack("first", FIRST)).unwrap();
        let error = set.add(pack("clash", CLASH)).unwrap_err();
        assert_eq!(error.identifier, "worldgen/template_pools/shared.json");
        assert_eq!((error.first.as_str(), error.second.as_str()), ("first", "clash"));
        let text = error.to_string();
        assert!(text.contains("first") && text.contains("clash") && text.contains("shared.json"), "{text}");
        assert_eq!(set.packs().len(), 1);
    }

    #[test]
    fn ordered_packs_expose_the_names_of_both() {
        let set = PackSet::new().with(pack("first", FIRST)).unwrap().with(pack("second", SECOND)).unwrap();
        let mut names = set.names();
        names.sort();
        assert_eq!(names.len(), 4);
        assert_eq!(set.read("structures/alpha/one.bin"), Some(b"1".to_vec()));
        assert_eq!(set.read("structures/beta/two.mcstructure"), Some(b"2".to_vec()));
        assert_eq!(set.read("structures/gamma/none.bin"), None);
        assert_eq!(set.owner("structures/beta/two.mcstructure"), Some("second"));
        assert_eq!(set.packs().iter().map(|pack| pack.id.as_str()).collect::<Vec<_>>(), ["first", "second"]);
    }

    #[test]
    fn families_are_discovered_from_the_first_segment_under_structures() {
        let set = PackSet::new().with(pack("first", FIRST)).unwrap();
        assert_eq!(set.structure_families().into_iter().collect::<Vec<_>>(), ["alpha"]);
        let set = set.with(pack("second", SECOND)).unwrap();
        assert_eq!(set.structure_families().into_iter().collect::<Vec<_>>(), ["alpha", "beta"]);
        assert_eq!(set.template_names(), ["alpha/one", "beta/two"]);
    }

    static LISTED: Table = &[("structures/alpha/one.bin", b"1")];
    static GRID: Table = &[("structures/alpha/one.mcstructure", b"2")];

    #[test]
    fn a_template_stored_in_both_formats_is_an_error() {
        let set = PackSet::new().with(pack("listed", LISTED)).unwrap().with(pack("grid", GRID)).unwrap();
        let error = set.template_entries().unwrap_err();
        assert_eq!(error.identifier, "alpha/one");
        assert_eq!((error.first.as_str(), error.second.as_str()), ("listed", "grid"));
    }

    #[test]
    fn the_vanilla_pack_discovers_every_template_family() {
        let set = PackSet::vanilla();
        let expected = [
            "abandoned_camp",
            "ancient_city",
            "bastion",
            "coralcrust",
            "endcity",
            "fossils",
            "igloo",
            "mansion",
            "nether_fossils",
            "pillageroutpost",
            "ruin",
            "ruined_portal",
            "shipwreck",
            "sulfur_spring",
            "trail_ruins",
            "trial_chambers",
            "village",
        ];
        assert_eq!(set.structure_families().into_iter().collect::<Vec<_>>(), expected);
        assert_eq!(set.packs().len(), 1);
        let entries = set.template_entries().unwrap();
        assert_eq!(entries.len(), 1300);
    }
}
