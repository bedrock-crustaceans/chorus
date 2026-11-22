use glam::IVec3;

use super::blocks::{BlockId, Blocks};
use super::chunk::region::BlockView;
use super::tags::Tag;
use chorus_block::block_id::*;
use chorus_block::component::placement_filter_component::PlacementFace;
use chorus_block::state::common::{CORAL_DIRECTION, HANGING, HEIGHT};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Direction {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl Direction {
    pub const ALL: [Direction; 6] = [Self::Down, Self::Up, Self::North, Self::South, Self::West, Self::East];
    pub const HORIZONTAL: [Direction; 4] = [Self::North, Self::East, Self::South, Self::West];

    pub fn offset(self) -> IVec3 {
        match self {
            Self::Down => IVec3::NEG_Y,
            Self::Up => IVec3::Y,
            Self::North => IVec3::NEG_Z,
            Self::South => IVec3::Z,
            Self::West => IVec3::NEG_X,
            Self::East => IVec3::X,
        }
    }

    pub fn opposite(self) -> Self {
        match self {
            Self::Down => Self::Up,
            Self::Up => Self::Down,
            Self::North => Self::South,
            Self::South => Self::North,
            Self::West => Self::East,
            Self::East => Self::West,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Down => "down",
            Self::Up => "up",
            Self::North => "north",
            Self::South => "south",
            Self::West => "west",
            Self::East => "east",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Fluid {
    Water,
    Lava,
}

pub fn fluid_at(blocks: &Blocks, block: BlockId) -> Option<Fluid> {
    let entry = blocks.entry(block);
    if entry.name.as_ref() == WATER || entry.flooded {
        Some(Fluid::Water)
    } else if entry.name.as_ref() == LAVA {
        Some(Fluid::Lava)
    } else {
        None
    }
}

fn sturdy(region: &(impl BlockView + ?Sized), pos: IVec3) -> bool {
    region.blocks().entry(region.get(pos)).full
}

pub fn vine_bit(face: Direction) -> i32 {
    match face {
        Direction::South => 1,
        Direction::West => 2,
        Direction::North => 4,
        Direction::East => 8,
        _ => 0,
    }
}

pub fn multiface_bit(face: Direction) -> i32 {
    match face {
        Direction::Down => 1,
        Direction::Up => 2,
        Direction::South => 4,
        Direction::West => 8,
        Direction::North => 16,
        Direction::East => 32,
    }
}

pub fn from_legacy_direction(value: i32) -> Direction {
    match value {
        1 => Direction::West,
        2 => Direction::North,
        3 => Direction::East,
        _ => Direction::South,
    }
}

pub fn coral_fan_facing(blocks: &Blocks, fan: BlockId) -> Direction {
    match blocks.int(fan, &CORAL_DIRECTION) {
        Some(1) => Direction::East,
        Some(2) => Direction::North,
        Some(3) => Direction::South,
        _ => Direction::West,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Support {
    Anywhere,
    SugarCane,
    Cactus,
    CactusFlower,
    Mushroom,
    SturdyFloor,
    Seagrass,
    SolidFloor,
    Kelp,
    Propagule,
    SnowLayer,
    AnyFloor,
    SporeBlossom,
    SturdyCeiling,
    PaleHangingMoss,
    Waterlily,
    BigDripleaf,
    WallCoral,
    Coral,
    Placed,
}

impl Support {
    pub fn of(name: &str, has: impl Fn(Tag) -> bool, has_placement: bool) -> Self {
        match name {
            REEDS => Self::SugarCane,
            CACTUS => Self::Cactus,
            CACTUS_FLOWER => Self::CactusFlower,
            BROWN_MUSHROOM | RED_MUSHROOM => Self::Mushroom,
            LEAF_LITTER => Self::SturdyFloor,
            SEAGRASS => Self::Seagrass,
            SEA_PICKLE => Self::SolidFloor,
            KELP => Self::Kelp,
            MANGROVE_PROPAGULE => Self::Propagule,
            SNOW_LAYER => Self::SnowLayer,
            MOSS_CARPET | PALE_MOSS_CARPET => Self::AnyFloor,
            SPORE_BLOSSOM => Self::SporeBlossom,
            HANGING_ROOTS => Self::SturdyCeiling,
            PALE_HANGING_MOSS => Self::PaleHangingMoss,
            WATERLILY => Self::Waterlily,
            BIG_DRIPLEAF => Self::BigDripleaf,
            MELON_BLOCK | PUMPKIN | CARVED_PUMPKIN | BUBBLE_COLUMN => Self::Anywhere,
            _ if has(Tag::Leaves) => Self::Anywhere,
            _ if has(Tag::WallCorals) => Self::WallCoral,
            _ if has(Tag::Corals) => Self::Coral,
            _ if has_placement => Self::Placed,
            _ => Self::Anywhere,
        }
    }
}

pub fn can_survive(region: &(impl BlockView + ?Sized), block: BlockId, pos: IVec3) -> bool {
    let blocks = region.blocks();
    let entry = blocks.entry(block);
    let below = region.get(pos - IVec3::Y);
    if entry.upper == Some(true) {
        return blocks.same_kind(below, block) && blocks.entry(below).upper != Some(true);
    }
    match entry.support {
        Support::SugarCane => {
            if blocks.same_kind(below, block) {
                return true;
            }
            blocks.placed_on(block, below, PlacementFace::Up)
                && Direction::HORIZONTAL.iter().any(|direction| {
                    let neighbor = region.get(pos - IVec3::Y + direction.offset());
                    fluid_at(blocks, neighbor) == Some(Fluid::Water) || blocks.is(neighbor, Tag::SupportsSugarCaneAdjacently)
                })
        }
        Support::Cactus => {
            let blocked = Direction::HORIZONTAL.iter().any(|direction| {
                let neighbor = region.get(pos + direction.offset());
                blocks.entry(neighbor).solid || fluid_at(blocks, neighbor) == Some(Fluid::Lava)
            });
            !blocked && (blocks.same_kind(below, block) || blocks.placed_on(block, below, PlacementFace::Up)) && fluid_at(blocks, region.get(pos + IVec3::Y)).is_none()
        }
        Support::CactusFlower => matches!(blocks.name_of(below), CACTUS | FARMLAND) || blocks.entry(below).full,
        Support::Mushroom => {
            let entry = blocks.entry(below);
            blocks.is(below, Tag::Mud) || matches!(entry.name.as_ref(), MYCELIUM | PODZOL) || (entry.full && !blocks.is(below, Tag::Leaves))
        }
        Support::SturdyFloor => sturdy(region, pos - IVec3::Y),
        Support::Seagrass => sturdy(region, pos - IVec3::Y) && !blocks.denied_on(block, below),
        Support::SolidFloor => blocks.entry(below).solid,
        Support::Kelp => (sturdy(region, pos - IVec3::Y) || blocks.name_of(below) == KELP) && !blocks.denied_on(block, below),
        Support::Propagule => {
            if blocks.flag(block, &HANGING) {
                blocks.placed_on(block, region.get(pos + IVec3::Y), PlacementFace::Down)
            } else {
                blocks.placed_on(block, below, PlacementFace::Up)
            }
        }
        Support::SnowLayer => {
            if blocks.denied_on(block, below) {
                return false;
            }
            blocks.placed_on(block, below, PlacementFace::Up) || blocks.entry(below).full || (blocks.name_of(below) == SNOW_LAYER && blocks.int(below, &HEIGHT) == Some(7))
        }
        Support::AnyFloor => !blocks.is_air(below),
        Support::SporeBlossom => sturdy(region, pos + IVec3::Y) && fluid_at(blocks, region.get(pos)).is_none(),
        Support::SturdyCeiling => sturdy(region, pos + IVec3::Y),
        Support::PaleHangingMoss => {
            let above = region.get(pos + IVec3::Y);
            sturdy(region, pos + IVec3::Y) || blocks.same_kind(above, block)
        }
        Support::Waterlily => blocks.placed_on(block, below, PlacementFace::Up) && fluid_at(blocks, region.get(pos)).is_none(),
        Support::BigDripleaf => blocks.placed_on(block, below, PlacementFace::Up) || blocks.name_of(below) == BIG_DRIPLEAF,
        Support::WallCoral => sturdy(region, pos - coral_fan_facing(blocks, block).offset()),
        Support::Coral => sturdy(region, pos - IVec3::Y),
        Support::Placed => blocks.placed_on(block, below, PlacementFace::Up),
        Support::Anywhere => true,
    }
}
