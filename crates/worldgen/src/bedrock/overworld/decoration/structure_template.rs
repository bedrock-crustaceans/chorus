use crate::shared::blocks::{BlockId, BlockTable, Blocks};
use crate::shared::chunk::region::{Heightmap, Region};
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::{Rotation, StateKind, StructureData, Template, TemplateLibrary, register_named};
use crate::shared::tags::Tag;
use glam::IVec3;
use serde_json::Value;
use std::sync::Arc;

const STRUCTURE_NAME_KEY: &str = "structure_name";
const ADJUSTMENT_RADIUS_KEY: &str = "adjustment_radius";
const GROUND_LEVEL_KEY: &str = "ground_level";
const ROTATE_AROUND_CENTER_KEY: &str = "rotate_around_center";
const FACING_DIRECTION_KEY: &str = "facing_direction";
const CONSTRAINTS_KEY: &str = "constraints";
const LEVELED_KEY: &str = "leveled";
const UNBURIED_KEY: &str = "unburied";
const GROUNDED_KEY: &str = "grounded";
const BLOCK_INTERSECTION_KEY: &str = "block_intersection";
const BLOCK_ALLOWLIST_KEY: &str = "block_allowlist";
const BLOCK_WHITELIST_KEY: &str = "block_whitelist";
const MOTION_BLOCKING_KEY: &str = "only_check_intersection_for_motion_blocking_blocks";
const MAX_STEEPNESS_KEY: &str = "max_steepness";
const TAGS_KEY: &str = "tags";
const DEFAULT_MAX_STEEPNESS: i32 = 2;
const DEFAULT_GROUND_LEVEL: i32 = 0;
const ROTATIONS: usize = 4;
const SOUTH: u8 = 0;
const WEST: u8 = 1;
const NORTH: u8 = 2;
const EAST: u8 = 3;
const GRAVEL: &str = "minecraft:gravel";

#[derive(Debug)]
struct PartBlock {
    x: i32,
    y: i32,
    z: i32,
    turned: [BlockId; ROTATIONS],
    motion_blocking: bool,
}

#[derive(Debug, Default)]
struct Allowlist {
    names: Vec<&'static str>,
    tags: Vec<Tag>,
}

impl Allowlist {
    fn parse(entry: &'static Value) -> Self {
        let mut allowlist = Self::default();
        let values = entry.get(BLOCK_ALLOWLIST_KEY).or_else(|| entry.get(BLOCK_WHITELIST_KEY));
        for item in values.and_then(Value::as_array).into_iter().flatten() {
            if let Some(name) = item.as_str() {
                allowlist.names.push(name);
                continue;
            }
            if let Some(expression) = item.get(TAGS_KEY).and_then(Value::as_str) {
                for tag in expression.split('\'').skip(1).step_by(2) {
                    match tag {
                        "dirt" => allowlist.tags.push(Tag::Dirt),
                        "sand" => allowlist.tags.push(Tag::Sand),
                        "gravel" => allowlist.names.push(GRAVEL),
                        _ => {}
                    }
                }
            }
        }
        allowlist
    }

    fn allows(&self, blocks: &Blocks, block: BlockId) -> bool {
        self.names.contains(&blocks.name_of(block)) || self.tags.iter().any(|tag| blocks.is(block, *tag))
    }
}

#[derive(Debug)]
pub struct StructureTemplate {
    blocks: Vec<PartBlock>,
    size: (i32, i32, i32),
    adjustment_radius: i32,
    ground_level: i32,
    rotate_around_center: bool,
    facing: Option<u8>,
    leveled: Option<i32>,
    unburied: bool,
    grounded: bool,
    allowlist: Option<Allowlist>,
    only_motion_blocking: bool,
}

fn turned_id(table: &mut BlockTable, library: &TemplateLibrary, state: u16, rotation: Rotation) -> Option<BlockId> {
    if library.kind(state) != StateKind::Block {
        return None;
    }
    let entry = library.entry(library.rotated(state, rotation as usize));
    let id = register_named(table, &entry.name, &entry.states);
    Some(if entry.waterlogged { table.flooded(id) } else { id })
}

fn resolve_template(library: &TemplateLibrary, name: &str) -> Option<Arc<Template>> {
    if let Some(template) = library.get(name) {
        return Some(template);
    }
    let slashed = name.replacen(':', "/", 1);
    if let Some(template) = library.get(&slashed) {
        return Some(template);
    }
    let path = name.rsplit_once(':').map_or(name, |(_, path)| path);
    let mut matches = library.names().filter(|candidate| candidate.rsplit('/').next() == Some(path));
    let matched = matches.next()?;
    matches.next().is_none().then(|| library.get(matched)).flatten()
}

fn facing_rotation(facing: Option<&str>) -> Option<u8> {
    match facing.unwrap_or("random") {
        "south" => Some(SOUTH),
        "west" => Some(WEST),
        "north" => Some(NORTH),
        "east" => Some(EAST),
        "random" => None,
        _ => None,
    }
}

impl StructureTemplate {
    pub fn parse(table: &mut BlockTable, body: &'static Value) -> Option<Self> {
        let data = StructureData::shared()?;
        let library = data.library.as_ref()?;
        let name = body.get(STRUCTURE_NAME_KEY)?.as_str()?;
        let template = resolve_template(library, name)?;
        let size = template.size;
        let blocks = template
            .blocks
            .iter()
            .filter_map(|block| {
                let mut turned = [0; ROTATIONS];
                for (rotation, slot) in turned.iter_mut().enumerate() {
                    *slot = turned_id(table, library, block.state, Rotation::from_index(rotation as i32))?;
                }
                let motion_blocking = table.registered().get(turned[0] as usize).is_some_and(|entry| entry.solid);
                Some(PartBlock {
                    x: block.x as i32,
                    y: block.y as i32,
                    z: block.z as i32,
                    turned,
                    motion_blocking,
                })
            })
            .collect();
        let constraints = body.get(CONSTRAINTS_KEY);
        let leveled = constraints
            .and_then(|constraints| constraints.get(LEVELED_KEY))
            .map(|leveled| leveled.get(MAX_STEEPNESS_KEY).and_then(Value::as_i64).unwrap_or(DEFAULT_MAX_STEEPNESS as i64) as i32);
        let block_intersection = constraints.and_then(|constraints| constraints.get(BLOCK_INTERSECTION_KEY));
        let allowlist = block_intersection.map(Allowlist::parse);
        let only_motion_blocking = block_intersection
            .and_then(|intersection| intersection.get(MOTION_BLOCKING_KEY))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        Some(Self {
            blocks,
            size,
            adjustment_radius: body.get(ADJUSTMENT_RADIUS_KEY).and_then(Value::as_i64).unwrap_or(0) as i32,
            ground_level: (body.get(GROUND_LEVEL_KEY).and_then(Value::as_i64).unwrap_or(DEFAULT_GROUND_LEVEL as i64) as i32).clamp(0, (size.1 - 1).max(0)),
            rotate_around_center: body.get(ROTATE_AROUND_CENTER_KEY).and_then(Value::as_bool).unwrap_or(false),
            facing: facing_rotation(body.get(FACING_DIRECTION_KEY).and_then(Value::as_str)),
            leveled,
            unburied: constraints.is_some_and(|constraints| constraints.get(UNBURIED_KEY).is_some()),
            grounded: constraints.is_some_and(|constraints| constraints.get(GROUNDED_KEY).is_some()),
            allowlist,
            only_motion_blocking,
        })
    }

    fn pivot(&self) -> (i32, i32) {
        if self.rotate_around_center { (self.size.0 / 2, self.size.2 / 2) } else { (0, 0) }
    }

    fn offset(&self, rotation: Rotation, local: (i32, i32, i32)) -> (i32, i32, i32) {
        let (pivot_x, pivot_z) = self.pivot();
        rotation.apply((local.0 - pivot_x, local.1, local.2 - pivot_z))
    }

    fn bounds(&self, rotation: Rotation) -> (i32, i32, i32, i32) {
        let corners = [(0, 0), (self.size.0 - 1, 0), (0, self.size.2 - 1), (self.size.0 - 1, self.size.2 - 1)];
        corners.iter().fold((i32::MAX, i32::MAX, i32::MIN, i32::MIN), |(min_x, min_z, max_x, max_z), &(x, z)| {
            let (dx, _, dz) = self.offset(rotation, (x, 0, z));
            (min_x.min(dx), min_z.min(dz), max_x.max(dx), max_z.max(dz))
        })
    }

    fn placed_position(&self, rotation: Rotation, origin: IVec3, block: &PartBlock) -> IVec3 {
        let (dx, dy, dz) = self.offset(rotation, (block.x, block.y, block.z));
        origin + IVec3::new(dx, dy, dz)
    }

    fn intersects(&self, region: &Region, rotation: Rotation, origin: IVec3) -> bool {
        let Some(allowlist) = &self.allowlist else { return true };
        self.blocks.iter().filter(|block| !self.only_motion_blocking || block.motion_blocking).all(|block| {
            let position = self.placed_position(rotation, origin, block);
            allowlist.allows(region.blocks, region.get(position))
        })
    }

    fn level(&self, region: &mut Region, rotation: Rotation, at: IVec3) -> bool {
        let Some(max_steepness) = self.leveled else { return true };
        let (min_x, min_z, max_x, max_z) = self.bounds(rotation);
        let (mid_x, mid_z) = ((min_x + max_x) / 2, (min_z + max_z) / 2);
        [(mid_x, min_z), (mid_x, max_z), (min_x, mid_z), (max_x, mid_z)].into_iter().all(|(dx, dz)| {
            let height = region.height(Heightmap::AboveTopSolid, at.x + dx, at.z + dz);
            height > region.min_y() && (height - at.y).abs() <= max_steepness
        })
    }

    fn unburied(&self, region: &Region, rotation: Rotation, origin: IVec3) -> bool {
        if !self.unburied {
            return true;
        }
        let (min_x, min_z, max_x, max_z) = self.bounds(rotation);
        let width = (max_x - min_x + 1) as usize;
        let depth = (max_z - min_z + 1) as usize;
        let mut tops = vec![i32::MIN; width * depth];
        for block in &self.blocks {
            let (dx, _, dz) = self.offset(rotation, (block.x, block.y, block.z));
            let index = (dx - min_x) as usize * depth + (dz - min_z) as usize;
            tops[index] = tops[index].max(block.y);
        }
        self.blocks.iter().all(|block| {
            let (dx, _, dz) = self.offset(rotation, (block.x, block.y, block.z));
            let index = (dx - min_x) as usize * depth + (dz - min_z) as usize;
            if block.y != tops[index] {
                return true;
            }
            let above = origin + IVec3::new(dx, block.y + 1, dz);
            region.blocks.is_air(region.get(above))
        })
    }

    fn grounded(&self, region: &Region, rotation: Rotation, origin: IVec3) -> bool {
        if !self.grounded {
            return true;
        }
        self.blocks.iter().filter(|block| block.y == 0).all(|block| {
            let below = self.placed_position(rotation, origin, block) - IVec3::Y;
            let entry = region.blocks.entry(region.get(below));
            entry.solid && !entry.liquid
        })
    }

    fn fits(&self, region: &mut Region, rotation: Rotation, origin: IVec3, at: IVec3) -> bool {
        self.intersects(region, rotation, origin) && self.level(region, rotation, at) && self.unburied(region, rotation, origin) && self.grounded(region, rotation, origin)
    }

    fn write(&self, region: &mut Region, rotation: Rotation, origin: IVec3) {
        for block in &self.blocks {
            region.set(self.placed_position(rotation, origin, block), block.turned[rotation as usize]);
        }
    }

    pub fn place(&self, region: &mut Region, random: &mut MersenneTwister, at: IVec3) -> bool {
        let rotation = Rotation::from_index(self.facing.unwrap_or_else(|| random.next_int_bounded(ROTATIONS as i32) as u8) as i32);
        let origin = IVec3::new(at.x, at.y - self.ground_level, at.z);
        if self.adjustment_radius <= 0 {
            if !self.fits(region, rotation, origin, at) {
                return false;
            }
            self.write(region, rotation, origin);
            return true;
        }
        let mut offsets: Vec<(i32, i32)> = (-self.adjustment_radius..=self.adjustment_radius)
            .flat_map(|dx| (-self.adjustment_radius..=self.adjustment_radius).map(move |dz| (dx, dz)))
            .collect();
        offsets.sort_by_key(|&(dx, dz)| (dx * dx + dz * dz, dz, dx));
        for (dx, dz) in offsets {
            let target = at + IVec3::new(dx, 0, dz);
            if self.fits(region, rotation, origin + IVec3::new(dx, 0, dz), target) {
                self.write(region, rotation, origin + IVec3::new(dx, 0, dz));
                return true;
            }
        }
        false
    }
}
