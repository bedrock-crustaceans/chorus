use super::kernel;
use crate::shared::CHUNK_WIDTH;
use crate::shared::structure::BoundingBox;
use crate::shared::structure::layout::Junction;

const COLUMN_COUNT: usize = 256;
const WIDE_COLUMN_REACH: i32 = 11;
const BURY_COLUMN_REACH: i32 = 5;
const ENCAPSULATION_HEIGHT_REACH: i32 = 11;
const BEARD_WEIGHT: f64 = 0.8;
const ENCAPSULATION_WEIGHT: f64 = 0.8;
const BURY_SCALE: i32 = 2;
const JUNCTION_BELOW: i32 = 12;
const JUNCTION_ABOVE: i32 = 11;
const JUNCTION_WEIGHT: f64 = 0.4;
const JUNCTION_LOW_COLUMN: i32 = -11;
const JUNCTION_HIGH_COLUMN: i32 = 12;

#[derive(Default)]
pub(super) struct ColumnIndex {
    offsets: Vec<u32>,
    entries: Vec<u16>,
}

impl ColumnIndex {
    pub(super) fn build(chunk: (i32, i32), count: usize, reaches: impl Fn(usize, i32, i32) -> bool) -> Self {
        let mut index = Self {
            offsets: Vec::with_capacity(COLUMN_COUNT + 1),
            entries: Vec::new(),
        };
        index.offsets.push(0);
        for column in 0..COLUMN_COUNT as i32 {
            let (x, z) = (chunk.0 * CHUNK_WIDTH + column % CHUNK_WIDTH, chunk.1 * CHUNK_WIDTH + column / CHUNK_WIDTH);
            index.entries.extend((0..count).filter(|&entry| reaches(entry, x, z)).map(|entry| entry as u16));
            index.offsets.push(index.entries.len() as u32);
        }
        index
    }

    pub(super) fn column(&self, column: usize) -> &[u16] {
        if self.offsets.is_empty() {
            &[]
        } else {
            &self.entries[self.offsets[column] as usize..self.offsets[column + 1] as usize]
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Adaptation {
    Bury,
    BeardThin,
    Encapsulate,
}

#[derive(Debug, Clone, Copy)]
pub struct Rigid {
    pub bounds: BoundingBox,
    pub ground: i32,
    pub adaptation: Adaptation,
}

impl Rigid {
    fn column_reach(&self) -> i32 {
        match self.adaptation {
            Adaptation::Bury => BURY_COLUMN_REACH,
            _ => WIDE_COLUMN_REACH,
        }
    }
}

#[derive(Default)]
pub struct JigsawAdjustments {
    rigids: Vec<Rigid>,
    junctions: Vec<Junction>,
    rigid_index: ColumnIndex,
    junction_index: ColumnIndex,
}

fn distances_outside(bounds: &BoundingBox, x: i32, z: i32) -> (i32, i32) {
    ((bounds.x0 - x).max(x - bounds.x1), (bounds.z0 - z).max(z - bounds.z1))
}

impl JigsawAdjustments {
    pub fn is_empty(&self) -> bool {
        self.rigids.is_empty() && self.junctions.is_empty()
    }

    pub fn push_rigid(&mut self, rigid: Rigid) {
        self.rigids.push(rigid);
    }

    pub fn extend_junctions(&mut self, junctions: impl Iterator<Item = Junction>) {
        self.junctions.extend(junctions);
    }

    pub fn index_columns(&mut self, chunk: (i32, i32)) {
        let rigids = &self.rigids;
        self.rigid_index = ColumnIndex::build(chunk, rigids.len(), |entry, x, z| {
            let (outside_x, outside_z) = distances_outside(&rigids[entry].bounds, x, z);
            let reach = rigids[entry].column_reach();
            outside_x <= reach && outside_z <= reach
        });
        let junctions = &self.junctions;
        self.junction_index = ColumnIndex::build(chunk, junctions.len(), |entry, x, z| {
            (JUNCTION_LOW_COLUMN..=JUNCTION_HIGH_COLUMN).contains(&(junctions[entry].x - x)) && (JUNCTION_LOW_COLUMN..=JUNCTION_HIGH_COLUMN).contains(&(junctions[entry].z - z))
        });
    }

    pub fn column_is_empty(&self, column: usize) -> bool {
        self.rigid_index.column(column).is_empty() && self.junction_index.column(column).is_empty()
    }

    pub fn apply(&self, column: usize, x: i32, y: i32, z: i32, density: f32) -> f32 {
        let (rigids, junctions) = (self.rigid_index.column(column), self.junction_index.column(column));
        if rigids.is_empty() && junctions.is_empty() {
            return density;
        }
        let mut contribution = 0.0f64;
        for &entry in rigids {
            contribution += self.rigid_contribution(&self.rigids[entry as usize], x, y, z);
        }
        let mut joined = 0.0f64;
        for &entry in junctions {
            let junction = &self.junctions[entry as usize];
            if junction.ground - JUNCTION_BELOW <= y && y <= junction.ground + JUNCTION_ABOVE {
                let dy = y - junction.ground;
                joined += kernel::beard(x - junction.x, dy, z - junction.z, dy) as f64 * JUNCTION_WEIGHT;
            }
        }
        (contribution + joined + density as f64) as f32
    }

    fn rigid_contribution(&self, rigid: &Rigid, x: i32, y: i32, z: i32) -> f64 {
        let (outside_x, outside_z) = distances_outside(&rigid.bounds, x, z);
        if outside_x > rigid.column_reach() || outside_z > rigid.column_reach() {
            return 0.0;
        }
        let (dx, dz) = (outside_x.max(0), outside_z.max(0));
        let to_ground = y - rigid.ground;
        match rigid.adaptation {
            Adaptation::Bury => kernel::bury(BURY_SCALE * dx, BURY_SCALE * dz, to_ground) as f64,
            Adaptation::BeardThin => kernel::beard(dx, to_ground, dz, to_ground) as f64 * BEARD_WEIGHT,
            Adaptation::Encapsulate => {
                let dy = (rigid.bounds.y0 - y).max(y - rigid.bounds.y1);
                if dy > ENCAPSULATION_HEIGHT_REACH {
                    return 0.0;
                }
                kernel::bury(dx, dz, dy.max(0)) as f64 * ENCAPSULATION_WEIGHT
            }
        }
    }
}
