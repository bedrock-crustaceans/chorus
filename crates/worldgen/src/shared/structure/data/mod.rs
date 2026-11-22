mod biome_ids;
pub mod climate;
pub mod jigsaw;
pub mod structure_set;
mod worldgen;

pub use biome_ids::pack_biome_id;
pub use chorus_structure_data::{ChainSource, DuplicateIdentifier, Pack, PackSet, PackSource, TableSource};
pub use worldgen::{PackBiomes, PackWorldgen, STRUCTURE_DOCUMENT_SUFFIX};

pub const CHORUS_PACK: &str = "chorus";

pub fn embedded_packs() -> PackSet {
    PackSet::vanilla()
        .with(Pack::new(CHORUS_PACK, Arc::new(TableSource::new(&[crate::shared::resources::CHORUS_FILES]))))
        .expect("the chorus resources define no name the vanilla tables define")
}

use super::builder::StructureRegistration;
use super::layout::{CATALOGUES, PoolCatalogue, PoolDocuments, declare_outputs};
use super::registry::StructureRegistry;
use super::template::TemplateLibrary;
use chorus_structure_data::{StatePoolBuilder, TemplateStore};
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, OnceLock};

pub struct StructureData {
    pub library: Option<TemplateLibrary>,
    pub documents: PoolDocuments,
    pub registry: StructureRegistry,
    pub(in crate::shared::structure) catalogues: HashMap<&'static str, PoolCatalogue>,
}

impl StructureData {
    pub fn empty() -> Self {
        Self {
            library: None,
            documents: PoolDocuments::default(),
            registry: StructureRegistry::load_default().expect("the structure registry data is part of the source tree"),
            catalogues: HashMap::new(),
        }
    }

    pub fn shared() -> Option<Arc<Self>> {
        static SHARED: OnceLock<Option<Arc<StructureData>>> = OnceLock::new();
        SHARED
            .get_or_init(|| {
                Self::load()
                    .inspect_err(|error| tracing::error!("structure data failed to load, generating without structures: {error}"))
                    .ok()
                    .map(Arc::new)
            })
            .clone()
    }

    pub fn load() -> Result<Self, Box<dyn Error>> {
        Self::load_with(&embedded_packs(), Vec::new())
    }

    pub fn load_with(packs: &PackSet, registrations: Vec<StructureRegistration>) -> Result<Self, Box<dyn Error>> {
        let documents = PoolDocuments::load(packs)?;
        let registry = StructureRegistry::load_with(packs, registrations)?;
        let mut pool = StatePoolBuilder::new();
        let store = TemplateStore::load(packs, &mut pool)?;
        declare_outputs(&documents, &mut pool)?;
        for (_, registration) in registry.registrations() {
            (registration.declare_states)(&mut pool);
        }
        let library = TemplateLibrary::new(store, Arc::new(pool.freeze()?));
        let catalogues = CATALOGUES
            .into_iter()
            .map(|name| Ok((name, PoolCatalogue::build(&documents, name, &library)?)))
            .collect::<Result<_, Box<dyn Error>>>()?;
        Ok(Self {
            library: Some(library),
            documents,
            registry,
            catalogues,
        })
    }

    pub fn catalogue(&self, name: &str) -> Option<&PoolCatalogue> {
        self.catalogues.get(name)
    }

    pub fn feature_element_names(&self) -> Vec<String> {
        self.documents.feature_names()
    }
}
