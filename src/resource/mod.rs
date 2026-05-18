use std::collections::HashMap;

use bevy_ecs::prelude::*;
use tracing::{debug, info, warn};

use crate::config::Config;
use crate::resource::resource_pack::ResourcePack;

pub mod resource_pack;

#[derive(Resource, Default)]
pub struct ResourcePacks {
    packs: Vec<ResourcePack>,
    by_id: HashMap<String, usize>,
}

impl ResourcePacks {
    pub fn packs(&self) -> &[ResourcePack] {
        &self.packs
    }

    pub fn get(&self, id: &str) -> Option<&ResourcePack> {
        if let Some(&idx) = self.by_id.get(id) {
            return Some(&self.packs[idx]);
        }
        let uuid_part = id.split('_').next().unwrap_or(id);
        self.packs.iter().find(|p| p.uuid.to_string() == uuid_part)
    }

    pub fn load(config: Res<Config>, mut commands: Commands) {
        let mut res = ResourcePacks::default();

        let entries = match std::fs::read_dir(&config.resource_packs_directory) {
            Ok(e) => e,
            Err(err) => {
                warn!("Could not read resource packs directory: {err}");
                commands.insert_resource(res);
                return;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext != "mcpack" && ext != "zip" {
                continue;
            }

            match ResourcePack::from_zip(&path) {
                Ok(pack) => {
                    debug!("Loaded resource pack '{}' ({})", pack.name, pack.uuid);
                    let idx = res.packs.len();
                    res.by_id.insert(pack.pack_id(), idx);
                    res.packs.push(pack);
                }
                Err(err) => {
                    warn!("Skipping {:?}: {err}", path.file_name().unwrap_or_default());
                }
            }
        }

        info!("Loaded {} resource pack(s).", res.packs.len());
        commands.insert_resource(res);
    }
}
