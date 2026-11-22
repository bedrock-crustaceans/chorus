use nbtx::Compound;

pub const FULL_CUBE: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StateKind {
    Block,
    Jigsaw,
    StructureBlock,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PaletteEntry {
    pub name: String,
    pub states: String,
    pub waterlogged: bool,
}

impl PaletteEntry {
    pub fn new(name: &str, states: &str, waterlogged: bool) -> Self {
        Self {
            name: name.to_owned(),
            states: states.to_owned(),
            waterlogged,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TemplateBlock {
    pub x: u8,
    pub y: u16,
    pub z: u8,
    pub state: u16,
}

#[derive(Debug, Clone)]
pub struct TemplateEntity {
    pub identifier: String,
    pub position: [f64; 3],
    pub nbt: Compound,
}

#[derive(Debug)]
pub struct Template {
    pub name: String,
    pub size: (i32, i32, i32),
    pub blocks: Vec<TemplateBlock>,
    pub markers: Vec<(usize, String)>,
    pub loot_chests: Vec<usize>,
    pub jigsaws: Vec<Jigsaw>,
    pub entities: Vec<TemplateEntity>,
}

#[derive(Debug, Clone)]
pub struct Jigsaw {
    pub block: usize,
    pub name: String,
    pub target: String,
    pub pool: String,
    pub joint: String,
    pub orientation: String,
    pub final_state: u16,
    pub selection_priority: i32,
    pub placement_priority: i32,
}
