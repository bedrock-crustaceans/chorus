use super::family::{Connections, Connector, Kind, SIDES, WALL_POST_SHIFT, WALL_TYPE_BITS};
use crate::shared::CHUNK_WIDTH;
use crate::shared::blocks::{BlockId, Blocks};
use crate::shared::chunk::proto::ProtoChunk;
use crate::shared::structure::palette::PaletteBlocks;
use crate::shared::structure::template::FULL_CUBE;
use crate::shared::structure::{Probed, StructureBlocks};
use crate::shared::tags::Tag;
use chorus_block::state::block_state::BlockState;

const UNJOINABLE_NAMES: [&str; 5] = ["minecraft:barrier", "minecraft:pumpkin", "minecraft:carved_pumpkin", "minecraft:lit_pumpkin", "minecraft:melon_block"];
const SHULKER_SUFFIX: &str = "shulker_box";
const COMPOSTER: &str = "minecraft:composter";
const POST_REACHING_NAMES: [&str; 4] = ["torch", "sign", "banner", "lantern"];
const VERTICAL_HALF: &str = "minecraft:vertical_half";
const BOTTOM: &str = "bottom";
const NORTH: usize = 0;
const EAST: usize = 1;
const SOUTH: usize = 2;
const WEST: usize = 3;
const NO_WALL: u16 = 0;
const SHORT_WALL: u16 = 1;
const TALL_WALL: u16 = 2;
const WALL_TYPE_MASK: u16 = (1 << WALL_TYPE_BITS) - 1;
pub(super) const POST_BIT: u16 = 1 << WALL_POST_SHIFT;

pub struct Surroundings<'a> {
    pub chunk: &'a ProtoChunk,
    pub ids: &'a StructureBlocks,
    pub blocks: &'a Blocks,
    pub connections: &'a Connections,
    pub palette: Option<&'a PaletteBlocks>,
    pub beside: &'a [Option<ProtoChunk>; 4],
    pub world: &'a dyn Fn(i32, i32, i32) -> Probed<Option<BlockId>>,
}

impl Surroundings<'_> {
    pub fn get(&self, x: i32, y: i32, z: i32) -> Probed<BlockId> {
        let (lx, lz) = (x - self.chunk.min_x(), z - self.chunk.min_z());
        let inside = |local: i32| (0..CHUNK_WIDTH).contains(&local);
        let side = match (inside(lx), inside(lz)) {
            (true, true) => return Ok(self.chunk.get(lx, y, lz)),
            (false, true) => {
                if lx < 0 {
                    WEST
                } else {
                    EAST
                }
            }
            (true, false) => {
                if lz < 0 {
                    NORTH
                } else {
                    SOUTH
                }
            }
            (false, false) => return Ok(self.ids.air),
        };
        match &self.beside[side] {
            Some(chunk) => Ok(chunk.get(lx.rem_euclid(CHUNK_WIDTH), y, lz.rem_euclid(CHUNK_WIDTH))),
            None => Ok((self.world)(x, y, z)?.unwrap_or(self.ids.air)),
        }
    }

    fn full_cube(&self, block: BlockId) -> bool {
        let entry = self.blocks.entry(block);
        if entry.name.as_ref() == "minecraft:mob_spawner" {
            return true;
        }
        match self.palette.and_then(|palette| palette.flags_of(block)) {
            Some(flags) => flags & FULL_CUBE != 0,
            None => entry.collision_cube,
        }
    }

    fn sturdy_toward(&self, block: BlockId, (dx, dz): (i32, i32)) -> bool {
        let name = &self.blocks.entry(block).name;
        if name.as_ref() == COMPOSTER {
            return true;
        }
        if let Some(stairs) = self.connections.member(block).filter(|connector| connector.kind == Kind::Stairs) {
            return stairs.facing_vector() == (-dx, -dz);
        }
        let unjoinable = self.blocks.is(block, Tag::Leaves) || UNJOINABLE_NAMES.iter().any(|member| *member == name.as_ref()) || name.ends_with(SHULKER_SUFFIX);
        !unjoinable && self.full_cube(block)
    }

    fn state_is(&self, block: BlockId, key: &str, value: &str) -> bool {
        self.blocks
            .entry(block)
            .states
            .iter()
            .any(|(name, state)| name.as_ref() == key && matches!(state, BlockState::Enum(text) if text.as_ref() == value))
    }

    fn fence_sturdy(&self, block: BlockId) -> bool {
        let name = &self.blocks.entry(block).name;
        if self.connections.member(block).is_some_and(|connector| connector.kind == Kind::Stairs) {
            return false;
        }
        if name.as_ref() == COMPOSTER {
            return true;
        }
        let unjoinable = self.blocks.is(block, Tag::Leaves) || UNJOINABLE_NAMES.iter().any(|member| *member == name.as_ref()) || name.ends_with(SHULKER_SUFFIX);
        !unjoinable && self.full_cube(block)
    }

    pub fn joins(&self, kind: Kind, block: BlockId, side: (i32, i32)) -> bool {
        if matches!(kind, Kind::Fence | Kind::NetherFence) {
            return self.fence_sturdy(block) || self.connections.kind(block) == Some(kind) || self.connections.kind(block) == Some(Kind::Gate);
        }
        if self.sturdy_toward(block, side) {
            return true;
        }
        let other = self.connections.kind(block);
        match kind {
            Kind::Fence | Kind::NetherFence => other == Some(kind) || other == Some(Kind::Gate),
            Kind::Pane => other == Some(Kind::Pane) || other == Some(Kind::Wall) || other == Some(Kind::Stairs),
            _ => other == Some(Kind::Pane) || other == Some(Kind::Wall),
        }
    }

    fn wall_joins(&self, block: BlockId, side: (i32, i32)) -> bool {
        match self.connections.member(block) {
            Some(Connector { kind: Kind::Wall | Kind::Pane, .. }) => true,
            Some(gate @ Connector { kind: Kind::Gate, .. }) => gate.runs_along_x() == (side.0 != 0),
            _ => self.sturdy_toward(block, side),
        }
    }

    fn covers_from_above(&self, above: BlockId) -> bool {
        self.full_cube(above) || self.connections.member(above).is_some_and(|connector| connector.kind == Kind::Stairs && !connector.upside_down) || self.state_is(above, VERTICAL_HALF, BOTTOM)
    }

    pub fn wall_code(&self, position: (i32, i32, i32)) -> Probed<u16> {
        self.wall_code_settled(position, None)
    }

    pub fn wall_code_settled(&self, (x, y, z): (i32, i32, i32), settled_above: Option<u16>) -> Probed<u16> {
        let above = self.get(x, y + 1, z)?;
        let stored_above = self.connections.member(above).filter(|connector| connector.kind == Kind::Wall).map(|connector| connector.code);
        let above_wall = settled_above.or(stored_above);
        let covers = self.covers_from_above(above);
        let mut code = 0u16;
        let mut types = [NO_WALL; 4];
        for (side, &(dx, dz)) in SIDES.iter().enumerate() {
            if !self.wall_joins(self.get(x + dx, y, z + dz)?, (dx, dz)) {
                continue;
            }
            let shift = side as u16 * WALL_TYPE_BITS;
            let above_has_side = above_wall.is_some_and(|wall| (wall >> shift) & WALL_TYPE_MASK != NO_WALL);
            types[side] = if covers || above_has_side { TALL_WALL } else { SHORT_WALL };
            code |= types[side] << shift;
        }
        let [north, east, south, west] = types;
        let straight_ns = north != NO_WALL && north == south && east == NO_WALL && west == NO_WALL;
        let straight_ew = east != NO_WALL && east == west && north == NO_WALL && south == NO_WALL;
        let above_post = above_wall.is_some_and(|wall| wall & POST_BIT != 0);
        let reaches = covers || above_post || POST_REACHING_NAMES.iter().any(|part| self.blocks.entry(above).name.contains(part));
        let post = above_post || !(straight_ns || straight_ew) || reaches && (if straight_ns { north } else { east }) == SHORT_WALL;
        Ok(code | u16::from(post) << WALL_POST_SHIFT)
    }

    pub fn corner_code(&self, (x, y, z): (i32, i32, i32), stairs: &Connector) -> Probed<u16> {
        let facing = stairs.facing_vector();
        let along_x = |vector: (i32, i32)| vector.0 != 0;
        let stair_at = |(dx, dz): (i32, i32)| -> Probed<Option<&Connector>> {
            Ok(self
                .connections
                .member(self.get(x + dx, y, z + dz)?)
                .filter(|connector| connector.kind == Kind::Stairs && connector.upside_down == stairs.upside_down))
        };
        let accepts = |offset: (i32, i32)| -> Probed<bool> { Ok(stair_at(offset)?.is_none_or(|neighbour| neighbour.facing_vector() != facing)) };
        let anticlockwise = (facing.1, -facing.0);
        if let Some(front) = stair_at(facing)? {
            let turned = front.facing_vector();
            if along_x(turned) != along_x(facing) && accepts((-turned.0, -turned.1))? {
                return Ok(if turned == anticlockwise { OUTER_LEFT } else { OUTER_RIGHT });
            }
        }
        if let Some(back) = stair_at((-facing.0, -facing.1))? {
            let turned = back.facing_vector();
            if along_x(turned) != along_x(facing) && accepts(turned)? {
                return Ok(if turned == anticlockwise { INNER_LEFT } else { INNER_RIGHT });
            }
        }
        Ok(NO_CORNER)
    }

    pub fn side_code(&self, kind: Kind, (x, y, z): (i32, i32, i32)) -> Probed<u16> {
        let mut code = 0u16;
        for (side, (dx, dz)) in SIDES.iter().enumerate() {
            if self.joins(kind, self.get(x + dx, y, z + dz)?, (*dx, *dz)) {
                code |= 1 << side;
            }
        }
        Ok(code)
    }
}

const NO_CORNER: u16 = 0;
const INNER_LEFT: u16 = 1;
const INNER_RIGHT: u16 = 2;
const OUTER_LEFT: u16 = 3;
const OUTER_RIGHT: u16 = 4;
