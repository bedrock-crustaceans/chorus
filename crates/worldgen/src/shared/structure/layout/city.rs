use std::cmp::Reverse;
use std::collections::BinaryHeap;

use super::CANDIDATE_ROTATIONS;
use super::catalogue::PoolCatalogue;
use super::entry::{Connector, Grounding};
use super::grow::{Part, middle, shuffle};
use crate::shared::CHUNK_WIDTH;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::BoundingBox;
use crate::shared::structure::place::Rotation;
use crate::shared::structure::template::TemplateLibrary;

const NAMESPACE: &str = "minecraft:";
const ROTATION_COUNT: i32 = 4;

pub struct CitySpec {
    pub pool: &'static str,
    pub anchor: &'static str,
    pub max_depth: i32,
    pub radius: i32,
    pub ground: i32,
}

pub struct QueuedSpec<'a> {
    pub pools: &'a [&'a str],
    pub anchor: Option<&'a str>,
    pub max_depth: i32,
    pub radius: i32,
    pub ground: i32,
}

#[derive(Default)]
struct Space {
    allowed: Vec<BoundingBox>,
    occupied: Vec<BoundingBox>,
}

impl Space {
    fn admits(&self, bounds: &BoundingBox) -> bool {
        let inside =
            |allowed: &BoundingBox| allowed.x0 <= bounds.x0 && bounds.x1 <= allowed.x1 && allowed.y0 <= bounds.y0 && bounds.y1 <= allowed.y1 && allowed.z0 <= bounds.z0 && bounds.z1 <= allowed.z1;
        self.allowed.iter().any(inside) && !self.occupied.iter().any(|taken| bounds.intersects(taken))
    }
}

struct Growth<'a> {
    catalogue: &'a PoolCatalogue,
    library: &'a TemplateLibrary,
    random: &'a mut MersenneTwister,
    ground: i32,
    max_depth: i32,
    pieces: Vec<Part>,
    spaces: Vec<Space>,
    queue: BinaryHeap<(Reverse<usize>, usize, usize)>,
    queued: usize,
}

impl CitySpec {
    pub fn assemble(&self, catalogue: &PoolCatalogue, library: &TemplateLibrary, random: &mut MersenneTwister, chunk: (i32, i32)) -> Vec<Part> {
        let spec = QueuedSpec {
            pools: &[self.pool],
            anchor: Some(self.anchor),
            max_depth: self.max_depth,
            radius: self.radius,
            ground: self.ground,
        };
        spec.assemble(catalogue, library, random, chunk)
    }
}

impl QueuedSpec<'_> {
    pub fn assemble(&self, catalogue: &PoolCatalogue, library: &TemplateLibrary, random: &mut MersenneTwister, chunk: (i32, i32)) -> Vec<Part> {
        let rotation = Rotation::from_index(random.next_int_bounded(ROTATION_COUNT));
        let pool_index = if self.pools.len() > 1 { random.next_int_bounded(self.pools.len() as i32) as usize } else { 0 };
        let pool = catalogue.pool(self.pools[pool_index]).expect("queued start pool");
        let entry = pool.list[random.next_int_bounded(pool.list.len() as i32) as usize];
        let column = (chunk.0 * CHUNK_WIDTH, chunk.1 * CHUNK_WIDTH);
        let (shift_x, anchor_y, shift_z) = match self.anchor {
            Some(anchor) => {
                let mut anchors = catalogue.entries[entry].jigsaws(library, (column.0, 0, column.1), rotation);
                shuffle(random, &mut anchors);
                anchors
                    .iter()
                    .find(|connector| connector.name == anchor)
                    .map_or((0, 0, 0), |connector| (column.0 - connector.pos.0, connector.pos.1, column.1 - connector.pos.2))
            }
            None => (0, 0, 0),
        };
        let mut start = Part::new(catalogue, entry, (column.0 + shift_x, 0, column.1 + shift_z), rotation, 0);
        let bounds = start.bounds;
        start.shift_y(self.ground - anchor_y - bounds.y0 - 1);
        let bounds = start.bounds;
        let (centre_x, centre_y, centre_z) = (middle(bounds.x0, bounds.x1), middle(bounds.y0, bounds.y1), middle(bounds.z0, bounds.z1));
        let limit = BoundingBox::new(
            centre_x - self.radius,
            centre_y - self.radius,
            centre_z - self.radius,
            centre_x + self.radius,
            centre_y + self.radius,
            centre_z + self.radius,
        );
        let mut growth = Growth {
            catalogue,
            library,
            random,
            ground: self.ground,
            max_depth: self.max_depth,
            pieces: vec![start],
            spaces: vec![Space {
                allowed: vec![limit],
                occupied: vec![bounds],
            }],
            queue: BinaryHeap::new(),
            queued: 0,
        };
        growth.enqueue(0, 0);
        while let Some((_, piece, space)) = growth.queue.pop() {
            growth.expand(piece, space);
        }
        growth.pieces
    }
}

impl Growth<'_> {
    fn enqueue(&mut self, piece: usize, space: usize) {
        self.queued += 1;
        self.queue.push((Reverse(self.queued), piece, space));
    }

    fn rigid(&self, entry: usize) -> bool {
        self.catalogue.entries[entry].grounding() == Grounding::Fixed
    }

    fn expand(&mut self, index: usize, space: usize) {
        let piece = self.pieces[index].clone();
        if piece.depth > self.max_depth {
            return;
        }
        let catalogue = self.catalogue;
        let mut connectors = catalogue.entries[piece.entry].jigsaws(self.library, piece.origin, piece.rotation);
        shuffle(self.random, &mut connectors);
        connectors.sort_by_key(|connector| Reverse(connector.selection_priority));
        for connector in &connectors {
            let name = connector.pool.strip_prefix(NAMESPACE).unwrap_or(connector.pool);
            let Some(pool) = catalogue.pool(name).filter(|pool| !pool.list.is_empty()) else { continue };
            let fallback = pool.fallback.as_deref().and_then(|name| catalogue.pool(name)).filter(|fallback| !fallback.list.is_empty());
            if piece.depth == self.max_depth {
                if let Some(fallback) = fallback {
                    self.try_pool(index, &piece, connector, &fallback.list, space);
                }
            } else if !self.try_pool(index, &piece, connector, &pool.list, space)
                && let Some(fallback) = fallback
            {
                self.try_pool(index, &piece, connector, &fallback.list, space);
            }
        }
    }

    fn try_pool(&mut self, _index: usize, source: &Part, connector: &Connector, list: &[usize], space: usize) -> bool {
        let mut order = list.to_vec();
        shuffle(self.random, &mut order);
        let offset = connector.front.offset();
        let target = (connector.pos.0 + offset.0, connector.pos.1 + offset.1, connector.pos.2 + offset.2);
        let inside = source.bounds.contains(target.0, target.1, target.2);
        let mut space = space;
        let catalogue = self.catalogue;
        for entry_index in order {
            let entry = &catalogue.entries[entry_index];
            if !entry.is_valid() {
                return true;
            }
            let mut rotations = CANDIDATE_ROTATIONS;
            shuffle(self.random, &mut rotations);
            for rotation in rotations {
                let mut candidates = entry.jigsaws(self.library, (0, 0, 0), rotation);
                shuffle(self.random, &mut candidates);
                for candidate in &candidates {
                    if !connector.can_attach(candidate, false) {
                        continue;
                    }
                    let mut origin = (target.0 - candidate.pos.0, target.1 - candidate.pos.1, target.2 - candidate.pos.2);
                    let mut bounds = entry.bounds(origin, rotation);
                    let bottom = if self.rigid(source.entry) && self.rigid(entry_index) {
                        connector.pos.1 - candidate.pos.1 + offset.1
                    } else {
                        connector.pos.1 - (source.bounds.y0 + candidate.pos.1) + self.ground - 1 + offset.1
                    };
                    let shift = bottom - bounds.y0;
                    bounds = bounds.shifted_y(shift);
                    origin.1 += shift;
                    if inside {
                        space += 1;
                        self.allow(space, source.bounds);
                    }
                    if !self.spaces[space].admits(&bounds) {
                        continue;
                    }
                    self.spaces[space].occupied.push(bounds);
                    let mut child = Part::new(catalogue, entry_index, origin, rotation, source.depth + 1);
                    child.ground = candidate.pos.1;
                    let at = self.pieces.len();
                    self.pieces.push(child);
                    self.enqueue(at, space);
                    return true;
                }
            }
        }
        false
    }

    fn allow(&mut self, space: usize, bounds: BoundingBox) {
        if space < self.spaces.len() {
            self.spaces[space].allowed.push(bounds);
        } else {
            self.spaces.push(Space {
                allowed: vec![bounds],
                occupied: Vec::new(),
            });
        }
    }
}
