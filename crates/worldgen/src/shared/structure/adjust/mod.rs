mod beard;
mod jigsaw;
mod kernel;
mod shaver;

use super::layout::Grounding;
use super::{BoundingBox, Probed, StartProbe, StructureContext, StructureKind, Structures};
use crate::shared::CHUNK_WIDTH;
use beard::Embankment;
use jigsaw::{Adaptation, ColumnIndex, JigsawAdjustments, Rigid};
use shaver::Shaver;

const V2_START_MARGIN: i32 = 1;
const JUNCTION_WINDOW: i32 = 27;
const CITY_START_REACH: i32 = 11;

const START_REACH: i32 = 10;
const EMBANKMENT_MARGIN: i32 = 12;
const VILLAGE_REACH: i32 = 9;
const COLUMN_COUNT: usize = 256;
const BURY_COLUMN_REACH: i32 = 6;
const BURY_HEIGHT_REACH: i32 = 13;
const BURY_FADE: f32 = 6.0;
const HALF_STEP: f32 = 0.5;

#[derive(Debug, Clone, Copy)]
struct Bury {
    bounds: BoundingBox,
    low: i32,
    high: i32,
}

impl Bury {
    fn new(bounds: BoundingBox) -> Self {
        Self {
            bounds,
            low: bounds.y0 - BURY_HEIGHT_REACH,
            high: bounds.y1.max(bounds.y0) + BURY_HEIGHT_REACH,
        }
    }

    fn distance_outside(&self, x: i32, z: i32) -> (i32, i32) {
        let bounds = &self.bounds;
        (((bounds.x0 - x).max(x - bounds.x1)).max(0), ((bounds.z0 - z).max(z - bounds.z1)).max(0))
    }

    fn reaches_column(&self, x: i32, z: i32) -> bool {
        let (dx, dz) = self.distance_outside(x, z);
        dx <= BURY_COLUMN_REACH && dz <= BURY_COLUMN_REACH
    }

    fn contribution(&self, x: i32, y: i32, z: i32) -> f32 {
        let clamped = y.clamp(self.bounds.y0, self.bounds.y1.max(self.bounds.y0));
        let (dx, dz) = self.distance_outside(x, z);
        let dy = ((y - clamped) as f32 * HALF_STEP) as i32;
        let distance = ((dz * dz + dx * dx + dy * dy) as f64).sqrt() as i32 as f32 / BURY_FADE;
        if distance > 1.0 {
            0.0
        } else if distance > 0.0 {
            1.0 - distance
        } else {
            1.0
        }
    }
}

#[derive(Default)]
pub struct Adjustments {
    buries: Vec<Bury>,
    embankments: Vec<Embankment>,
    column_offsets: Vec<u32>,
    column_entries: Vec<u16>,
    embankment_offsets: Vec<u32>,
    embankment_entries: Vec<u16>,
    shavers: Vec<Shaver>,
    shaver_index: ColumnIndex,
    jigsaw: JigsawAdjustments,
}

pub struct ColumnAdjustments<'a> {
    adjustments: &'a Adjustments,
    column: usize,
    entries: &'a [u16],
    embankments: &'a [u16],
    shavers: &'a [u16],
}

impl Adjustments {
    pub fn is_empty(&self) -> bool {
        self.buries.is_empty() && self.embankments.is_empty() && self.shavers.is_empty() && self.jigsaw.is_empty()
    }

    pub fn column(&self, local_x: i32, local_z: i32) -> ColumnAdjustments<'_> {
        let column = (local_z * CHUNK_WIDTH + local_x) as usize;
        fn slice<'a>(offsets: &[u32], entries: &'a [u16], column: usize) -> &'a [u16] {
            if offsets.is_empty() {
                &[]
            } else {
                &entries[offsets[column] as usize..offsets[column + 1] as usize]
            }
        }
        ColumnAdjustments {
            adjustments: self,
            column,
            entries: slice(&self.column_offsets, &self.column_entries, column),
            embankments: slice(&self.embankment_offsets, &self.embankment_entries, column),
            shavers: self.shaver_index.column(column),
        }
    }

    fn index_columns(&mut self, (chunk_x, chunk_z): (i32, i32)) {
        self.column_offsets.reserve(COLUMN_COUNT + 1);
        self.column_offsets.push(0);
        for column in 0..COLUMN_COUNT as i32 {
            let (x, z) = (chunk_x * CHUNK_WIDTH + column % CHUNK_WIDTH, chunk_z * CHUNK_WIDTH + column / CHUNK_WIDTH);
            for (index, bury) in self.buries.iter().enumerate() {
                if bury.reaches_column(x, z) {
                    self.column_entries.push(index as u16);
                }
            }
            self.column_offsets.push(self.column_entries.len() as u32);
        }
        self.embankment_offsets.reserve(COLUMN_COUNT + 1);
        self.embankment_offsets.push(0);
        for column in 0..COLUMN_COUNT as i32 {
            let (x, z) = (chunk_x * CHUNK_WIDTH + column % CHUNK_WIDTH, chunk_z * CHUNK_WIDTH + column / CHUNK_WIDTH);
            for (index, embankment) in self.embankments.iter().enumerate() {
                if embankment.reaches_column(x, z) {
                    self.embankment_entries.push(index as u16);
                }
            }
            self.embankment_offsets.push(self.embankment_entries.len() as u32);
        }
        let shavers = &self.shavers;
        self.shaver_index = ColumnIndex::build((chunk_x, chunk_z), shavers.len(), |entry, x, z| shavers[entry].reaches_column(x, z));
    }
}

impl ColumnAdjustments<'_> {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty() && self.embankments.is_empty() && self.shavers.is_empty() && self.adjustments.jigsaw.column_is_empty(self.column)
    }

    pub fn apply(&self, x: i32, y: i32, z: i32, density: f32) -> f32 {
        let density = self.adjustments.jigsaw.apply(self.column, x, y, z, density);
        let mut raised = 0.0f32;
        for &index in self.shavers {
            let shaver = &self.adjustments.shavers[index as usize];
            let (low, high) = shaver.heights();
            if low <= y && y <= high {
                raised += shaver.contribution(x, y, z);
            }
        }
        for &index in self.embankments {
            let embankment = &self.adjustments.embankments[index as usize];
            let (low, high) = embankment.heights();
            if low <= y && y <= high {
                raised += embankment.contribution(x, y, z);
            }
        }
        if density + raised > 0.0 {
            return raised + density;
        }
        let mut buried = raised;
        for &index in self.entries {
            let bury = &self.adjustments.buries[index as usize];
            if bury.low <= y && y <= bury.high {
                buried += bury.contribution(x, y, z);
            }
        }
        buried + density
    }
}

impl Structures {
    pub fn adjustments(&self, probe: &dyn StartProbe, chunk: (i32, i32)) -> Probed<Adjustments> {
        let context = StructureContext { structures: self, probe };
        let mut adjustments = Adjustments::default();
        for &index in &self.adjust_order {
            self.builders[index].adjust(&context, chunk, &mut adjustments)?;
        }
        if !adjustments.is_empty() {
            adjustments.index_columns(chunk);
            adjustments.jigsaw.index_columns(chunk);
        }
        Ok(adjustments)
    }

    fn jigsaw_adaptation(&self, kind: StructureKind) -> Option<Adaptation> {
        match self.registry().adaptation(kind)?.mode.as_str() {
            "bury" => Some(Adaptation::Bury),
            "encapsulate" => Some(Adaptation::Encapsulate),
            "beard_thin" => Some(Adaptation::BeardThin),
            _ => None,
        }
    }

    pub(super) fn adjust_order(&self) -> Vec<usize> {
        let mut order: Vec<usize> = (0..self.kinds.len()).collect();
        order.sort_by_key(|&index| self.registry().adaptation(self.kinds[index]).map_or(i32::MIN, |rule| rule.order));
        order
    }

    pub(super) fn adjust_strongholds(&self, probe: &dyn StartProbe, chunk: (i32, i32), adjustments: &mut Adjustments) -> Probed<()> {
        let reach = BoundingBox::new(
            chunk.0 * CHUNK_WIDTH - BURY_COLUMN_REACH,
            i32::MIN,
            chunk.1 * CHUNK_WIDTH - BURY_COLUMN_REACH,
            chunk.0 * CHUNK_WIDTH + CHUNK_WIDTH - 1 + BURY_COLUMN_REACH,
            i32::MAX,
            chunk.1 * CHUNK_WIDTH + CHUNK_WIDTH - 1 + BURY_COLUMN_REACH,
        );
        {
            for start in self.starts_near(probe, StructureKind::Stronghold, chunk, START_REACH)? {
                let Some(stronghold) = self.stronghold(probe, start)? else { continue };
                let boxes = stronghold.pieces.iter().map(|piece| piece.frame.bounds).filter(|bounds| bounds.intersects(&reach));
                adjustments.buries.extend(boxes.map(Bury::new));
            }
        }
        Ok(())
    }

    pub(super) fn adjust_villages(&self, probe: &dyn StartProbe, chunk: (i32, i32), adjustments: &mut Adjustments) -> Probed<()> {
        let embankment_reach = BoundingBox::new(
            chunk.0 * CHUNK_WIDTH - EMBANKMENT_MARGIN,
            i32::MIN,
            chunk.1 * CHUNK_WIDTH - EMBANKMENT_MARGIN,
            chunk.0 * CHUNK_WIDTH + CHUNK_WIDTH - 1 + EMBANKMENT_MARGIN,
            i32::MAX,
            chunk.1 * CHUNK_WIDTH + CHUNK_WIDTH - 1 + EMBANKMENT_MARGIN,
        );
        if let Some(catalogue) = self.layout() {
            for start in self.starts_near(probe, StructureKind::Village, chunk, VILLAGE_REACH)? {
                let Some(village) = self.village(probe, start)? else { continue };
                let fixed = village.pieces.iter().filter(|part| catalogue.entries[part.entry].grounding() == Grounding::Fixed);
                adjustments
                    .embankments
                    .extend(fixed.filter(|part| part.bounds.intersects(&embankment_reach)).map(|part| Embankment::new(part.bounds, part.ground)));
            }
        }
        Ok(())
    }

    pub(super) fn adjust_cities(&self, probe: &dyn StartProbe, chunk: (i32, i32), adjustments: &mut Adjustments) -> Probed<()> {
        let embankment_reach = BoundingBox::new(
            chunk.0 * CHUNK_WIDTH - EMBANKMENT_MARGIN,
            i32::MIN,
            chunk.1 * CHUNK_WIDTH - EMBANKMENT_MARGIN,
            chunk.0 * CHUNK_WIDTH + CHUNK_WIDTH - 1 + EMBANKMENT_MARGIN,
            i32::MAX,
            chunk.1 * CHUNK_WIDTH + CHUNK_WIDTH - 1 + EMBANKMENT_MARGIN,
        );
        if let Some(catalogue) = self.layout() {
            for start in self.starts_near(probe, StructureKind::AncientCity, chunk, CITY_START_REACH)? {
                let Some(city) = self.ancient_city(probe, start)? else { continue };
                let fixed = city.pieces.iter().filter(|part| catalogue.entries[part.entry].grounding() == Grounding::Fixed);
                adjustments
                    .shavers
                    .extend(fixed.filter(|part| part.bounds.intersects(&embankment_reach)).map(|part| Shaver::new(part.bounds, part.ground)));
            }
        }
        Ok(())
    }

    pub(super) fn adjust_v2(&self, probe: &dyn StartProbe, kind: StructureKind, chunk: (i32, i32), adjustments: &mut Adjustments) -> Probed<()> {
        let embankment_reach = BoundingBox::new(
            chunk.0 * CHUNK_WIDTH - EMBANKMENT_MARGIN,
            i32::MIN,
            chunk.1 * CHUNK_WIDTH - EMBANKMENT_MARGIN,
            chunk.0 * CHUNK_WIDTH + CHUNK_WIDTH - 1 + EMBANKMENT_MARGIN,
            i32::MAX,
            chunk.1 * CHUNK_WIDTH + CHUNK_WIDTH - 1 + EMBANKMENT_MARGIN,
        );
        let (x0, z0) = (chunk.0 * CHUNK_WIDTH, chunk.1 * CHUNK_WIDTH);
        if let Some(adaptation) = self.jigsaw_adaptation(kind) {
            for start in self.starts_near(probe, kind, chunk, self.registry().reach(kind) + V2_START_MARGIN)? {
                let Some(structure) = self.v2_structure(probe, kind, start)? else { continue };
                for part in structure.pieces.iter().filter(|part| part.bounds.intersects(&embankment_reach)) {
                    adjustments.jigsaw.extend_junctions(
                        part.junctions
                            .iter()
                            .copied()
                            .filter(|junction| (x0..=x0 + JUNCTION_WINDOW).contains(&junction.x) && (z0..=z0 + JUNCTION_WINDOW).contains(&junction.z)),
                    );
                    if self.v2_layout().is_some_and(|catalogue| catalogue.entries[part.entry].grounding() == Grounding::Fixed) {
                        adjustments.jigsaw.push_rigid(Rigid {
                            bounds: part.bounds,
                            ground: part.bounds.y0 + part.ground,
                            adaptation,
                        });
                    }
                }
            }
        }
        Ok(())
    }
}
