use super::super::placed::Placed;
use super::super::world::read;
use super::BlockSet;
use super::support::{holds_face, is_open, is_water_source, is_wet};
use crate::shared::blocks::{BlockId, BlockTable, Blocks, int, state};
use crate::shared::chunk::region::Region;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::Probed;
use crate::shared::survival::{Direction, multiface_bit};
use crate::shared::tags::Tag;
use chorus_block::block_id::{CALIBRATED_SCULK_SENSOR, FIRE, LAVA, SCULK, SCULK_CATALYST, SCULK_SENSOR, SCULK_SHRIEKER, SCULK_VEIN, SNOW_LAYER, SOUL_FIRE};
use chorus_block::state::common::{MINECRAFT_VERTICAL_HALF, MULTI_FACE_DIRECTION_BITS, UPSIDE_DOWN_BIT, WEIRDO_DIRECTION};
use glam::IVec3;
use serde_json::Value;
use std::sync::Arc;

const FACE_COMBINATIONS: usize = 64;
const SPREAD_KINDS: [u8; 3] = [0, 1, 2];
const STAIRS_SUFFIX: &str = "_stairs";
const SLAB_SUFFIX: &str = "_slab";
const PANE_SUFFIX: &str = "_pane";
const BARS_SUFFIX: &str = "_bars";
const TOP_HALF: &str = "top";
pub const SPREAD_ORDER: [Direction; 6] = [Direction::Up, Direction::Down, Direction::North, Direction::South, Direction::West, Direction::East];

fn stair_back(weirdo: i32) -> Direction {
    match weirdo {
        0 => Direction::East,
        1 => Direction::West,
        2 => Direction::South,
        _ => Direction::North,
    }
}

fn multiface_support(blocks: &Blocks, block: BlockId, face: Direction) -> bool {
    let entry = blocks.entry(block);
    let name = entry.name.as_ref();
    if name == SCULK_SENSOR || name == SCULK_SHRIEKER || name == CALIBRATED_SCULK_SENSOR {
        return face == Direction::Up;
    }
    if name.ends_with(PANE_SUFFIX) || name.ends_with(BARS_SUFFIX) {
        return false;
    }
    if entry.cube || blocks.is(block, Tag::Leaves) {
        return true;
    }
    if name.ends_with(STAIRS_SUFFIX) {
        let vertical = if blocks.flag(block, &UPSIDE_DOWN_BIT) { Direction::Up } else { Direction::Down };
        return face.opposite() == vertical || Some(face) == Some(stair_back(blocks.int(block, &WEIRDO_DIRECTION).unwrap_or(0)).opposite());
    }
    if name.ends_with(SLAB_SUFFIX) {
        let vertical = if blocks.text(block, &MINECRAFT_VERTICAL_HALF) == Some(TOP_HALF) {
            Direction::Up
        } else {
            Direction::Down
        };
        return face.opposite() == vertical;
    }
    false
}

#[derive(Debug)]
pub struct Multiface {
    by_bits: Vec<BlockId>,
    kind: u16,
    vein: bool,
    sculk: [u16; 2],
    faces: Vec<Direction>,
    search_range: i32,
    can_place_on: BlockSet,
    spread_chance: f32,
}

#[derive(Debug)]
pub struct Threshold {
    pub feature: Arc<Placed>,
    pub depth: i32,
}

impl Threshold {
    pub fn place(&self, region: &mut Region, random: &mut MersenneTwister, pos: IVec3) -> Probed<bool> {
        let Some(surface) = region.preliminary_surface(pos.x >> 2, pos.z >> 2) else { return Ok(false) };
        if surface == crate::bedrock::overworld::terrain::SURFACE_NOT_FOUND || pos.y >= surface - self.depth {
            return Ok(false);
        }
        self.feature.place(region, random, pos)
    }
}

impl Multiface {
    pub fn parse(table: &mut BlockTable, body: &'static Value) -> Option<Self> {
        let name = body.get("places_block")?.as_str()?;
        let flag = |key: &str| body.get(key).and_then(Value::as_bool).unwrap_or(false);
        let mut faces = Vec::new();
        if flag("can_place_on_floor") {
            faces.push(Direction::Down);
        }
        if flag("can_place_on_ceiling") {
            faces.push(Direction::Up);
        }
        if flag("can_place_on_wall") {
            faces.extend([Direction::North, Direction::East, Direction::South, Direction::West]);
        }
        let mut by_bits = vec![0; FACE_COMBINATIONS];
        for (bits, slot) in by_bits.iter_mut().enumerate().skip(1) {
            let block = table.get(name, &[state(&MULTI_FACE_DIRECTION_BITS, int(bits as i32))]);
            table.flooded(block);
            *slot = block;
        }
        Some(Self {
            by_bits,
            kind: table.kind(name),
            vein: name == SCULK_VEIN,
            sculk: [table.kind(SCULK), table.kind(SCULK_CATALYST)],
            faces,
            search_range: body.get("search_range")?.as_i64()? as i32,
            can_place_on: BlockSet::parse(table, body.get("can_place_on")),
            spread_chance: body.get("chance_of_spreading")?.as_f64()? as f32,
        })
    }

    fn bits(&self, region: &Region, block: BlockId) -> i32 {
        if region.blocks.is_kind(block, self.kind) {
            region.blocks.int(block, &MULTI_FACE_DIRECTION_BITS).unwrap_or(0)
        } else {
            0
        }
    }

    fn has_face(&self, region: &Region, block: BlockId, face: Direction) -> bool {
        self.bits(region, block) & multiface_bit(face) != 0
    }

    fn supports(&self, region: &Region, pos: IVec3, face: Direction) -> bool {
        let block = read(region, pos);
        if self.vein {
            multiface_support(region.blocks, block, face)
        } else {
            region.blocks.is(block, Tag::Leaves) || holds_face(region.blocks, block)
        }
    }

    fn placement(&self, region: &Region, at: IVec3, existing: BlockId, face: Direction) -> BlockId {
        let own = self.bits(region, existing);
        if own & multiface_bit(face) != 0 || !self.supports(region, at + face.offset(), face) {
            return existing;
        }
        let block = self.by_bits[(own | multiface_bit(face)) as usize];
        if is_wet(region.blocks, existing) { region.blocks.flooded(block) } else { block }
    }

    fn takes(&self, region: &Region, at: IVec3, face: Direction) -> bool {
        self.can_place_on.is_empty() || self.can_place_on.contains(region.blocks, read(region, at + face.offset()))
    }

    fn attach(&self, region: &mut Region, random: &mut MersenneTwister, at: IVec3, faces: &[Direction]) -> bool {
        let existing = read(region, at);
        let Some(&face) = faces.iter().find(|&&face| self.takes(region, at, face)) else { return false };
        let placed = self.placement(region, at, existing, face);
        if placed == existing {
            return false;
        }
        region.set(at, placed);
        if random.next_float() < self.spread_chance {
            self.spread(region, at, placed, face);
        }
        true
    }

    pub fn place(&self, region: &mut Region, random: &mut MersenneTwister, at: IVec3) -> bool {
        if !is_open(region.blocks, read(region, at)) {
            return false;
        }
        if self.attach(region, random, at, &self.faces) {
            return true;
        }
        for &facing in &self.faces {
            let next = at + facing.offset();
            let block = read(region, next);
            let searchable = self.search_range > 0 && (is_open(region.blocks, block) || region.blocks.is_kind(block, self.kind));
            if !searchable {
                continue;
            }
            let faces: Vec<Direction> = self.faces.iter().copied().filter(|&face| face != facing.opposite()).collect();
            if self.attach(region, random, next, &faces) {
                return true;
            }
        }
        false
    }

    fn can_spread_into(&self, region: &Region, cell: IVec3, face: Direction) -> bool {
        let blocks = region.blocks;
        let block = read(region, cell);
        if self.vein {
            let behind = read(region, cell + face.offset());
            if self.sculk.iter().any(|&kind| blocks.is_kind(behind, kind)) {
                return false;
            }
            if self.replaceable(region, block) {
                return true;
            }
        }
        blocks.is_air(block) || is_water_source(blocks, block) || blocks.is_kind(block, self.kind)
    }

    fn spread_target(&self, region: &Region, state: BlockId, at: IVec3, from: Direction, to: Direction) -> Option<(IVec3, BlockId)> {
        self.spread_target_with(region, state, at, from, to, &SPREAD_KINDS)
    }

    fn spread_target_with(&self, region: &Region, state: BlockId, at: IVec3, from: Direction, to: Direction, kinds: &[u8]) -> Option<(IVec3, BlockId)> {
        if to == from || to == from.opposite() {
            return None;
        }
        let is_vein_state = self.vein && region.blocks.is_kind(state, self.kind);
        if is_vein_state && !self.has_face(region, state, from) {
            return None;
        }
        let other_valid_source = self.vein && !is_vein_state;
        if !(other_valid_source || self.has_face(region, state, from) && !self.has_face(region, state, to)) {
            return None;
        }
        kinds.iter().find_map(|&kind| {
            let (cell, face) = match kind {
                0 => (at, to),
                1 => (at + to.offset(), from),
                _ => (at + from.offset() + to.offset(), to.opposite()),
            };
            if !self.can_spread_into(region, cell, face) {
                return None;
            }
            let existing = read(region, cell);
            let placed = self.placement(region, cell, existing, face);
            (placed != existing).then_some((cell, placed))
        })
    }

    pub fn spreader(table: &mut BlockTable) -> Self {
        let mut by_bits = vec![0; FACE_COMBINATIONS];
        for (bits, slot) in by_bits.iter_mut().enumerate().skip(1) {
            let block = table.get(SCULK_VEIN, &[state(&MULTI_FACE_DIRECTION_BITS, int(bits as i32))]);
            table.flooded(block);
            *slot = block;
        }
        Self {
            by_bits,
            kind: table.kind(SCULK_VEIN),
            vein: true,
            sculk: [table.kind(SCULK), table.kind(SCULK_CATALYST)],
            faces: Vec::new(),
            search_range: 0,
            can_place_on: BlockSet::default(),
            spread_chance: 1.0,
        }
    }

    pub fn is_vein(&self, region: &Region, block: BlockId) -> bool {
        region.blocks.is_kind(block, self.kind)
    }

    pub fn face_bits(&self, region: &Region, block: BlockId) -> i32 {
        self.bits(region, block)
    }

    pub fn holds_face(&self, region: &Region, block: BlockId, face: Direction) -> bool {
        self.has_face(region, block, face)
    }

    pub fn with_faces(&self, bits: i32) -> BlockId {
        self.by_bits[bits as usize]
    }

    pub fn replaceable(&self, region: &Region, block: BlockId) -> bool {
        let blocks = region.blocks;
        if blocks.name_of(block) == FIRE || blocks.name_of(block) == SOUL_FIRE || blocks.name_of(block) == LAVA {
            return false;
        }
        is_water_source(blocks, block)
            || blocks.is_air(block)
            || blocks.is(block, Tag::Replaceable)
            || blocks.name_of(block) == SNOW_LAYER
            || blocks.name_of(block).ends_with("_vein")
            || blocks.name_of(block) == chorus_block::block_id::GLOW_LICHEN
    }

    pub fn spread_all(&self, region: &mut Region, state: BlockId, at: IVec3, same_space_only: bool) -> usize {
        let kinds: &[u8] = if same_space_only { &SPREAD_KINDS[..1] } else { &SPREAD_KINDS };
        let mut placed = 0;
        for from in SPREAD_ORDER {
            for to in SPREAD_ORDER {
                if let Some((cell, block)) = self.spread_target_with(region, state, at, from, to, kinds) {
                    region.set(cell, block);
                    placed += 1;
                }
            }
        }
        placed
    }

    fn spread(&self, region: &mut Region, at: IVec3, state: BlockId, from: Direction) {
        let mut targets = Direction::ALL.iter().filter_map(|&to| self.spread_target(region, state, at, from, to));
        let (first, second) = (targets.next(), targets.next());
        if let Some((cell, block)) = first
            && (!self.vein || second.is_none())
        {
            region.set(cell, block);
        }
    }
}
