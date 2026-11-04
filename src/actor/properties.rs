use crate::actor::viewers::{Viewers, send_to_viewers};
use crate::entity::components::actor_id::ActorId;
use crate::network::BedrockProtocol;
use crate::network::session::Session;
use crate::network::session::state::SessionState;
use bedrock::protocol::v662::packets::{SetActorDataPacket, SyncActorPropertyPacket};
use bedrock::protocol::v662::types::{ActorRuntimeID, FloatEntry, IntEntry, PropertySyncData};
use bevy_ecs::prelude::{Changed, Component, Entity, Query, Resource};
use std::collections::{BTreeMap, HashMap};

pub const PLAYER_ENTITY_TYPE: &str = "minecraft:player";

#[derive(Clone, Debug, PartialEq)]
pub enum PropertyKind {
    Int { min: i32, max: i32 },
    Float { min: f32, max: f32 },
    Bool,
    Enum(Vec<String>),
}

impl PropertyKind {
    fn network_type(&self) -> i32 {
        match self {
            Self::Int { .. } => 0,
            Self::Float { .. } => 1,
            Self::Bool => 2,
            Self::Enum(_) => 3,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PropertyDefinition {
    pub name: String,
    pub kind: PropertyKind,
}

impl PropertyDefinition {
    pub fn int(name: impl Into<String>, min: i32, max: i32) -> Self {
        Self { name: name.into(), kind: PropertyKind::Int { min, max } }
    }

    pub fn float(name: impl Into<String>, min: f32, max: f32) -> Self {
        Self { name: name.into(), kind: PropertyKind::Float { min, max } }
    }

    pub fn bool(name: impl Into<String>) -> Self {
        Self { name: name.into(), kind: PropertyKind::Bool }
    }

    pub fn enumeration(name: impl Into<String>, values: Vec<String>) -> Self {
        Self { name: name.into(), kind: PropertyKind::Enum(values) }
    }

    fn to_nbt(&self) -> nbtx::Compound {
        let mut tag = nbtx::Compound::new();
        tag.insert("name".into(), nbtx::Value::String(self.name.as_str().into()));
        tag.insert("type".into(), nbtx::Value::Int(self.kind.network_type()));
        match &self.kind {
            PropertyKind::Int { min, max } => {
                tag.insert("min".into(), nbtx::Value::Int(*min));
                tag.insert("max".into(), nbtx::Value::Int(*max));
            }
            PropertyKind::Float { min, max } => {
                tag.insert("min".into(), nbtx::Value::Float(*min));
                tag.insert("max".into(), nbtx::Value::Float(*max));
            }
            PropertyKind::Bool => {}
            PropertyKind::Enum(values) => {
                let values = values.iter().map(|value| value.as_str().into()).collect();
                tag.insert("enum".into(), nbtx::Value::List(nbtx::ValueList::String(values)));
            }
        }
        tag
    }
}

#[derive(Resource, Default, Clone, Debug)]
pub struct ActorPropertyRegistry {
    types: Vec<(String, Vec<PropertyDefinition>)>,
}

impl ActorPropertyRegistry {
    pub fn register(&mut self, entity_type: impl Into<String>, definitions: Vec<PropertyDefinition>) {
        let entity_type = entity_type.into();
        match self.types.iter_mut().find(|(existing, _)| *existing == entity_type) {
            Some((_, existing)) => *existing = definitions,
            None => self.types.push((entity_type, definitions)),
        }
    }

    pub fn definitions(&self, entity_type: &str) -> Option<&[PropertyDefinition]> {
        self.types.iter().find(|(existing, _)| existing == entity_type).map(|(_, definitions)| definitions.as_slice())
    }

    pub fn index_of(&self, entity_type: &str, name: &str) -> Option<u32> {
        self.definitions(entity_type)?.iter().position(|definition| definition.name == name).map(|index| index as u32)
    }

    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

    pub fn network_data(&self, entity_type: &str) -> Option<HashMap<String, nbtx::Value>> {
        let definitions = self.definitions(entity_type).filter(|definitions| !definitions.is_empty())?;
        let mut root: HashMap<String, nbtx::Value> = HashMap::new();
        root.insert("type".to_string(), nbtx::Value::String(entity_type.into()));
        let properties = definitions.iter().map(PropertyDefinition::to_nbt).collect();
        root.insert("properties".to_string(), nbtx::Value::List(nbtx::ValueList::Compound(properties)));
        Some(root)
    }

    pub fn sync_packets(&self) -> Vec<BedrockProtocol> {
        self.types
            .iter()
            .filter(|(entity_type, _)| entity_type != PLAYER_ENTITY_TYPE)
            .filter_map(|(entity_type, _)| self.network_data(entity_type))
            .map(|property_data| BedrockProtocol::SyncActorPropertyPacket(SyncActorPropertyPacket { property_data }.into()))
            .collect()
    }
}

#[derive(Component, Default, Clone, Debug, PartialEq)]
pub struct ActorProperties {
    ints: BTreeMap<u32, i32>,
    floats: BTreeMap<u32, f32>,
}

impl ActorProperties {
    pub fn set_int(&mut self, index: u32, value: i32) -> bool {
        self.ints.insert(index, value) != Some(value)
    }

    pub fn set_float(&mut self, index: u32, value: f32) -> bool {
        self.floats.insert(index, value) != Some(value)
    }

    pub fn get_int(&self, index: u32) -> Option<i32> {
        self.ints.get(&index).copied()
    }

    pub fn get_float(&self, index: u32) -> Option<f32> {
        self.floats.get(&index).copied()
    }

    pub fn sync_data(&self) -> PropertySyncData {
        PropertySyncData {
            int_entries_list: self.ints.iter().map(|(&property_index, &data)| IntEntry { property_index, data }).collect(),
            float_entries_list: self.floats.iter().map(|(&property_index, &data)| FloatEntry { property_index, data }).collect(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.ints.is_empty() && self.floats.is_empty()
    }
}

fn data_packet(actor: &ActorId, properties: &ActorProperties) -> BedrockProtocol {
    BedrockProtocol::SetActorDataPacket(
        SetActorDataPacket {
            target_runtime_id: ActorRuntimeID(actor.runtime_id),
            actor_data: vec![],
            synced_properties: properties.sync_data(),
            tick: 0,
        }
        .into(),
    )
}

pub fn broadcast_property_changes(changed: Query<(Entity, &ActorId, &ActorProperties, Option<&Viewers>), Changed<ActorProperties>>, mut sessions: Query<&mut Session>) {
    for (entity, actor, properties, viewers) in &changed {
        if properties.is_empty() {
            continue;
        }
        let packet = data_packet(actor, properties);
        if let Some(viewers) = viewers {
            send_to_viewers(viewers, &mut sessions, &packet);
        }
        let already_sent = viewers.is_some_and(|viewers| viewers.0.contains(&entity));
        if !already_sent && let Ok(mut session) = sessions.get_mut(entity) && session.get_state() == SessionState::Play {
            session.send(packet);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> ActorPropertyRegistry {
        let mut registry = ActorPropertyRegistry::default();
        registry.register("test:thing", vec![PropertyDefinition::int("a:head", 0, 100), PropertyDefinition::float("a:glow", 0.0, 1.0), PropertyDefinition::bool("a:flag")]);
        registry
    }

    #[test]
    fn indices_follow_definition_order() {
        let registry = registry();
        assert_eq!(registry.index_of("test:thing", "a:head"), Some(0));
        assert_eq!(registry.index_of("test:thing", "a:flag"), Some(2));
        assert_eq!(registry.index_of("test:thing", "missing"), None);
        assert_eq!(registry.index_of("test:other", "a:head"), None);
    }

    #[test]
    fn registering_a_type_again_replaces_it() {
        let mut registry = registry();
        registry.register("test:thing", vec![PropertyDefinition::bool("only")]);
        assert_eq!(registry.definitions("test:thing").unwrap().len(), 1);
        assert_eq!(registry.sync_packets().len(), 1);
    }

    fn field<'a>(tag: &'a nbtx::Compound, key: &str) -> Option<&'a nbtx::Value> {
        tag.iter().find(|(name, _)| name.to_string() == key).map(|(_, value)| value)
    }

    #[test]
    fn sync_packet_uses_the_network_layout() {
        let packets = registry().sync_packets();
        assert_eq!(packets.len(), 1);
        let BedrockProtocol::SyncActorPropertyPacket(packet) = &packets[0] else { panic!("wrong packet") };
        assert!(matches!(packet.property_data.get("type"), Some(nbtx::Value::String(name)) if name.to_string() == "test:thing"));
        let Some(nbtx::Value::List(nbtx::ValueList::Compound(properties))) = packet.property_data.get("properties") else { panic!("no property list") };
        assert_eq!(properties.len(), 3);
        assert!(matches!(field(&properties[0], "type"), Some(nbtx::Value::Int(0))));
        assert!(matches!(field(&properties[0], "max"), Some(nbtx::Value::Int(100))));
        assert!(matches!(field(&properties[1], "type"), Some(nbtx::Value::Int(1))));
        assert!(matches!(field(&properties[2], "type"), Some(nbtx::Value::Int(2))));
    }

    #[test]
    fn setting_the_same_value_reports_no_change() {
        let mut properties = ActorProperties::default();
        assert!(properties.set_int(0, 5));
        assert!(!properties.set_int(0, 5));
        assert!(properties.set_int(0, 6));
        assert_eq!(properties.get_int(0), Some(6));
    }

    #[test]
    fn sync_data_lists_every_value() {
        let mut properties = ActorProperties::default();
        properties.set_int(2, 1);
        properties.set_int(0, 7);
        properties.set_float(1, 0.5);
        let data = properties.sync_data();
        assert_eq!(data.int_entries_list.iter().map(|entry| (entry.property_index, entry.data)).collect::<Vec<_>>(), vec![(0, 7), (2, 1)]);
        assert_eq!(data.float_entries_list.len(), 1);
    }

    #[test]
    fn the_player_type_is_left_out_of_the_sync_packets() {
        let mut registry = registry();
        registry.register(PLAYER_ENTITY_TYPE, vec![PropertyDefinition::int("a:head", 0, 100)]);
        assert_eq!(registry.sync_packets().len(), 1);
        let data = registry.network_data(PLAYER_ENTITY_TYPE).expect("player data");
        assert!(matches!(data.get("type"), Some(nbtx::Value::String(name)) if name.to_string() == PLAYER_ENTITY_TYPE));
        assert!(registry.network_data("test:missing").is_none());
    }
}
