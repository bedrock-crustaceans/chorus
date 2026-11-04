use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use bedrock::protocol::ProtoVersionPackets;
use bedrock::protocol::v776::enums::ItemVersion;
use bedrock::protocol::v776::packets::{ItemComponentPacket, ItemsEntry};
use bedrock::protocol::v2168::packets::{CreativeContentPacket, CreativeItemCategory, CreativeItemData, CreativeItemGroup};
use bedrock::protocol::v2168::types::NetworkItemInstanceDescriptor;
use bevy_ecs::prelude::{Commands, Resource};
use chorus_core::protocol::BedrockProtocol;
use indexmap::IndexMap;
use serde::Deserialize;
use std::collections::HashMap;
use tracing::{info, warn};

/// The item table the client resolves every item id against, and the creative menu built on top of
/// it. Both are vanilla data for the protocol chorus speaks.
const ITEM_PALETTE: &str = include_str!("../resources/item_palette.json");
const CREATIVE_ITEMS: &str = include_str!("../resources/creative_items.json");
const ITEM_COMPONENTS: &[u8] = include_bytes!("../resources/item_components.nbt");

#[derive(Deserialize)]
struct ItemPalette {
    pub items: Vec<ItemState>,
}

#[derive(Deserialize)]
struct ItemState {
    name: String,
    id: i16,
    version: i32,
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
    max_stacks: HashMap<i16, u16>,
    components: HashMap<String, nbtx::Value>,

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

    pub fn identifier(&self, runtime_id: i16) -> Option<&str> {
        self.items
            .iter()
            .find(|definition| definition.runtime_id == runtime_id)
            .map(|definition| definition.identifier.as_str())
    }

    pub fn max_stack_size(&self, runtime_id: i16) -> u16 {
        self.max_stacks.get(&runtime_id).copied().unwrap_or(64)
    }

    #[tracing::instrument(name = "registry", skip_all)]
    pub fn init(mut commands: Commands) {
        let mut registry = Self::new();

        registry.load_items();
        registry.load_components();
        registry.load_creative();

        info!("registered {} items, {} creative entries", registry.items.len(), registry.creative.contents.len());

        commands.insert_resource(registry);
    }

    fn load_items(&mut self) {
        let palette = match serde_json::from_str::<ItemPalette>(ITEM_PALETTE) {
            Ok(palette) => palette.items,
            Err(err) => {
                warn!("failed to read the item table: {}", err);
                return;
            }
        };

        for item in palette {
            self.index.insert(item.name.clone(), item.id);
            self.max_stacks.insert(item.id, crate::stack_size::max_stack_size(&item.name));
            self.items.push(ItemDefinition {
                identifier: item.name,
                runtime_id: item.id,
                version: item_version(item.version),
                component_based: item.component_based,
            });
        }
    }

    fn load_components(&mut self) {
        let mut bytes = ITEM_COMPONENTS;
        let entries: HashMap<String, HashMap<String, nbtx::Value>> = match nbtx::from_be_bytes(&mut bytes) {
            Ok(entries) => entries,
            Err(err) => {
                warn!("failed to read the item components: {}", err);
                return;
            }
        };
        for (identifier, mut entry) in entries {
            if let Some(components) = entry.remove("components") {
                self.components.insert(identifier, components);
            }
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
            stack_size: 1,
            aux_value: entry.damage,
            // zero means the item does not place a block
            block_runtime_id: entry.block_state_b64.as_deref().and_then(block_runtime_id).unwrap_or(0),
            user_data_buffer: vec![],
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
                .filter_map(|item| {
                    let components = self.components.get(&item.identifier)?;
                    Some(ItemsEntry {
                        component_item_name: item.identifier.clone(),
                        runtime_id: item.runtime_id,
                        is_component_based: item.component_based,
                        version: item.version.clone(),
                        component_data: nbtx::Value::Compound(IndexMap::from([("components".into(), components.clone())])),
                    })
                })
                .collect(),
        }
    }

    pub fn to_creative_packet(&self) -> <BedrockProtocol as ProtoVersionPackets>::CreativeContentPacket {
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
        stack_size: 0,
        aux_value: 0,
        block_runtime_id: 0,
        user_data_buffer: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn registry() -> ItemRegistry {
        let mut registry = ItemRegistry::new();
        registry.load_items();
        registry.load_components();
        registry.load_creative();
        registry
    }

    #[test]
    fn known_items_resolve_by_identifier() {
        let registry = registry();
        for identifier in ["minecraft:stone", "minecraft:apple", "minecraft:diamond"] {
            assert!(registry.get(identifier).is_some(), "{identifier}");
        }
        assert_eq!(registry.get("minecraft:not_an_item"), None);
    }

    #[test]
    fn lookups_agree_with_the_item_list() {
        let registry = registry();
        assert!(!registry.items().is_empty());
        for item in registry.items() {
            assert_eq!(registry.get(&item.identifier), Some(item.runtime_id), "{}", item.identifier);
        }
    }

    #[test]
    fn identifiers_and_runtime_ids_are_unique() {
        let registry = registry();
        let identifiers: HashSet<&str> = registry.items().iter().map(|item| item.identifier.as_str()).collect();
        let ids: HashSet<i16> = registry.items().iter().map(|item| item.runtime_id).collect();
        assert_eq!(identifiers.len(), registry.items().len());
        assert_eq!(ids.len(), registry.items().len());
    }

    #[test]
    fn the_item_packet_lists_the_items_that_have_components() {
        let registry = registry();
        let packet = registry.to_packet();
        assert!(!packet.items.is_empty() && packet.items.len() < registry.items().len());
        let apple = packet.items.iter().find(|item| item.component_item_name == "minecraft:apple").unwrap();
        assert_eq!(Some(apple.runtime_id), registry.get("minecraft:apple"));
    }

    fn child<'a>(value: &'a nbtx::Value, key: &str) -> &'a nbtx::Value {
        let nbtx::Value::Compound(compound) = value else { panic!("{key}: not a compound") };
        compound
            .iter()
            .find(|(name, _)| name.as_slice() == key.as_bytes())
            .map(|(_, value)| value)
            .unwrap_or_else(|| panic!("{key} is missing"))
    }

    #[test]
    fn a_golden_apple_can_always_be_eaten() {
        let registry = registry();
        let packet = registry.to_packet();
        let apple = packet.items.iter().find(|item| item.component_item_name == "minecraft:golden_apple").unwrap();
        let components = child(&apple.component_data, "components");
        let food = child(components, "minecraft:food");
        assert!(matches!(child(food, "can_always_eat"), nbtx::Value::Byte(1)));
    }

    #[test]
    fn creative_blocks_are_known_to_the_block_registry() {
        let mut blocks = chorus_block::block_registry::BlockRegistry::new();
        blocks.register_all(chorus_block::r#impl::DEFINITIONS.iter().copied());
        let registry = registry();
        let unknown: Vec<i32> = registry
            .creative
            .contents
            .iter()
            .map(|entry| entry.item_instance.block_runtime_id)
            .filter(|id| *id != 0 && blocks.get_permutation(*id).is_none())
            .collect();
        assert!(unknown.is_empty(), "{} creative blocks are not in the block registry, first {:?}", unknown.len(), unknown.first());
    }

    #[test]
    fn the_creative_list_is_not_empty_and_has_sequential_net_ids() {
        let registry = registry();
        let packet = registry.to_creative_packet();
        assert!(!packet.contents.is_empty());
        assert!(!packet.groups.is_empty());
        for (index, entry) in packet.contents.iter().enumerate() {
            assert_eq!(entry.creative_net_id, index as u32 + 1);
            assert!((entry.group_id as usize) < packet.groups.len());
        }
    }

    #[test]
    fn creative_entries_point_at_registered_items() {
        let registry = registry();
        let ids: HashSet<i32> = registry.items().iter().map(|item| item.runtime_id as i32).collect();
        for entry in registry.to_creative_packet().contents {
            assert!(ids.contains(&entry.item_instance.id), "creative entry {}", entry.creative_net_id);
        }
    }

    #[test]
    fn an_empty_registry_has_no_items_and_no_creative_entries() {
        let registry = ItemRegistry::new();
        assert!(registry.items().is_empty());
        assert_eq!(registry.get("minecraft:stone"), None);
        assert!(registry.to_creative_packet().contents.is_empty());
    }
}
