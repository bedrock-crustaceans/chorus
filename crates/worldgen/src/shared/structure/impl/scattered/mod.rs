mod builder;
pub use builder::TempleBuilder;
mod cellar;
mod desert;
mod hut;
mod jungle;
mod material;
mod piece;

use crate::shared::CHUNK_WIDTH;
use crate::shared::blocks::Blocks;
use crate::shared::chunk::proto::ProtoChunk;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::Probed;
pub use material::{Material, ScatteredBlocks};
use piece::Piece;

const PYRAMID_LOWERING: i32 = 3;
const PYRAMID_HEIGHT_CHUNK_SHIFT: (i32, i32) = (0, 0);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scattered {
    DesertPyramid,
    JungleTemple,
    SwampHut,
}

pub struct Site<'a> {
    pub start: (i32, i32),
    pub first_random: MersenneTwister,
    pub columns: &'a dyn Fn(i32, i32) -> Probed<(i32, i32)>,
}

fn surface_with_water((solid, top): (i32, i32)) -> i32 {
    top.max(solid)
}

pub fn pyramid_height_chunk((x, z): (i32, i32)) -> (i32, i32) {
    (x + PYRAMID_HEIGHT_CHUNK_SHIFT.0, z + PYRAMID_HEIGHT_CHUNK_SHIFT.1)
}

impl Scattered {
    pub fn of_biome(biome: i32, registry: &crate::shared::structure::StructureRegistry) -> Option<Self> {
        match biome {
            _ if registry.biomes("desert_pyramid").contains(&biome) => Some(Self::DesertPyramid),
            _ if registry.biomes("jungle_temple").contains(&biome) => Some(Self::JungleTemple),
            _ if registry.biomes("swamp_hut").contains(&biome) => Some(Self::SwampHut),
            _ => None,
        }
    }

    fn size(self) -> (i32, i32) {
        match self {
            Self::DesertPyramid => (desert::WIDTH, desert::DEPTH),
            Self::JungleTemple => (jungle::WIDTH, jungle::DEPTH),
            Self::SwampHut => (hut::WIDTH, hut::DEPTH),
        }
    }

    fn columns_in(self, start: (i32, i32), chunk: (i32, i32)) -> impl Iterator<Item = (i32, i32)> {
        let (width, depth) = self.size();
        let (x0, z0) = (start.0 * CHUNK_WIDTH, start.1 * CHUNK_WIDTH);
        let (min_x, min_z) = (chunk.0 * CHUNK_WIDTH, chunk.1 * CHUNK_WIDTH);
        let (xs, zs) = (
            x0.max(min_x)..=(x0 + width - 1).min(min_x + CHUNK_WIDTH - 1),
            z0.max(min_z)..=(z0 + depth - 1).min(min_z + CHUNK_WIDTH - 1),
        );
        zs.flat_map(move |z| xs.clone().map(move |x| (x, z)))
    }

    fn floor(self, site: &Site, random: &mut MersenneTwister) -> Probed<Option<i32>> {
        let mut first = site.first_random.clone();
        let lowering = first.next_int_bounded(PYRAMID_LOWERING);
        random.next_int_bounded(PYRAMID_LOWERING);
        let chunk = pyramid_height_chunk(site.start);
        let mut lowest: Option<i32> = None;
        for (x, z) in self.columns_in(site.start, chunk) {
            let surface = surface_with_water((site.columns)(x, z)?);
            lowest = Some(lowest.map_or(surface, |known| known.min(surface)));
        }
        Ok(lowest.map(|lowest| lowest - lowering))
    }

    fn average_top(self, site: &Site) -> Probed<Option<i32>> {
        let (mut sum, mut count) = (0, 0);
        for (x, z) in self.columns_in(site.start, site.start) {
            sum += surface_with_water((site.columns)(x, z)?);
            count += 1;
        }
        Ok((count > 0).then(|| sum / count))
    }

    pub fn intersects(self, start: (i32, i32), chunk: (i32, i32)) -> bool {
        let (width, depth) = self.size();
        let (x0, z0) = (start.0 * CHUNK_WIDTH, start.1 * CHUNK_WIDTH);
        let (min_x, min_z) = (chunk.0 * CHUNK_WIDTH, chunk.1 * CHUNK_WIDTH);
        x0 < min_x + CHUNK_WIDTH && x0 + width > min_x && z0 < min_z + CHUNK_WIDTH && z0 + depth > min_z
    }

    pub fn post_process(self, chunk: &mut ProtoChunk, blocks: &Blocks, ids: &ScatteredBlocks, site: &Site, random: &mut MersenneTwister) -> Probed<()> {
        if !self.intersects(site.start, (chunk.min_x().div_euclid(CHUNK_WIDTH), chunk.min_z().div_euclid(CHUNK_WIDTH))) {
            return Ok(());
        }
        let y = match self {
            Self::DesertPyramid => self.floor(site, random)?,
            Self::JungleTemple => self.average_top(site)?,
            Self::SwampHut => self.average_top(site)?.map(|average| average + 1),
        };
        let Some(y) = y else { return Ok(()) };
        let mut piece = Piece {
            chunk,
            blocks,
            ids,
            origin: (site.start.0 * CHUNK_WIDTH, y, site.start.1 * CHUNK_WIDTH),
        };
        match self {
            Self::DesertPyramid => desert::post_process(&mut piece, random),
            Self::JungleTemple => jungle::post_process(&mut piece, random),
            Self::SwampHut => hut::post_process(&mut piece, random),
        }
        Ok(())
    }
}
