use super::ore::Draw;
use super::world::{is_air, read};
use crate::shared::blocks::{BlockId, BlockTable};
use crate::shared::chunk::region::Region;
use glam::IVec3;
use serde_json::Value;

const DEFAULT_MIN_RADIUS: i32 = 2;

#[derive(Debug)]
pub struct Disk {
    block: BlockId,
    covered_block: BlockId,
    radius: i32,
    min_radius: i32,
    half_height: i32,
    needs: Option<Vec<u16>>,
    replaces: Vec<u16>,
}

pub(super) fn kinds(table: &mut BlockTable, body: &'static Value, key: &str) -> Option<Vec<u16>> {
    body.get(key)?.as_array()?.iter().map(|name| Some(table.kind(name.as_str()?))).collect()
}

impl Disk {
    pub fn parse(table: &mut BlockTable, body: &'static Value) -> Option<Self> {
        let needs = match body.get("needs") {
            Some(_) => Some(kinds(table, body, "needs")?),
            None => None,
        };
        let replaces = kinds(table, body, "replaces")?;
        let block = table.name(body.get("block")?.as_str()?);
        let covered_block = match body.get("covered_block") {
            Some(covered) => table.name(covered.as_str()?),
            None => block,
        };
        Some(Self {
            block,
            covered_block,
            radius: body.get("radius")?.as_i64()? as i32,
            min_radius: body.get("min_radius").and_then(Value::as_i64).map_or(DEFAULT_MIN_RADIUS, |radius| radius as i32),
            half_height: body.get("half_height")?.as_i64()? as i32,
            needs,
            replaces,
        })
    }

    pub fn place<D: Draw>(&self, region: &mut Region, random: &mut D, origin: IVec3) -> bool {
        if let Some(needs) = &self.needs
            && !needs.contains(&region.blocks.entry(read(region, origin)).kind)
        {
            return false;
        }
        let radius = random.below(self.radius - self.min_radius) + self.min_radius;
        let mut placed = false;
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                if dx * dx + dz * dz > radius * radius {
                    continue;
                }
                for dy in -self.half_height..=self.half_height {
                    let pos = origin + IVec3::new(dx, dy, dz);
                    if self.replaces.contains(&region.blocks.entry(read(region, pos)).kind) {
                        let block = if is_air(region, pos + IVec3::Y) { self.block } else { self.covered_block };
                        placed |= region.set(pos, block);
                    }
                }
            }
        }
        placed
    }
}
