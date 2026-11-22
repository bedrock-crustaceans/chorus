use nbtx::{Compound, Value, ValueList};
use std::collections::{BTreeMap, HashMap};

use super::template::{Jigsaw, TemplateBlock, TemplateEntity};
use std::error::Error;
use std::fmt;

const NAMESPACE: &str = "minecraft:";
const ENTITY_ID_KEYS: [&str; 2] = ["identifier", "id"];
const ENTITY_POSITION_KEYS: [&str; 2] = ["Pos", "pos"];
const JIGSAW: &str = "jigsaw";
const STRUCTURE_BLOCK: &str = "structure_block";
const CHEST_ENTITY: &str = "minecraft:chest";
const AIR: &str = "air";
const DATA_TYPE: &str = "data";
const JIGSAW_FACING: [&str; 6] = ["down", "up", "north", "south", "west", "east"];
const JIGSAW_TOP: [&str; 4] = ["north", "east", "south", "west"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadError {
    Nbt(String),
    Missing { field: &'static str },
    WrongType { field: &'static str, expected: &'static str, found: String },
    Invalid { field: &'static str, reason: &'static str },
}

impl fmt::Display for ReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nbt(message) => write!(formatter, "unreadable nbt: {message}"),
            Self::Missing { field } => write!(formatter, "missing field `{field}`"),
            Self::WrongType { field, expected, found } => write!(formatter, "field `{field}` should be {expected}, found {found}"),
            Self::Invalid { field, reason } => write!(formatter, "field `{field}` is invalid: {reason}"),
        }
    }
}

impl Error for ReadError {}

pub struct JigsawData {
    pub name: String,
    pub target: String,
    pub pool: String,
    pub joint: String,
    pub orientation: String,
    pub final_state: BlockState,
    pub selection_priority: i32,
    pub placement_priority: i32,
}

pub enum Special {
    None,
    Jigsaw(JigsawData),
    StructureBlock(Option<String>),
    LootChest,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct BlockState {
    pub name: String,
    pub states: String,
    pub waterlogged: bool,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum LocalState {
    Block(BlockState),
    Jigsaw,
    StructureBlock,
}

pub struct Parsed {
    pub size: [u16; 3],
    pub palette: Vec<LocalState>,
    pub blocks: Vec<TemplateBlock>,
    pub markers: Vec<(usize, String)>,
    pub loot_chests: Vec<usize>,
    pub jigsaws: Vec<Jigsaw>,
    pub entities: Vec<TemplateEntity>,
}

#[derive(Default)]
struct Assembly {
    palette: Vec<LocalState>,
    index: HashMap<LocalState, u16>,
    blocks: Vec<TemplateBlock>,
    markers: Vec<(usize, String)>,
    loot_chests: Vec<usize>,
    jigsaws: Vec<Jigsaw>,
}

impl Assembly {
    fn local(&mut self, state: LocalState) -> Result<u16, ReadError> {
        if let Some(&known) = self.index.get(&state) {
            return Ok(known);
        }
        let next = u16::try_from(self.palette.len()).map_err(|_| ReadError::Invalid {
            field: "palette",
            reason: "more block states than a template can index",
        })?;
        self.index.insert(state.clone(), next);
        self.palette.push(state);
        Ok(next)
    }

    fn entry_state(&mut self, name: &str, states: &BTreeMap<String, String>, waterlogged: bool) -> Result<u16, ReadError> {
        let state = match name {
            JIGSAW => LocalState::Jigsaw,
            STRUCTURE_BLOCK => LocalState::StructureBlock,
            _ => LocalState::Block(BlockState {
                name: name.to_owned(),
                states: join_states(states),
                waterlogged,
            }),
        };
        self.local(state)
    }

    fn push(&mut self, (x, y, z): (u8, u16, u8), local: u16, special: Special) -> Result<(), ReadError> {
        let index = self.blocks.len();
        self.blocks.push(TemplateBlock { x, y, z, state: local });
        match special {
            Special::Jigsaw(data) => {
                let final_state = self.local(LocalState::Block(data.final_state))?;
                self.jigsaws.push(Jigsaw {
                    block: index,
                    name: data.name,
                    target: data.target,
                    pool: data.pool,
                    joint: data.joint,
                    orientation: data.orientation,
                    final_state,
                    selection_priority: data.selection_priority,
                    placement_priority: data.placement_priority,
                });
            }
            Special::StructureBlock(Some(data)) => self.markers.push((index, data)),
            Special::LootChest => self.loot_chests.push(index),
            Special::StructureBlock(None) | Special::None => {}
        }
        Ok(())
    }

    fn finish(self, size: [u16; 3], entities: Vec<TemplateEntity>) -> Parsed {
        Parsed {
            size,
            palette: self.palette,
            blocks: self.blocks,
            markers: self.markers,
            loot_chests: self.loot_chests,
            jigsaws: self.jigsaws,
            entities,
        }
    }
}

fn field<'a>(compound: &'a Compound, name: &str) -> Option<&'a Value> {
    compound.get(name.as_bytes())
}

fn required<'a>(compound: &'a Compound, name: &'static str) -> Result<&'a Value, ReadError> {
    field(compound, name).ok_or(ReadError::Missing { field: name })
}

fn wrong_type(field: &'static str, expected: &'static str, found: &Value) -> ReadError {
    let debug = format!("{found:?}");
    let tag = debug.split('(').next().unwrap_or_default().to_owned();
    ReadError::WrongType { field, expected, found: tag }
}

fn text(value: &Value, field: &'static str) -> Result<String, ReadError> {
    match value {
        Value::String(bytes) => std::str::from_utf8(&bytes[..]).map(str::to_owned).map_err(|_| ReadError::Invalid { field, reason: "not utf-8" }),
        other => Err(wrong_type(field, "a string", other)),
    }
}

fn integer(value: &Value, field: &'static str) -> Result<i64, ReadError> {
    match value {
        Value::Byte(value) => Ok(i64::from(*value)),
        Value::Short(value) => Ok(i64::from(*value)),
        Value::Int(value) => Ok(i64::from(*value)),
        Value::Long(value) => Ok(*value),
        other => Err(wrong_type(field, "an integer", other)),
    }
}

fn state_value(value: &Value, field: &'static str) -> Result<String, ReadError> {
    match value {
        Value::String(_) => text(value, field),
        other => integer(other, field).map(|number| number.to_string()),
    }
}

fn ints(value: &Value, field: &'static str) -> Result<Vec<i32>, ReadError> {
    match value {
        Value::IntArray(items) => Ok(items.clone()),
        Value::List(ValueList::Int(items)) => Ok(items.clone()),
        other => Err(wrong_type(field, "an int list", other)),
    }
}

fn triple(values: &[i32], field: &'static str) -> Result<[i32; 3], ReadError> {
    <[i32; 3]>::try_from(values).map_err(|_| ReadError::Invalid {
        field,
        reason: "expected exactly three values",
    })
}

fn dimensions(root: &Compound) -> Result<[u16; 3], ReadError> {
    let size = triple(&ints(required(root, "size")?, "size")?, "size")?;
    let fit = |value: i32| {
        u16::try_from(value).map_err(|_| ReadError::Invalid {
            field: "size",
            reason: "outside the u16 range",
        })
    };
    Ok([fit(size[0])?, fit(size[1])?, fit(size[2])?])
}

fn compounds<'a>(value: &'a Value, field: &'static str) -> Result<Vec<&'a Compound>, ReadError> {
    match value {
        Value::List(ValueList::Compound(items)) => Ok(items.iter().collect()),
        Value::List(ValueList::End) => Ok(Vec::new()),
        other => Err(wrong_type(field, "a compound list", other)),
    }
}

fn state_map(compound: Option<&Compound>, field: &'static str) -> Result<BTreeMap<String, String>, ReadError> {
    compound
        .into_iter()
        .flatten()
        .map(|(key, value)| {
            let key = std::str::from_utf8(key.as_ref()).map_err(|_| ReadError::Invalid { field, reason: "key is not utf-8" })?;
            Ok((key.to_owned(), state_value(value, field)?))
        })
        .collect()
}

fn short_name(name: &str) -> String {
    name.strip_prefix(NAMESPACE).unwrap_or(name).to_owned()
}

pub(crate) fn join_states(states: &BTreeMap<String, String>) -> String {
    states.iter().map(|(key, value)| format!("{key}={value}")).collect::<Vec<_>>().join(",")
}

pub(crate) fn split_states(states: &str) -> BTreeMap<String, String> {
    states
        .split(',')
        .filter(|pair| !pair.is_empty())
        .filter_map(|pair| pair.split_once('='))
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect()
}

fn orientation(states: &BTreeMap<String, String>) -> String {
    let facing = states
        .get("facing_direction")
        .and_then(|value| value.parse::<usize>().ok())
        .and_then(|index| JIGSAW_FACING.get(index))
        .copied()
        .unwrap_or("north");
    let rotation = states.get("rotation").and_then(|value| value.parse::<usize>().ok()).unwrap_or(0);
    let top = if matches!(facing, "down" | "up") { JIGSAW_TOP[rotation % 4] } else { "up" };
    format!("{facing}_{top}")
}

fn parse_final_state(text: &str) -> BlockState {
    let (name, rest) = text.split_once('[').unwrap_or((text, ""));
    BlockState {
        name: short_name(name),
        states: join_states(&split_states(rest.trim_end_matches(']'))),
        waterlogged: false,
    }
}

fn is_loot_chest(entity: Option<&Compound>) -> Result<bool, ReadError> {
    let Some(entity) = entity else { return Ok(false) };
    let identifier = field(entity, "id").map(|value| text(value, "id")).transpose()?;
    Ok(identifier.as_deref() == Some(CHEST_ENTITY) && field(entity, "LootTableSeed").is_none())
}

fn special(name: &str, states: &BTreeMap<String, String>, entity: Option<&Compound>) -> Result<Special, ReadError> {
    let text_of = |keys: &[&'static str]| -> Result<Option<String>, ReadError> {
        keys.iter()
            .find_map(|key| entity.and_then(|entity| field(entity, key)).map(|value| (*key, value)))
            .map(|(key, value)| text(value, key))
            .transpose()
    };
    let number = |key: &'static str| -> Result<i32, ReadError> { entity.and_then(|entity| field(entity, key)).map_or(Ok(0), |value| integer(value, key).map(|number| number as i32)) };
    Ok(match name {
        JIGSAW => Special::Jigsaw(JigsawData {
            name: text_of(&["name"])?.unwrap_or_default(),
            target: text_of(&["target"])?.unwrap_or_default(),
            pool: text_of(&["pool", "target_pool"])?.unwrap_or_default(),
            joint: text_of(&["joint"])?.unwrap_or_default(),
            orientation: orientation(states),
            final_state: parse_final_state(&text_of(&["final_state"])?.unwrap_or_else(|| "minecraft:air".to_owned())),
            selection_priority: number("selection_priority")?,
            placement_priority: number("placement_priority")?,
        }),
        STRUCTURE_BLOCK => Special::StructureBlock(if states.get("structure_block_type").is_some_and(|kind| kind == DATA_TYPE) {
            Some(text_of(&["metadata"])?.unwrap_or_default())
        } else {
            None
        }),
        _ if is_loot_chest(entity)? => Special::LootChest,
        _ => Special::None,
    })
}

fn palette_entry(entry: &Compound, name_key: &'static str, states_key: &'static str) -> Result<(String, BTreeMap<String, String>), ReadError> {
    let name = short_name(&text(required(entry, name_key)?, name_key)?);
    let states = state_map(field(entry, states_key).and_then(Value::as_compound), states_key)?;
    Ok((name, states))
}

fn palette_entries(root: &Compound) -> Result<Vec<(String, BTreeMap<String, String>)>, ReadError> {
    let Some(palette) = field(root, "palette") else { return Ok(Vec::new()) };
    compounds(palette, "palette")?.into_iter().map(|entry| palette_entry(entry, "Name", "Properties")).collect()
}

fn position_of(entity: &Compound) -> [f64; 3] {
    let listed: Vec<f64> = match ENTITY_POSITION_KEYS.iter().find_map(|key| field(entity, key)) {
        Some(Value::List(ValueList::Float(items))) => items.iter().map(|item| f64::from(*item)).collect(),
        Some(Value::List(ValueList::Double(items))) => items.clone(),
        Some(Value::List(ValueList::Int(items))) => items.iter().map(|item| f64::from(*item)).collect(),
        _ => Vec::new(),
    };
    let at = |index: usize| listed.get(index).copied().unwrap_or(0.0);
    [at(0), at(1), at(2)]
}

fn entity_of(entity: &Compound) -> Result<TemplateEntity, ReadError> {
    let identifier = match ENTITY_ID_KEYS.iter().find_map(|key| field(entity, key)) {
        Some(value @ Value::String(_)) => text(value, "identifier")?,
        _ => String::new(),
    };
    Ok(TemplateEntity {
        identifier,
        position: position_of(entity),
        nbt: entity.clone(),
    })
}

fn listed_entities(root: &Compound) -> Result<Vec<TemplateEntity>, ReadError> {
    let Some(list) = field(root, "entities") else { return Ok(Vec::new()) };
    compounds(list, "entities")?
        .into_iter()
        .map(|entry| {
            let mut entity = match field(entry, "nbt").and_then(Value::as_compound) {
                Some(nbt) => entity_of(nbt)?,
                None => entity_of(entry)?,
            };
            if let Some(position) = ENTITY_POSITION_KEYS.iter().find_map(|key| field(entry, key)) {
                let mut carried = Compound::new();
                carried.insert(ENTITY_POSITION_KEYS[0].as_bytes().into(), position.clone());
                entity.position = position_of(&carried);
            }
            Ok(entity)
        })
        .collect()
}

fn grid_entities(root: &Compound, structure: &Compound) -> Result<Vec<TemplateEntity>, ReadError> {
    let Some(list) = field(structure, "entities") else { return Ok(Vec::new()) };
    let origin = field(root, "structure_world_origin")
        .map(|value| ints(value, "structure_world_origin"))
        .transpose()?
        .unwrap_or_default();
    let shift = |index: usize| origin.get(index).copied().map_or(0.0, f64::from);
    compounds(list, "entities")?
        .into_iter()
        .map(|entry| {
            let mut entity = entity_of(entry)?;
            for (axis, coordinate) in entity.position.iter_mut().enumerate() {
                *coordinate -= shift(axis);
            }
            Ok(entity)
        })
        .collect()
}

struct PreparedState {
    name: String,
    states: BTreeMap<String, String>,
    waterlogged: bool,
}

struct ListedBuilder {
    palette: Vec<PreparedState>,
    assembly: Assembly,
    locals: Vec<Option<u16>>,
}

impl ListedBuilder {
    fn new(entries: Vec<(String, BTreeMap<String, String>)>) -> Self {
        let palette: Vec<PreparedState> = entries
            .into_iter()
            .map(|(name, mut states)| {
                let waterlogged = states.remove("waterlogged").is_some_and(|value| value == "true");
                PreparedState { name, states, waterlogged }
            })
            .collect();
        let locals = vec![None; palette.len()];
        Self {
            palette,
            assembly: Assembly::default(),
            locals,
        }
    }

    fn block(&mut self, position: (u8, u16, u8), state_index: usize, entity: Option<&Compound>) -> Result<(), ReadError> {
        let prepared = self.palette.get(state_index).ok_or(ReadError::Invalid {
            field: "state",
            reason: "index outside the palette",
        })?;
        let local = match self.locals[state_index] {
            Some(local) => local,
            None => {
                let local = self.assembly.entry_state(&prepared.name, &prepared.states, prepared.waterlogged)?;
                self.locals[state_index] = Some(local);
                local
            }
        };
        let special = if entity.is_some() || prepared.name == JIGSAW || prepared.name == STRUCTURE_BLOCK {
            special(&prepared.name, &prepared.states, entity)?
        } else {
            Special::None
        };
        self.assembly.push(position, local, special)
    }

    fn finish(self, size: [u16; 3], entities: Vec<TemplateEntity>) -> Parsed {
        self.assembly.finish(size, entities)
    }
}

fn root_compound(bytes: &[u8]) -> Result<Compound, ReadError> {
    let root: Value = nbtx::from_le_bytes(&mut &*bytes).map_err(|error| ReadError::Nbt(error.to_string()))?;
    root.as_compound().cloned().ok_or(ReadError::WrongType {
        field: "root",
        expected: "a compound",
        found: "another tag".to_owned(),
    })
}

pub fn load_listed(bytes: &[u8]) -> Result<Parsed, ReadError> {
    let root = root_compound(bytes)?;
    let size = dimensions(&root)?;
    let mut builder = ListedBuilder::new(palette_entries(&root)?);
    for block in compounds(required(&root, "blocks")?, "blocks")? {
        let [x, y, z] = triple(&ints(required(block, "pos")?, "pos")?, "pos")?;
        let state_index = integer(required(block, "state")?, "state")? as usize;
        let entity = field(block, "nbt").and_then(Value::as_compound);
        builder.block((x as u8, y as u16, z as u8), state_index, entity)?;
    }
    Ok(builder.finish(size, listed_entities(&root)?))
}

pub fn load_packed(listed: &bedrock_vanilla::Listed<'_>) -> Result<Parsed, ReadError> {
    let entries = listed
        .palette
        .iter()
        .map(|entry| (short_name(&entry.name), entry.properties.iter().cloned().collect::<BTreeMap<String, String>>()))
        .collect();
    let mut builder = ListedBuilder::new(entries);
    for block in &listed.blocks {
        builder.block((block.x, block.y, block.z), usize::from(block.state), block.nbt.as_ref())?;
    }
    Ok(builder.finish(listed.size, Vec::new()))
}

fn grid_layer(structure: &Compound) -> Result<Vec<i32>, ReadError> {
    match required(structure, "block_indices")? {
        Value::List(ValueList::List(layers)) => match layers.first().ok_or(ReadError::Invalid {
            field: "block_indices",
            reason: "no block layer",
        })? {
            ValueList::Int(values) => Ok(values.clone()),
            _ => Err(ReadError::Invalid {
                field: "block_indices",
                reason: "the block layer is not an int list",
            }),
        },
        other => Err(wrong_type("block_indices", "a list of layers", other)),
    }
}

pub fn load_grid(bytes: &[u8]) -> Result<Parsed, ReadError> {
    let root = root_compound(bytes)?;
    let size = dimensions(&root)?;
    let structure = required(&root, "structure")?.as_compound().ok_or(ReadError::Invalid {
        field: "structure",
        reason: "not a compound",
    })?;
    let palette = field(structure, "palette")
        .and_then(Value::as_compound)
        .and_then(|palette| field(palette, "default"))
        .and_then(Value::as_compound)
        .ok_or(ReadError::Missing { field: "default palette" })?;
    let entries = match field(palette, "block_palette") {
        Some(list) => compounds(list, "block_palette")?
            .into_iter()
            .map(|entry| palette_entry(entry, "name", "states"))
            .collect::<Result<Vec<_>, _>>()?,
        None => Vec::new(),
    };
    let entity_data = field(palette, "block_position_data").and_then(Value::as_compound);
    let cells = grid_layer(structure)?;
    let (size_y, size_z) = (usize::from(size[1]), usize::from(size[2]));
    let layer_stride = size_y * size_z;
    if layer_stride == 0 {
        return Err(ReadError::Invalid {
            field: "size",
            reason: "a dimension is zero",
        });
    }
    let mut assembly = Assembly::default();
    let mut locals: Vec<Option<u16>> = vec![None; entries.len()];
    for (index, state) in cells.iter().enumerate() {
        if *state < 0 {
            continue;
        }
        let state = *state as usize;
        let (name, states) = entries.get(state).ok_or(ReadError::Invalid {
            field: "block_indices",
            reason: "index outside the block palette",
        })?;
        if name == AIR {
            continue;
        }
        let entity = entity_data
            .and_then(|data| data.get(index.to_string().as_bytes()))
            .and_then(Value::as_compound)
            .and_then(|data| field(data, "block_entity_data"))
            .and_then(Value::as_compound);
        let local = match locals[state] {
            Some(local) => local,
            None => {
                let local = assembly.entry_state(name, states, false)?;
                locals[state] = Some(local);
                local
            }
        };
        let special = if entity.is_some() || name == JIGSAW || name == STRUCTURE_BLOCK {
            special(name, states, entity)?
        } else {
            Special::None
        };
        assembly.push(((index / layer_stride) as u8, (index / size_z % size_y) as u16, (index % size_z) as u8), local, special)?;
    }
    Ok(assembly.finish(size, grid_entities(&root, structure)?))
}

#[cfg(test)]
mod packed_equivalence {
    use super::{Parsed, load_listed, load_packed};

    fn summary(parsed: &Parsed) -> String {
        let blocks: Vec<(u8, u16, u8, u16)> = parsed.blocks.iter().map(|block| (block.x, block.y, block.z, block.state)).collect();
        let palette: Vec<(&str, &str, bool)> = parsed
            .palette
            .iter()
            .map(|state| match state {
                super::LocalState::Block(block) => (block.name.as_str(), block.states.as_str(), block.waterlogged),
                super::LocalState::Jigsaw => ("jigsaw", "", false),
                super::LocalState::StructureBlock => ("structure_block", "", false),
            })
            .collect();
        format!(
            "{:?} {palette:?} {blocks:?} {:?} {:?} {:?} {}",
            parsed.size,
            parsed.markers,
            parsed.loot_chests,
            parsed.jigsaws,
            parsed.entities.len()
        )
    }

    #[test]
    fn the_typed_path_gives_what_the_nbt_path_gives_for_every_packed_template() {
        let pack = bedrock_vanilla::structures();
        let mut compared = 0;
        for name in pack.names() {
            let Ok(Some(bedrock_vanilla::Entry::Listed(listed))) = pack.get(name).transpose() else { continue };
            let typed = load_packed(&listed).expect("the typed path parses");
            let legacy = load_listed(&bedrock_vanilla::Entry::Listed(listed).to_bytes().expect("the template encodes")).expect("the nbt path parses");
            assert_eq!(summary(&typed), summary(&legacy), "{name}");
            compared += 1;
        }
        assert_eq!(compared, 1285);
    }
}

#[cfg(test)]
mod malformed {
    use super::{ReadError, load_grid, load_listed};
    use nbtx::{Compound, Value, ValueList};

    fn compound(entries: Vec<(&str, Value)>) -> Compound {
        let mut compound = Compound::new();
        for (key, value) in entries {
            compound.insert(key.as_bytes().into(), value);
        }
        compound
    }

    fn bytes(entries: Vec<(&str, Value)>) -> Vec<u8> {
        nbtx::to_le_bytes(&Value::Compound(compound(entries))).expect("the test compound encodes")
    }

    fn ints(values: &[i32]) -> Value {
        Value::List(ValueList::Int(values.to_vec()))
    }

    fn compounds(items: Vec<Compound>) -> Value {
        Value::List(ValueList::Compound(items))
    }

    fn palette_of(name: &str) -> Value {
        compounds(vec![compound(vec![("Name", Value::from(name))])])
    }

    fn block(position: &[i32], state: i32) -> Compound {
        compound(vec![("pos", ints(position)), ("state", Value::Int(state))])
    }

    fn listed(size: Value, blocks: Value) -> Vec<u8> {
        bytes(vec![("size", size), ("palette", palette_of("minecraft:stone")), ("blocks", blocks)])
    }

    fn error_of(result: Result<super::Parsed, ReadError>) -> ReadError {
        match result {
            Ok(_) => panic!("the document was accepted"),
            Err(error) => error,
        }
    }

    #[test]
    fn a_wrong_tag_type_names_the_field() {
        let error = error_of(load_listed(&listed(Value::from("big"), compounds(Vec::new()))));
        assert!(matches!(error, ReadError::WrongType { field: "size", .. }), "{error}");
    }

    #[test]
    fn a_missing_field_is_named() {
        let error = error_of(load_listed(&bytes(vec![("size", ints(&[1, 1, 1]))])));
        assert_eq!(error, ReadError::Missing { field: "blocks" });
    }

    #[test]
    fn a_size_with_two_values_is_rejected() {
        let error = error_of(load_listed(&listed(ints(&[1, 1]), compounds(Vec::new()))));
        assert!(matches!(error, ReadError::Invalid { field: "size", .. }), "{error}");
    }

    #[test]
    fn a_block_state_outside_the_palette_is_rejected() {
        let error = error_of(load_listed(&listed(ints(&[1, 1, 1]), compounds(vec![block(&[0, 0, 0], 7)]))));
        assert!(matches!(error, ReadError::Invalid { field: "state", .. }), "{error}");
    }

    #[test]
    fn a_short_block_position_is_rejected() {
        let error = error_of(load_listed(&listed(ints(&[1, 1, 1]), compounds(vec![block(&[0, 0], 0)]))));
        assert!(matches!(error, ReadError::Invalid { field: "pos", .. }), "{error}");
    }

    #[test]
    fn a_palette_name_of_the_wrong_type_is_rejected() {
        let palette = compounds(vec![compound(vec![("Name", Value::Int(3))])]);
        let document = bytes(vec![("size", ints(&[1, 1, 1])), ("palette", palette), ("blocks", compounds(Vec::new()))]);
        let error = error_of(load_listed(&document));
        assert!(matches!(error, ReadError::WrongType { field: "Name", .. }), "{error}");
    }

    #[test]
    fn a_root_that_is_not_a_compound_is_rejected() {
        let document = nbtx::to_le_bytes(&Value::Int(1)).expect("an int encodes");
        assert!(load_listed(&document).is_err());
    }

    #[test]
    fn a_grid_without_a_structure_is_rejected() {
        let error = error_of(load_grid(&bytes(vec![("size", ints(&[1, 1, 1]))])));
        assert_eq!(error, ReadError::Missing { field: "structure" });
    }

    #[test]
    fn a_grid_with_a_zero_dimension_is_rejected() {
        let palette = compound(vec![("default", Value::Compound(compound(vec![])))]);
        let structure = compound(vec![("palette", Value::Compound(palette))]);
        let document = bytes(vec![("size", ints(&[1, 0, 1])), ("structure", Value::Compound(structure))]);
        assert!(load_grid(&document).is_err());
    }

    #[test]
    fn a_grid_index_layer_of_the_wrong_type_is_rejected() {
        let default = compound(vec![("block_palette", compounds(Vec::new()))]);
        let palette = compound(vec![("default", Value::Compound(default))]);
        let layers = Value::List(ValueList::List(vec![ValueList::Float(vec![1.0])]));
        let structure = compound(vec![("palette", Value::Compound(palette)), ("block_indices", layers)]);
        let document = bytes(vec![("size", ints(&[1, 1, 1])), ("structure", Value::Compound(structure))]);
        let error = error_of(load_grid(&document));
        assert!(matches!(error, ReadError::Invalid { field: "block_indices", .. }), "{error}");
    }
}
