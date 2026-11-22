use super::super::world::read;
use super::support::is_wet;
use crate::shared::blocks::{BlockId, BlockTable};
use crate::shared::chunk::region::Region;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::survival::Direction;
use chorus_block::block_id::{BUBBLE_COLUMN, MAGMA};
use glam::IVec3;
use serde_json::Value;

#[derive(Debug)]
pub struct Blob {
    block: BlockId,
    radius: i32,
    probability: f32,
    exposed: Direction,
    ticks_after_placing: bool,
}

const MAGMA_TICK_DELAY: u64 = 1;

fn exposed_face(name: &str) -> Option<Direction> {
    Direction::ALL.into_iter().find(|direction| direction.name() == name)
}

fn offsets(radius: i32) -> Vec<IVec3> {
    let mut out = Vec::new();
    for ring in 0..=3 * radius {
        let reach = ring.min(radius);
        for dx in -reach..=reach {
            let by = (ring - dx.abs()).min(radius);
            for dy in -by..=by {
                let dz = ring - dx.abs() - dy.abs();
                if dz > radius {
                    continue;
                }
                out.push(IVec3::new(dx, dy, dz));
                if dz != 0 {
                    out.push(IVec3::new(dx, dy, -dz));
                }
            }
        }
    }
    out
}

impl Blob {
    pub fn parse(table: &mut BlockTable, body: &'static Value) -> Option<Self> {
        let name = body.get("places_block")?.as_str()?;
        Some(Self {
            block: table.name(name),
            ticks_after_placing: name == MAGMA,
            radius: body.get("placement_radius_around_floor")?.as_i64()? as i32,
            probability: body.get("placement_probability_per_valid_position")?.as_f64()? as f32,
            exposed: exposed_face(body.get("exposed_face")?.as_str()?)?,
        })
    }

    fn open(region: &Region, pos: IVec3) -> bool {
        let block = read(region, pos);
        let blocks = region.blocks;
        blocks.is_air(block) || is_wet(blocks, block) || blocks.name_of(block) == BUBBLE_COLUMN
    }

    fn enclosed(&self, region: &Region, pos: IVec3) -> bool {
        !Self::open(region, pos) && Direction::ALL.iter().all(|&face| face == self.exposed || !Self::open(region, pos + face.offset()))
    }

    pub fn place(&self, region: &mut Region, random: &mut MersenneTwister, at: IVec3) -> bool {
        let mut placed = false;
        for offset in offsets(self.radius) {
            let pos = at - IVec3::Y + offset;
            if random.next_float() <= self.probability && self.enclosed(region, pos) {
                region.set(pos, self.block);
                if self.ticks_after_placing {
                    region.schedule_on_chunk(pos, false, MAGMA_TICK_DELAY);
                }
                placed = true;
            }
        }
        placed
    }
}
