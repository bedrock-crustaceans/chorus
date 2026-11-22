use super::super::aquatic::{is_solid_blocking, is_water};
use super::super::ore::Draw;
use super::super::world::read;
use super::reef::Reef;
use super::{CORAL_COLORS, DOWN, EAST, FACES, FAN_DIRECTIONS, NORTH, SOUTH, WEST};
use crate::shared::blocks::{BlockId, BlockTable, int, state};
use crate::shared::chunk::region::Region;
use chorus_block::block_id::*;
use chorus_block::state::common::{CORAL_DIRECTION, CORAL_FAN_DIRECTION};
use glam::IVec3;

const CORAL_DEPTH: i32 = 4;

const CORAL_VARIANTS: usize = CORAL_COLORS * 2;

const WALL_DIRECTIONS: usize = 4;

const LIVE_BLOCKS: [&str; CORAL_COLORS] = [TUBE_CORAL_BLOCK, BRAIN_CORAL_BLOCK, BUBBLE_CORAL_BLOCK, FIRE_CORAL_BLOCK, HORN_CORAL_BLOCK];

const DEAD_BLOCKS: [&str; CORAL_COLORS] = [DEAD_TUBE_CORAL_BLOCK, DEAD_BRAIN_CORAL_BLOCK, DEAD_BUBBLE_CORAL_BLOCK, DEAD_FIRE_CORAL_BLOCK, DEAD_HORN_CORAL_BLOCK];

const PLANTS: [&str; CORAL_COLORS] = [TUBE_CORAL, BRAIN_CORAL, BUBBLE_CORAL, FIRE_CORAL, HORN_CORAL];

const FANS: [&str; CORAL_COLORS] = [TUBE_CORAL_FAN, BRAIN_CORAL_FAN, BUBBLE_CORAL_FAN, FIRE_CORAL_FAN, HORN_CORAL_FAN];

const WALL_FANS: [&str; CORAL_COLORS] = [TUBE_CORAL_WALL_FAN, BRAIN_CORAL_WALL_FAN, BUBBLE_CORAL_WALL_FAN, FIRE_CORAL_WALL_FAN, HORN_CORAL_WALL_FAN];

#[derive(Debug)]
pub struct Coral {
    water_kind: u16,
    pub(super) stone_kind: u16,
    kinds: [u16; CORAL_VARIANTS],
    pub(super) blocks: [BlockId; CORAL_VARIANTS],
    pub(super) plants: [BlockId; CORAL_COLORS],
    pub(super) fans: [[BlockId; FAN_DIRECTIONS]; CORAL_COLORS],
    pub(super) wall_fans: [[BlockId; WALL_DIRECTIONS]; CORAL_COLORS],
}

impl Coral {
    pub fn new(table: &mut BlockTable) -> Self {
        let water_kind = table.kind(WATER);
        let stone_kind = table.kind(STONE);
        let mut kinds = [0; CORAL_VARIANTS];
        let mut blocks = [0; CORAL_VARIANTS];
        let mut plants = [0; CORAL_COLORS];
        let mut fans = [[0; FAN_DIRECTIONS]; CORAL_COLORS];
        let mut wall_fans = [[0; WALL_DIRECTIONS]; CORAL_COLORS];
        for color in 0..CORAL_COLORS {
            kinds[color] = table.kind(LIVE_BLOCKS[color]);
            kinds[CORAL_COLORS + color] = table.kind(DEAD_BLOCKS[color]);
            blocks[color] = table.name(LIVE_BLOCKS[color]);
            blocks[CORAL_COLORS + color] = table.name(DEAD_BLOCKS[color]);
            let plant = table.name(PLANTS[color]);
            plants[color] = table.flooded(plant);
            for (direction, slot) in fans[color].iter_mut().enumerate() {
                let fan = table.get(FANS[color], &[state(&CORAL_FAN_DIRECTION, int(direction as i32))]);
                *slot = table.flooded(fan);
            }
            for (direction, slot) in wall_fans[color].iter_mut().enumerate() {
                let wall_fan = table.get(WALL_FANS[color], &[state(&CORAL_DIRECTION, int(direction as i32))]);
                *slot = table.flooded(wall_fan);
            }
        }
        Self {
            water_kind,
            stone_kind,
            kinds,
            blocks,
            plants,
            fans,
            wall_fans,
        }
    }

    fn index(&self, region: &Region, pos: IVec3) -> Option<usize> {
        let kind = region.blocks.entry(read(region, pos)).kind;
        self.kinds.iter().position(|&known| known == kind)
    }

    pub(super) fn is_coral(&self, region: &Region, pos: IVec3) -> bool {
        self.index(region, pos).is_some()
    }

    pub(super) fn is_live_coral(&self, region: &Region, pos: IVec3) -> bool {
        self.index(region, pos).is_some_and(|index| index < CORAL_COLORS)
    }

    pub(super) fn can_overwrite(&self, region: &Region, pos: IVec3, color: usize) -> bool {
        self.index(region, pos).is_none_or(|index| index == color)
    }

    pub(super) fn submerged(&self, region: &Region, pos: IVec3) -> bool {
        is_water(region, pos) || region.blocks.entry(read(region, pos)).flooded
    }

    pub(super) fn fits(&self, region: &Region, pos: IVec3) -> bool {
        self.submerged(region, pos + IVec3::Y) && [DOWN, NORTH, SOUTH, WEST, EAST].iter().all(|&face| !region.blocks.is_air(read(region, pos + FACES[face])))
    }

    pub(super) fn water(&self, region: &Region, pos: IVec3) -> bool {
        region.blocks.entry(read(region, pos)).kind == self.water_kind
    }

    pub fn place<D: Draw>(&self, region: &mut Region, random: &mut D, origin: IVec3) -> bool {
        let ceiling = region.sea_level() - CORAL_DEPTH;
        let mut pos = origin;
        while pos.y < ceiling && !self.water(region, pos) {
            pos.y += 1;
        }
        if pos.y >= ceiling || !is_solid_blocking(region, pos - IVec3::Y) || self.is_live_coral(region, pos - IVec3::Y) {
            return false;
        }
        let color = random.below(CORAL_COLORS as i32) as usize;
        let mut reef = Reef::new(self, region, random);
        reef.place_coral(pos, color);
        reef.decorate();
        true
    }
}
