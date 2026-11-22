use super::entities::PlacedEntity;
use super::{BoundingBox, Probed, StartProbe, StructureBlocks};
use crate::shared::blocks::{BlockId, Blocks};
use crate::shared::chunk::proto::ProtoChunk;
use crate::shared::tags::Tag;
use chorus_level::dimension_definition::DimensionDefinition;
use rustc_hash::{FxHashMap, FxHashSet};

const SURFACE_LAYERS: i32 = 4;
const DEEPSLATE_BELOW: i32 = 0;
const NAMESPACE_LENGTH: usize = "minecraft:".len();

#[derive(Clone)]
pub struct ColumnHeights {
    pub solid: Box<[i16; 256]>,
    pub top: Box<[i16; 256]>,
    pub min_y: i32,
}

const EMPTY_COLUMN_HEIGHT: i32 = 0;

impl ColumnHeights {
    pub fn lowest_top(&self) -> i32 {
        self.top
            .iter()
            .map(|&top| if top as i32 <= self.min_y { EMPTY_COLUMN_HEIGHT } else { top as i32 })
            .min()
            .unwrap_or(EMPTY_COLUMN_HEIGHT)
    }
}

pub fn column_heights_of(chunk: &ProtoChunk, blocks: &Blocks) -> ColumnHeights {
    let min_y = chunk.min_y;
    let mut solid = Box::new([min_y as i16; 256]);
    let mut top = Box::new([min_y as i16; 256]);
    for column in 0..256 {
        let (cx, cz) = (column & 15, column >> 4);
        let highest = chunk.top(cx, cz);
        let highest_solid = (min_y..highest).rev().find(|&y| blocks.entry(chunk.get(cx, y, cz)).solid).map_or(min_y, |y| y + 1);
        solid[column as usize] = highest_solid as i16;
        top[column as usize] = highest as i16;
    }
    ColumnHeights { solid, top, min_y }
}

pub struct Plan<'a> {
    pub blocks: &'a Blocks,
    pub ids: &'a StructureBlocks,
    columns: &'a dyn Fn(i32, i32) -> Probed<(i32, i32)>,
    overlay: FxHashMap<(i32, i32, i32), BlockId>,
    touched: FxHashSet<(i32, i32)>,
    pub writes: Vec<(i32, i32, i32, BlockId)>,
    pub entities: Vec<PlacedEntity>,
    pub clip: Option<BoundingBox>,
    sea_level: i32,
    definition: DimensionDefinition,
}

pub struct PlanWrites {
    pub writes: Vec<(i32, i32, i32, BlockId)>,
    pub entities: Vec<PlacedEntity>,
}

impl<'a> Plan<'a> {
    pub fn new(blocks: &'a Blocks, ids: &'a StructureBlocks, columns: &'a dyn Fn(i32, i32) -> Probed<(i32, i32)>, probe: &dyn StartProbe) -> Self {
        Self {
            blocks,
            ids,
            columns,
            overlay: FxHashMap::default(),
            touched: FxHashSet::default(),
            writes: Vec::new(),
            entities: Vec::new(),
            clip: None,
            sea_level: probe.sea_level(),
            definition: probe.definition(),
        }
    }

    pub fn definition(&self) -> DimensionDefinition {
        self.definition
    }

    fn in_world(&self, y: i32) -> bool {
        (self.definition.min_y..self.definition.top()).contains(&y)
    }

    pub fn sea_level(&self) -> i32 {
        self.sea_level
    }

    fn ground(&self, x: i32, y: i32, z: i32) -> Probed<BlockId> {
        let (solid, top) = (self.columns)(x, z)?;
        Ok(if y < solid {
            if y >= solid - SURFACE_LAYERS {
                self.ids.dirt
            } else if y < DEEPSLATE_BELOW {
                self.ids.deepslate
            } else {
                self.ids.stone
            }
        } else if y < top {
            self.ids.water
        } else {
            self.ids.air
        })
    }

    pub fn get(&self, x: i32, y: i32, z: i32) -> Probed<BlockId> {
        if !self.in_world(y) {
            return Ok(self.ids.air);
        }
        match self.overlay.get(&(x, y, z)) {
            Some(block) => Ok(*block),
            None => self.ground(x, y, z),
        }
    }

    pub fn world(&self, probe: &dyn StartProbe, x: i32, y: i32, z: i32) -> Probed<BlockId> {
        if !self.in_world(y) {
            return Ok(self.ids.air);
        }
        if let Some(block) = self.overlay.get(&(x, y, z)) {
            return Ok(*block);
        }
        match probe.block(x, y, z)? {
            Some(block) => Ok(block),
            None => self.ground(x, y, z),
        }
    }

    pub fn staged(&self, probe: &dyn StartProbe, x: i32, y: i32, z: i32) -> Probed<BlockId> {
        if !self.in_world(y) {
            return Ok(self.ids.air);
        }
        if let Some(block) = self.overlay.get(&(x, y, z)) {
            return Ok(*block);
        }
        match probe.staged_block(x, y, z)? {
            Some(block) => Ok(block),
            None => self.ground(x, y, z),
        }
    }

    pub fn set(&mut self, x: i32, y: i32, z: i32, block: BlockId) {
        if !self.in_world(y) {
            return;
        }
        if self.clip.is_some_and(|clip| !clip.contains(x, y, z)) {
            return;
        }
        self.overlay.insert((x, y, z), block);
        self.touched.insert((x, z));
        self.writes.push((x, y, z, block));
    }

    pub fn short_name(&self, block: BlockId) -> &str {
        &self.blocks.name_of(block)[NAMESPACE_LENGTH..]
    }

    pub fn is_solid(&self, block: BlockId) -> bool {
        self.blocks.entry(block).solid
    }

    pub fn is_water(&self, block: BlockId) -> bool {
        block == self.ids.water
    }

    pub fn is_leaves(&self, block: BlockId) -> bool {
        self.blocks.is(block, Tag::Leaves)
    }

    fn scan_down(&self, x: i32, z: i32, test: impl Fn(BlockId) -> bool) -> Probed<i32> {
        for y in (self.definition.min_y..self.definition.top()).rev() {
            if test(self.get(x, y, z)?) {
                return Ok(y + 1);
            }
        }
        Ok(self.definition.min_y)
    }

    pub fn blocking_height(&self, x: i32, z: i32, water: bool) -> Probed<i32> {
        if !self.touched.contains(&(x, z)) {
            let (solid, top) = (self.columns)(x, z)?;
            return Ok(if water { top.max(solid) } else { solid });
        }
        self.scan_down(x, z, |block| self.is_solid(block) || water && self.is_water(block))
    }

    pub fn floor_height(&self, x: i32, z: i32) -> Probed<i32> {
        self.blocking_height(x, z, false)
    }

    pub fn finish(self) -> PlanWrites {
        PlanWrites {
            writes: self.writes,
            entities: self.entities,
        }
    }
}
