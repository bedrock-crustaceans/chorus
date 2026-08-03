use crate::network::BedrockProtocol;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use bedrock::protocol::v662::types::NetworkItemInstanceDescriptor;
use bedrock::protocol::v776::enums::ItemVersion;
use bedrock::protocol::v776::packets::{CreativeContentPacket, CreativeItemCategory, CreativeItemData, CreativeItemGroup, ItemComponentPacket, ItemsEntry};
use bevy_ecs::prelude::{Commands, Resource};
use serde::Deserialize;
use std::collections::HashMap;
use tracing::{info, warn};

/// The item table the client resolves every item id against, and the creative menu built on top of
/// it. Both are vanilla data for the protocol chorus speaks.
const ITEM_STATES: &str = include_str!("../resources/runtime_item_states.json");
const CREATIVE_ITEMS: &str = include_str!("../resources/creative_items.json");

#[derive(Deserialize)]
struct ItemState {
    name: String,
    id: i16,
    version: i32,
    #[serde(rename = "componentBased")]
    component_based: bool,
}

#[derive(Deserialize)]
struct CreativeData {
    groups: Vec<CreativeGroupData>,
    items: Vec<CreativeItemEntry>,
}

#[derive(Deserialize)]
struct CreativeGroupData {
    creative_category: i32,
    name: String,
    icon: CreativeItemEntry,
}

#[derive(Deserialize)]
struct CreativeItemEntry {
    id: String,
    #[serde(default)]
    damage: u32,
    #[serde(default)]
    block_state_b64: Option<String>,
    #[serde(default)]
    group_index: u32,
}

pub struct ItemDefinition {
    pub identifier: String,
    pub runtime_id: i16,
    pub version: ItemVersion,
    pub component_based: bool,
}

#[derive(Resource, Default)]
pub struct ItemRegistry {
    items: Vec<ItemDefinition>,
    index: HashMap<String, i16>,

    creative: CreativeContent,
}

#[derive(Default)]
struct CreativeContent {
    groups: Vec<CreativeItemGroup<BedrockProtocol>>,
    contents: Vec<CreativeItemData<BedrockProtocol>>,
}

impl ItemRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(mut commands: Commands) {
        let mut registry = Self::new();

        registry.load_items();
        registry.load_creative();

        info!("registered {} items, {} creative entries", registry.items.len(), registry.creative.contents.len());

        commands.insert_resource(registry);
    }

    fn load_items(&mut self) {
        let states: Vec<ItemState> = match serde_json::from_str(ITEM_STATES) {
            Ok(states) => states,
            Err(err) => {
                warn!("failed to read the item table: {}", err);
                return;
            }
        };

        for state in states {
            self.index.insert(state.name.clone(), state.id);
            self.items.push(ItemDefinition {
                identifier: state.name,
                runtime_id: state.id,
                version: item_version(state.version),
                component_based: state.component_based,
            });
        }
    }

    fn load_creative(&mut self) {
        let creative: CreativeData = match serde_json::from_str(CREATIVE_ITEMS) {
            Ok(creative) => creative,
            Err(err) => {
                warn!("failed to read the creative contents: {}", err);
                return;
            }
        };

        self.creative.groups = creative
            .groups
            .iter()
            .map(|group| CreativeItemGroup {
                category: creative_category(group.creative_category),
                name: group.name.clone(),
                icon: self.describe(&group.icon).unwrap_or_else(air_instance),
            })
            .collect();

        self.creative.contents = creative
            .items
            .iter()
            .filter_map(|item| {
                Some(CreativeItemData {
                    // ids start at 1, the client uses them to identify what was picked
                    creative_net_id: 0,
                    item_instance: self.describe(item)?,
                    group_id: item.group_index,
                })
            })
            .enumerate()
            .map(|(index, mut item)| {
                item.creative_net_id = index as u32 + 1;
                item
            })
            .collect();
    }

    fn describe(&self, entry: &CreativeItemEntry) -> Option<NetworkItemInstanceDescriptor> {
        let runtime_id = *self.index.get(&entry.id)?;

        Some(NetworkItemInstanceDescriptor {
            id: runtime_id as i32,
            stack_size: Some(1),
            aux_value: Some(entry.damage),
            // zero means the item does not place a block
            block_runtime_id: Some(entry.block_state_b64.as_deref().and_then(block_runtime_id).unwrap_or(0)),
            user_data_buffer: Some(vec![]),
        })
    }

    pub fn items(&self) -> &[ItemDefinition] {
        &self.items
    }

    pub fn get(&self, identifier: &str) -> Option<i16> {
        self.index.get(identifier).copied()
    }

    pub fn to_packet(&self) -> ItemComponentPacket<BedrockProtocol> {
        ItemComponentPacket {
            items: self
                .items
                .iter()
                .map(|item| ItemsEntry {
                    component_item_name: item.identifier.clone(),
                    runtime_id: item.runtime_id,
                    is_component_based: item.component_based,
                    version: item.version.clone(),
                    // chorus does not ship the component definitions yet
                    component_data: nbtx::Value::Compound(HashMap::new()),
                })
                .collect(),
        }
    }

    pub fn to_creative_packet(&self) -> CreativeContentPacket<BedrockProtocol> {
        CreativeContentPacket {
            groups: self.creative.groups.clone(),
            contents: self.creative.contents.clone(),
        }
    }
}

/// The block state the item places carries its own network id, so it does not have to be derived.
fn block_runtime_id(block_state_b64: &str) -> Option<i32> {
    let bytes = BASE64_STANDARD.decode(block_state_b64).ok()?;
    let state: HashMap<String, nbtx::Value> = nbtx::from_le_bytes(&mut bytes.as_slice()).ok()?;

    match state.get("network_id")? {
        nbtx::Value::Int(id) => Some(*id),
        _ => None,
    }
}

fn item_version(version: i32) -> ItemVersion {
    match version {
        0 => ItemVersion::Legacy,
        1 => ItemVersion::DataDriven,
        _ => ItemVersion::None,
    }
}

fn creative_category(category: i32) -> CreativeItemCategory {
    match category {
        1 => CreativeItemCategory::Construction,
        2 => CreativeItemCategory::Nature,
        3 => CreativeItemCategory::Equipment,
        4 => CreativeItemCategory::Items,
        _ => CreativeItemCategory::All,
    }
}

fn air_instance() -> NetworkItemInstanceDescriptor {
    NetworkItemInstanceDescriptor {
        id: 0,
        stack_size: None,
        aux_value: None,
        block_runtime_id: None,
        user_data_buffer: None,
    }
}
