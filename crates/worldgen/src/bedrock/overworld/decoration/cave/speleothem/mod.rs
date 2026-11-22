mod cluster;
mod large;

use super::super::world::read;
use super::support::{is_lava, is_open_or_wet, is_water_source};
use super::{BlockSet, CaveDraw};
use crate::shared::blocks::{BlockId, BlockTable, flag, state, text};
use crate::shared::chunk::region::Region;
use crate::shared::survival::Direction;
use crate::shared::tags::Tag;
use chorus_block::block_id::*;
use chorus_block::state::common::{DRIPSTONE_THICKNESS, HANGING};
use glam::IVec3;

const THICKNESSES: [&str; 5] = ["tip", "frustum", "middle", "base", "merge"];
const THICKNESS_TIP: usize = 0;
const THICKNESS_FRUSTUM: usize = 1;
const THICKNESS_MIDDLE: usize = 2;
const THICKNESS_BASE: usize = 3;
const THICKNESS_MERGE: usize = 4;
const DRIPSTONE_REPLACEABLE: [&str; 5] = [DIRT, COARSE_DIRT, STONE, TUFF, DEEPSLATE];
const FACE_SKIP_CHANCE: f32 = 0.3;
const TALL_CHANCE: f32 = 0.2;
const FACE_COUNT: i32 = 6;

#[derive(Debug)]
pub struct Family {
    base: BlockId,
    spike_kind: u16,
    spikes: [[BlockId; 5]; 2],
    replaceable: BlockSet,
}

impl Family {
    fn new(table: &mut BlockTable, base: &'static str, spike: &'static str, replaceable: &[&'static str]) -> Self {
        let spikes = [false, true].map(|hanging| {
            THICKNESSES.map(|thickness| {
                let block = table.get(spike, &[state(&HANGING, flag(hanging)), state(&DRIPSTONE_THICKNESS, text(thickness))]);
                table.flooded(block);
                block
            })
        });
        Self {
            base: table.name(base),
            spike_kind: table.kind(spike),
            spikes,
            replaceable: BlockSet::from_names(table, replaceable),
        }
    }

    pub fn dripstone(table: &mut BlockTable) -> Self {
        Self::new(table, DRIPSTONE_BLOCK, POINTED_DRIPSTONE, &DRIPSTONE_REPLACEABLE)
    }

    pub fn sulfur(table: &mut BlockTable) -> Self {
        Self::new(table, SULFUR, SULFUR_SPIKE, Tag::SulfurSpikeReplaceableBlocks.members())
    }

    fn is_base(&self, region: &Region, pos: IVec3) -> bool {
        let block = read(region, pos);
        self.replaceable.contains(region.blocks, block) || block == self.base
    }

    fn is_spike(&self, region: &Region, block: BlockId) -> bool {
        region.blocks.is_kind(block, self.spike_kind)
    }

    fn replace(&self, region: &mut Region, pos: IVec3) -> bool {
        let fits = self.replaceable.contains(region.blocks, read(region, pos));
        if fits {
            region.set(pos, self.base);
        }
        fits
    }

    fn grow(&self, region: &mut Region, origin: IVec3, up: bool, height: i32, merge: bool) {
        let direction = if up { Direction::Up } else { Direction::Down };
        for index in 0..height.max(0) {
            let pos = origin + direction.offset() * index;
            let block = read(region, pos);
            if !is_open_or_wet(region.blocks, block) || is_lava(region.blocks, block) {
                return;
            }
            let thickness = match height - index {
                1 if merge => THICKNESS_MERGE,
                1 => THICKNESS_TIP,
                2 => THICKNESS_FRUSTUM,
                _ if index == 0 => THICKNESS_BASE,
                _ => THICKNESS_MIDDLE,
            };
            let spike = self.spikes[usize::from(!up)][thickness];
            let spike = if is_water_source(region.blocks, block) { region.blocks.flooded(spike) } else { spike };
            region.set(pos, spike);
        }
    }

    pub fn place_pointed<D: CaveDraw>(&self, region: &mut Region, random: &mut D, at: IVec3) -> bool {
        let below = self.is_base(region, at - IVec3::Y);
        let above = self.is_base(region, at + IVec3::Y);
        let up = match (below, above) {
            (true, true) => random.boolean(),
            (true, false) => true,
            (false, true) => false,
            (false, false) => return false,
        };
        let root = at + if up { IVec3::NEG_Y } else { IVec3::Y };
        self.replace(region, root);
        for face in Direction::ALL {
            if random.float() < FACE_SKIP_CHANCE {
                continue;
            }
            let near = root + face.offset();
            self.replace(region, near);
            if random.boolean() {
                continue;
            }
            let far = near + Direction::ALL[random.below(FACE_COUNT) as usize].offset();
            self.replace(region, far);
            if random.boolean() {
                continue;
            }
            let farthest = far + Direction::ALL[random.below(FACE_COUNT) as usize].offset();
            self.replace(region, farthest);
        }
        let step = if up { IVec3::Y } else { IVec3::NEG_Y };
        let taller = random.float() < TALL_CHANCE && is_open_or_wet(region.blocks, read(region, at + step));
        self.grow(region, at, up, if taller { 2 } else { 1 }, false);
        true
    }
}

#[derive(Debug)]
pub enum Speleothem {
    Pointed(Family),
    Cluster { family: Family, height: [i32; 2], wetness: [f32; 4] },
    Large(Family),
}

impl Speleothem {
    pub fn place<D: CaveDraw>(&self, region: &mut Region, random: &mut D, at: IVec3) -> bool {
        match self {
            Self::Pointed(family) => family.place_pointed(region, random, at),
            Self::Cluster { family, height, wetness } => cluster::place(family, region, random, *height, *wetness, at),
            Self::Large(family) => large::place(family, region, random, at),
        }
    }
}
