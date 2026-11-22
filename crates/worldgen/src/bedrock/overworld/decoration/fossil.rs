use super::world::read;
use crate::shared::CHUNK_WIDTH;
use crate::shared::blocks::{BlockId, BlockTable};
use crate::shared::chunk::region::{Heightmap, Region};
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::{Probed, Rotation, StateKind, StructureData, Template, TemplateLibrary, register_named};
use glam::IVec3;
use serde_json::Value;
use std::sync::Arc;

const TEMPLATES: [&str; 8] = [
    "fossils/fossil_spine_01",
    "fossils/fossil_spine_02",
    "fossils/fossil_spine_03",
    "fossils/fossil_spine_04",
    "fossils/fossil_skull_01",
    "fossils/fossil_skull_02",
    "fossils/fossil_skull_03",
    "fossils/fossil_skull_04",
];
const ORE_BLOCK_KEY: &str = "ore_block";
const MAX_EMPTY_CORNERS_KEY: &str = "max_empty_corners";
const BONE_INTEGRITY: f32 = 0.9;
const ORE_INTEGRITY: f32 = 0.1;
const ROTATIONS: usize = 4;
const FOSSIL_DROP: i32 = 15;
const FOSSIL_DEPTH: i32 = 10;
const BOTTOM_MARGIN: i32 = 10;

#[derive(Debug)]
struct PartBlock {
    x: u8,
    y: u16,
    z: u8,
    turned: [BlockId; ROTATIONS],
}

#[derive(Debug)]
struct Part {
    size: (i32, i32, i32),
    blocks: Vec<PartBlock>,
}

#[derive(Debug)]
pub struct FossilParts {
    parts: Vec<Part>,
}

#[derive(Debug)]
pub struct Fossil {
    parts: Arc<FossilParts>,
    ore: BlockId,
    max_empty_corners: i32,
}

fn turned_id(table: &mut BlockTable, library: &TemplateLibrary, state: u16, rotation: Rotation) -> Option<BlockId> {
    if library.kind(state) != StateKind::Block {
        return None;
    }
    let entry = library.entry(library.rotated(state, rotation as usize));
    let id = register_named(table, &entry.name, &entry.states);
    Some(if entry.waterlogged { table.flooded(id) } else { id })
}

fn part(table: &mut BlockTable, library: &TemplateLibrary, template: &Template) -> Vec<PartBlock> {
    template
        .blocks
        .iter()
        .filter_map(|block| {
            let mut turned = [0; ROTATIONS];
            for (rotation, slot) in turned.iter_mut().enumerate() {
                *slot = turned_id(table, library, block.state, Rotation::from_index(rotation as i32))?;
            }
            Some(PartBlock {
                x: block.x,
                y: block.y,
                z: block.z,
                turned,
            })
        })
        .collect()
}

impl FossilParts {
    pub fn load(table: &mut BlockTable) -> Option<Arc<Self>> {
        let data = StructureData::shared()?;
        let library = data.library.as_ref()?;
        let templates: Vec<Arc<Template>> = TEMPLATES.iter().map(|name| library.get(name)).collect::<Option<_>>()?;
        let parts = templates
            .iter()
            .map(|template| Part {
                size: template.size,
                blocks: part(table, library, template),
            })
            .collect();
        Some(Arc::new(Self { parts }))
    }

    fn count(&self) -> i32 {
        self.parts.len() as i32
    }
}

fn empty(region: &Region, pos: IVec3) -> bool {
    let block = read(region, pos);
    region.blocks.is_air(block) || region.blocks.entry(block).liquid
}

impl Fossil {
    pub fn parse(table: &mut BlockTable, body: &'static Value, parts: Option<Arc<FossilParts>>) -> Option<Self> {
        let parts = parts?;
        let ore: &'static str = body.get(ORE_BLOCK_KEY)?.as_str()?;
        let ore = table.name(ore);
        let max_empty_corners = body.get(MAX_EMPTY_CORNERS_KEY).and_then(Value::as_i64).unwrap_or_default() as i32;
        Some(Self { parts, ore, max_empty_corners })
    }

    pub fn place(&self, region: &mut Region, random: &mut MersenneTwister, at: IVec3) -> Probed<bool> {
        if region.in_structure(at)? {
            return Ok(false);
        }
        if self.parts.parts.is_empty() {
            return Ok(false);
        }
        let rotation = Rotation::from_index(random.next_int_bounded(ROTATIONS as i32));
        let part = &self.parts.parts[random.next_int_bounded(self.parts.count()) as usize];
        let (size_x, size_y, size_z) = part.size;
        let (wide, deep) = if matches!(rotation, Rotation::Clockwise90 | Rotation::CounterClockwise90) {
            (size_z, size_x)
        } else {
            (size_x, size_z)
        };
        let x = at.x + random.next_int_bounded(CHUNK_WIDTH - wide);
        let z = at.z + random.next_int_bounded(CHUNK_WIDTH - deep);
        let mut top = at.y;
        for dx in 0..wide {
            for dz in 0..deep {
                top = top.min(region.height(Heightmap::AboveTopSolid, x + dx, z + dz));
            }
        }
        let y = (top - random.next_int_bounded(FOSSIL_DEPTH) - FOSSIL_DROP).max(region.min_y() + BOTTOM_MARGIN);
        let (far_x, _, far_z) = rotation.apply((size_x - 1, 0, size_z - 1));
        let corners = [x, x + far_x]
            .into_iter()
            .flat_map(|cx| [y, y + size_y - 1].into_iter().flat_map(move |cy| [z, z + far_z].into_iter().map(move |cz| IVec3::new(cx, cy, cz))));
        if corners.filter(|&pos| empty(region, pos)).count() as i32 > self.max_empty_corners {
            return Ok(false);
        }
        let seed = random.seed();
        for (integrity, ore) in [(BONE_INTEGRITY, None), (ORE_INTEGRITY, Some(self.ore))] {
            let mut placement = MersenneTwister::new(seed);
            for block in &part.blocks {
                if integrity < placement.next_float() {
                    continue;
                }
                let (dx, dy, dz) = rotation.apply((block.x as i32, block.y as i32, block.z as i32));
                let placed = ore.unwrap_or(block.turned[rotation as usize]);
                region.set(IVec3::new(x + dx, y + dy, z + dz), placed);
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_listed_fossil_is_in_the_pack() {
        let pack = bedrock_vanilla::structures();
        for name in TEMPLATES {
            assert!(pack.get(&format!("{name}.bin")).is_some(), "{name}");
        }
        assert_eq!(pack.names().iter().filter(|name| name.starts_with("fossils/")).count(), TEMPLATES.len());
    }
}
