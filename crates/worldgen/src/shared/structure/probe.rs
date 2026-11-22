use std::cell::RefCell;
use std::fmt;
use std::sync::Arc;

use rustc_hash::FxHashMap;

use super::plan::ColumnHeights;
use super::starts::Starts;
use crate::shared::SEA_LEVEL;
use crate::shared::blocks::BlockId;
use crate::shared::chunk::proto::ProtoChunk;
use crate::shared::rng::mersenne::MersenneTwister;
use chorus_level::dimension_definition::DimensionDefinition;

pub const BIOME_LATTICE_STEP: usize = 4;
pub const NO_SURFACE: i32 = 0x7fff;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MissingInput {
    pub what: &'static str,
    pub cell: (i32, i32),
}

impl fmt::Display for MissingInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the phase graph did not supply the {} of {:?}", self.what, self.cell)
    }
}

impl std::error::Error for MissingInput {}

pub type Probed<T> = Result<T, MissingInput>;

type ChunkCache = FxHashMap<((i32, i32), bool), Option<Arc<ProtoChunk>>>;

pub trait StartProbe {
    fn preliminary_surface(&self, chunk: (i32, i32)) -> i32;

    fn surface_at_quart(&self, quart: (i32, i32)) -> i32;

    fn biome(&self, x: i32, y: i32, z: i32) -> i32;

    fn definition(&self) -> DimensionDefinition {
        DimensionDefinition::OVERWORLD
    }

    fn sea_level(&self) -> i32 {
        SEA_LEVEL
    }

    fn lowest_column_top(&self, chunk: (i32, i32)) -> Probed<i32> {
        Ok(self.column_heights(chunk)?.lowest_top())
    }

    fn column_heights(&self, chunk: (i32, i32)) -> Probed<Arc<ColumnHeights>>;

    fn supplied_heights(&self, _chunk: (i32, i32)) -> Option<Arc<ColumnHeights>> {
        None
    }

    fn supplied_chunk(&self, _chunk: (i32, i32)) -> Option<Arc<ProtoChunk>> {
        None
    }

    fn starts_at(&self, _cell: (i32, i32)) -> Option<Arc<Starts>> {
        None
    }

    fn remember_starts(&self, _cell: (i32, i32), _starts: Arc<Starts>) {}

    fn block(&self, _x: i32, _y: i32, _z: i32) -> Probed<Option<BlockId>> {
        Ok(None)
    }

    fn adjusted_block(&self, _x: i32, _y: i32, _z: i32) -> Probed<Option<BlockId>> {
        Ok(None)
    }

    fn staged_block(&self, x: i32, y: i32, z: i32) -> Probed<Option<BlockId>> {
        match self.adjusted_block(x, y, z)? {
            Some(block) => Ok(Some(block)),
            None => self.block(x, y, z),
        }
    }

    fn chunk(&self, _chunk_x: i32, _chunk_z: i32) -> Probed<Option<ProtoChunk>> {
        Ok(None)
    }

    fn run_feature(&self, _chunk: &mut ProtoChunk, _id: &str, _origin: (i32, i32, i32), _random: &mut MersenneTwister) -> Probed<bool> {
        Ok(false)
    }
}

pub struct Remembering<'p> {
    inner: &'p dyn StartProbe,
    dry: bool,
    placeholder: Arc<ColumnHeights>,
    heights: RefCell<FxHashMap<(i32, i32), Arc<ColumnHeights>>>,
    starts: RefCell<FxHashMap<(i32, i32), Arc<Starts>>>,
    chunks: RefCell<ChunkCache>,
    missed_heights: RefCell<Vec<(i32, i32)>>,
    missed_starts: RefCell<Vec<(i32, i32)>>,
    missed_chunks: RefCell<Vec<(i32, i32)>>,
}

impl<'p> Remembering<'p> {
    pub fn new(inner: &'p dyn StartProbe) -> Self {
        let placeholder_height = inner.sea_level() as i16;
        Self {
            inner,
            dry: false,
            placeholder: Arc::new(ColumnHeights {
                solid: Box::new([placeholder_height; 256]),
                top: Box::new([placeholder_height; 256]),
                min_y: inner.definition().min_y,
            }),
            heights: RefCell::new(FxHashMap::default()),
            starts: RefCell::new(FxHashMap::default()),
            chunks: RefCell::new(FxHashMap::default()),
            missed_heights: RefCell::new(Vec::new()),
            missed_starts: RefCell::new(Vec::new()),
            missed_chunks: RefCell::new(Vec::new()),
        }
    }

    pub fn dry(inner: &'p dyn StartProbe) -> Self {
        Self { dry: true, ..Self::new(inner) }
    }

    pub fn missed_heights(&self) -> Vec<(i32, i32)> {
        self.missed_heights.borrow().clone()
    }

    pub fn missed_starts(&self) -> Vec<(i32, i32)> {
        self.missed_starts.borrow().clone()
    }

    pub fn missed_chunks(&self) -> Vec<(i32, i32)> {
        self.missed_chunks.borrow().clone()
    }

    fn chunk_of(&self, chunk: (i32, i32), adjusted: bool) -> Probed<Option<Arc<ProtoChunk>>> {
        if let Some(known) = self.chunks.borrow().get(&(chunk, adjusted)) {
            return Ok(known.clone());
        }
        let found = self.inner.supplied_chunk(chunk);
        if found.is_none() {
            if !self.dry {
                return Err(MissingInput { what: "carved terrain", cell: chunk });
            }
            self.missed_chunks.borrow_mut().push(chunk);
        }
        self.chunks.borrow_mut().insert((chunk, adjusted), found.clone());
        Ok(found)
    }
}

impl StartProbe for Remembering<'_> {
    fn preliminary_surface(&self, chunk: (i32, i32)) -> i32 {
        self.inner.preliminary_surface(chunk)
    }

    fn surface_at_quart(&self, quart: (i32, i32)) -> i32 {
        self.inner.surface_at_quart(quart)
    }

    fn biome(&self, x: i32, y: i32, z: i32) -> i32 {
        self.inner.biome(x, y, z)
    }

    fn definition(&self) -> DimensionDefinition {
        self.inner.definition()
    }

    fn sea_level(&self) -> i32 {
        self.inner.sea_level()
    }

    fn column_heights(&self, chunk: (i32, i32)) -> Probed<Arc<ColumnHeights>> {
        if let Some(known) = self.heights.borrow().get(&chunk) {
            return Ok(known.clone());
        }
        let heights = match self.inner.supplied_heights(chunk) {
            Some(supplied) => supplied,
            None if self.dry => {
                self.missed_heights.borrow_mut().push(chunk);
                self.placeholder.clone()
            }
            None => return Err(MissingInput { what: "column heights", cell: chunk }),
        };
        self.heights.borrow_mut().insert(chunk, heights.clone());
        Ok(heights)
    }

    fn starts_at(&self, cell: (i32, i32)) -> Option<Arc<Starts>> {
        if let Some(known) = self.starts.borrow().get(&cell) {
            return Some(known.clone());
        }
        if let Some(supplied) = self.inner.starts_at(cell) {
            self.starts.borrow_mut().insert(cell, supplied.clone());
            return Some(supplied);
        }
        if !self.dry {
            return None;
        }
        self.missed_starts.borrow_mut().push(cell);
        let empty = Arc::new(Starts::default());
        self.starts.borrow_mut().insert(cell, empty.clone());
        Some(empty)
    }

    fn remember_starts(&self, cell: (i32, i32), starts: Arc<Starts>) {
        self.starts.borrow_mut().insert(cell, starts);
    }

    fn block(&self, x: i32, y: i32, z: i32) -> Probed<Option<BlockId>> {
        Ok(self.chunk_of((x >> 4, z >> 4), false)?.map(|chunk| chunk.get(x & 15, y, z & 15)))
    }

    fn adjusted_block(&self, x: i32, y: i32, z: i32) -> Probed<Option<BlockId>> {
        Ok(self.chunk_of((x >> 4, z >> 4), true)?.map(|chunk| chunk.get(x & 15, y, z & 15)))
    }

    fn chunk(&self, chunk_x: i32, chunk_z: i32) -> Probed<Option<ProtoChunk>> {
        Ok(self.chunk_of((chunk_x, chunk_z), true)?.map(|chunk| (*chunk).clone()))
    }

    fn run_feature(&self, chunk: &mut ProtoChunk, id: &str, origin: (i32, i32, i32), random: &mut MersenneTwister) -> Probed<bool> {
        self.inner.run_feature(chunk, id, origin, random)
    }
}

pub fn contains_only(probe: &dyn StartProbe, (x, y, z): (i32, i32, i32), radius: i32, allowed: &[i32]) -> bool {
    let definition = probe.definition();
    let (low, high) = ((y - radius).max(definition.min_y), (y + radius).min(definition.max_y()));
    (x - radius..=x + radius).step_by(BIOME_LATTICE_STEP).all(|bx| {
        (z - radius..=z + radius)
            .step_by(BIOME_LATTICE_STEP)
            .all(|bz| (low..=high).step_by(BIOME_LATTICE_STEP).all(|by| allowed.contains(&probe.biome(bx, by, bz))))
    })
}
