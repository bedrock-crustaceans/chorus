use std::sync::Arc;

use super::processors::Processor;
use crate::shared::structure::BoundingBox;
use crate::shared::structure::place::Rotation;
use crate::shared::structure::template::{Template, TemplateLibrary};

const RING: [Dir; 4] = [Dir::North, Dir::East, Dir::South, Dir::West];
const NAMESPACE: &str = "minecraft:";
const FEATURE_JIGSAW_NAME: &str = "minecraft:bottom";
const FEATURE_JIGSAW_TARGET: &str = "minecraft:empty";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grounding {
    Fixed,
    FollowsTerrain,
}

#[derive(Debug, Clone)]
pub struct Capped {
    pub limit: usize,
    pub rules: Arc<[Processor]>,
}

pub enum EntryKind {
    Template {
        template: Arc<Template>,
        processors: Arc<[Processor]>,
        grounding: Grounding,
        foundation: u8,
        #[cfg(test)]
        aliases: Option<Arc<[(String, String)]>>,
        capped: Arc<[Capped]>,
        prepared: Arc<[PreparedJigsaw]>,
    },
    Feature(String),
    Missing,
    Empty,
}

#[derive(Debug)]
pub struct PreparedJigsaw {
    front: Dir,
    top: Dir,
    aligned: bool,
    pool: String,
}

impl PreparedJigsaw {
    pub fn all(template: &Template, aliases: Option<&[(String, String)]>) -> Arc<[PreparedJigsaw]> {
        template
            .jigsaws
            .iter()
            .map(|layout| {
                let (front, top) = layout.orientation.split_once('_').expect("layout orientation");
                let wanted = layout.pool.strip_prefix(NAMESPACE).unwrap_or(&layout.pool);
                let renamed = aliases.and_then(|aliases| aliases.iter().find(|(from, _)| from == wanted));
                PreparedJigsaw {
                    front: Dir::parse(front),
                    top: Dir::parse(top),
                    aligned: layout.joint == "aligned",
                    pool: renamed.map_or_else(|| layout.pool.clone(), |(_, to)| to.clone()),
                }
            })
            .collect()
    }
}

pub struct PoolEntry {
    pub kind: EntryKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl Dir {
    fn parse(name: &str) -> Dir {
        match name {
            "down" => Dir::Down,
            "up" => Dir::Up,
            "north" => Dir::North,
            "south" => Dir::South,
            "west" => Dir::West,
            "east" => Dir::East,
            other => panic!("bad direction {other}"),
        }
    }

    pub fn offset(self) -> (i32, i32, i32) {
        match self {
            Dir::Down => (0, -1, 0),
            Dir::Up => (0, 1, 0),
            Dir::North => (0, 0, -1),
            Dir::South => (0, 0, 1),
            Dir::West => (-1, 0, 0),
            Dir::East => (1, 0, 0),
        }
    }

    pub fn opposite(self) -> Dir {
        match self {
            Dir::Down => Dir::Up,
            Dir::Up => Dir::Down,
            Dir::North => Dir::South,
            Dir::South => Dir::North,
            Dir::West => Dir::East,
            Dir::East => Dir::West,
        }
    }

    pub fn vertical(self) -> bool {
        matches!(self, Dir::Down | Dir::Up)
    }

    fn rotate(self, rotation: Rotation) -> Dir {
        match RING.iter().position(|&direction| direction == self) {
            Some(index) => RING[(index + rotation as usize) % RING.len()],
            None => self,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Connector<'d> {
    pub pos: (i32, i32, i32),
    pub front: Dir,
    pub top: Dir,
    pub name: &'d str,
    pub target: &'d str,
    pub pool: &'d str,
    pub aligned: bool,
    pub final_state: u16,
    pub selection_priority: i32,
    pub placement_priority: i32,
}

impl Connector<'_> {
    pub fn can_attach(&self, other: &Connector, ignore_names: bool) -> bool {
        (ignore_names || self.target == other.name) && self.front == other.front.opposite() && !(self.front.vertical() && self.aligned && self.top != other.top)
    }
}

impl PoolEntry {
    pub fn is_valid(&self) -> bool {
        !matches!(self.kind, EntryKind::Empty)
    }

    pub fn is_feature(&self) -> bool {
        matches!(self.kind, EntryKind::Feature(_))
    }

    pub fn grounding(&self) -> Grounding {
        match self.kind {
            EntryKind::Template { grounding, .. } => grounding,
            _ => Grounding::Fixed,
        }
    }

    pub fn bounds(&self, (x, y, z): (i32, i32, i32), rotation: Rotation) -> BoundingBox {
        let (sx, sy, sz) = match &self.kind {
            EntryKind::Template { template, .. } => template.size,
            EntryKind::Feature(_) => return feature_bounds((x, y, z), rotation),
            _ => (1, 1, 1),
        };
        let (dx, dy, dz) = rotation.apply((sx - 1, sy - 1, sz - 1));
        BoundingBox::new(x.min(x + dx), y.min(y + dy), z.min(z + dz), x.max(x + dx), y.max(y + dy), z.max(z + dz))
    }

    pub fn jigsaws<'d>(&'d self, library: &TemplateLibrary, origin: (i32, i32, i32), rotation: Rotation) -> Vec<Connector<'d>> {
        match &self.kind {
            EntryKind::Template { template, prepared, .. } => template
                .jigsaws
                .iter()
                .zip(prepared.iter())
                .map(|(layout, prepared)| {
                    let block = template.blocks[layout.block];
                    let (dx, dy, dz) = rotation.apply((block.x as i32, block.y as i32, block.z as i32));
                    Connector {
                        pos: (origin.0 + dx, origin.1 + dy, origin.2 + dz),
                        front: prepared.front.rotate(rotation),
                        top: prepared.top.rotate(rotation),
                        name: &layout.name,
                        target: &layout.target,
                        pool: &prepared.pool,
                        aligned: prepared.aligned,
                        final_state: library.rotated(layout.final_state, rotation as usize),
                        selection_priority: layout.selection_priority,
                        placement_priority: layout.placement_priority,
                    }
                })
                .collect(),
            EntryKind::Feature(_) => vec![Connector {
                pos: origin,
                front: Dir::Down,
                top: Dir::South,
                name: FEATURE_JIGSAW_NAME,
                target: FEATURE_JIGSAW_TARGET,
                pool: FEATURE_JIGSAW_TARGET,
                aligned: false,
                final_state: 0,
                selection_priority: 0,
                placement_priority: 0,
            }],
            EntryKind::Missing | EntryKind::Empty => Vec::new(),
        }
    }
}

fn feature_bounds((x, y, z): (i32, i32, i32), rotation: Rotation) -> BoundingBox {
    let turn = rotation as usize;
    let (x0, x1) = if matches!(turn, 1 | 2) { (x + 1, x) } else { (x, x - 1) };
    let (z0, z1) = if matches!(turn, 2 | 3) { (z + 1, z) } else { (z, z - 1) };
    BoundingBox::new(x0, y, z0, x1, y - 1, z1)
}
