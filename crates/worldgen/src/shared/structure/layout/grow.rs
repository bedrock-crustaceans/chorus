use super::CANDIDATE_ROTATIONS;
use super::catalogue::PoolCatalogue;
use super::entry::{Connector, Grounding, PoolEntry};
use super::junction::Junction;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::place::Rotation;
use crate::shared::structure::template::TemplateLibrary;
use crate::shared::structure::{BoundingBox, Probed};

const NAMESPACE: &str = "minecraft:";

#[derive(Debug, Clone)]
pub struct Part {
    pub entry: usize,
    pub origin: (i32, i32, i32),
    pub rotation: Rotation,
    pub bounds: BoundingBox,
    pub depth: i32,
    pub ground: i32,
    pub junctions: Vec<Junction>,
}

impl Part {
    pub fn new(data: &PoolCatalogue, entry: usize, origin: (i32, i32, i32), rotation: Rotation, depth: i32) -> Self {
        Self {
            entry,
            origin,
            rotation,
            bounds: data.entries[entry].bounds(origin, rotation),
            depth,
            ground: 1,
            junctions: Vec::new(),
        }
    }

    pub fn shift_y(&mut self, dy: i32) {
        self.origin.1 += dy;
        self.bounds = self.bounds.shifted_y(dy);
    }
}

pub fn middle(a: i32, b: i32) -> i32 {
    a + (b - a + 1).div_euclid(2)
}

pub(super) fn shuffle<T>(random: &mut MersenneTwister, items: &mut [T]) {
    for index in 1..items.len() {
        let other = (random.next_u32() % (index as u32 + 1)) as usize;
        if other != index {
            items.swap(index, other);
        }
    }
}

pub struct Grower<'a> {
    pub data: &'a PoolCatalogue,
    library: &'a TemplateLibrary,
    random: &'a mut MersenneTwister,
    height: &'a dyn Fn(i32, i32) -> Probed<i32>,
    max_depth: i32,
    pub pieces: Vec<Part>,
}

impl<'a> Grower<'a> {
    pub fn new(data: &'a PoolCatalogue, library: &'a TemplateLibrary, random: &'a mut MersenneTwister, height: &'a dyn Fn(i32, i32) -> Probed<i32>, max_depth: i32) -> Self {
        Self {
            data,
            library,
            random,
            height,
            max_depth,
            pieces: Vec::new(),
        }
    }

    pub fn run(mut self, start: Part) -> Probed<Vec<Part>> {
        self.pieces.push(start);
        self.expand(0)?;
        Ok(self.pieces)
    }

    fn expand(&mut self, index: usize) -> Probed<()> {
        let piece = self.pieces[index].clone();
        if piece.depth > self.max_depth {
            return Ok(());
        }
        let data = self.data;
        let mut jigsaws = data.entries[piece.entry].jigsaws(self.library, piece.origin, piece.rotation);
        shuffle(self.random, &mut jigsaws);
        let mut inside: Vec<BoundingBox> = Vec::new();
        for layout in &jigsaws {
            let name = layout.pool.strip_prefix(NAMESPACE).unwrap_or(layout.pool);
            let Some(pool) = data.pool(name).filter(|pool| !pool.list.is_empty()) else { continue };
            let fallback = pool.fallback.as_deref().and_then(|name| data.pool(name)).filter(|fallback| !fallback.list.is_empty());
            if piece.depth == self.max_depth {
                if let Some(fallback) = fallback {
                    self.try_pool(index, &piece, layout, &fallback.list, &mut inside)?;
                }
            } else if !self.try_pool(index, &piece, layout, &pool.list, &mut inside)?
                && let Some(fallback) = fallback
            {
                self.try_pool(index, &piece, layout, &fallback.list, &mut inside)?;
            }
        }
        Ok(())
    }

    fn try_pool(&mut self, _index: usize, source: &Part, layout: &Connector, list: &[usize], inside: &mut Vec<BoundingBox>) -> Probed<bool> {
        let mut order = list.to_vec();
        shuffle(self.random, &mut order);
        let offset = layout.front.offset();
        let target = (layout.pos.0 + offset.0, layout.pos.1 + offset.1, layout.pos.2 + offset.2);
        let is_inside = source.bounds.contains(target.0, target.1, target.2);
        let data = self.data;
        for element_index in order {
            let element = &data.entries[element_index];
            if !element.is_valid() {
                return Ok(true);
            }
            let mut rotations = CANDIDATE_ROTATIONS;
            shuffle(self.random, &mut rotations);
            for rotation in rotations {
                let mut candidates = element.jigsaws(self.library, (0, 0, 0), rotation);
                shuffle(self.random, &mut candidates);
                for candidate in &candidates {
                    if !layout.can_attach(candidate, false) {
                        continue;
                    }
                    let mut origin = (target.0 - candidate.pos.0, target.1 - candidate.pos.1, target.2 - candidate.pos.2);
                    let mut bounds = element.bounds(origin, rotation);
                    let bottom = self.child_bottom(source, layout, element, candidate)?;
                    let delta = bottom - bounds.y0;
                    bounds = bounds.shifted_y(delta);
                    origin.1 += delta;
                    if !self.fits(&bounds, &source.bounds, is_inside, inside) {
                        continue;
                    }
                    let mut piece = Part::new(data, element_index, origin, rotation, source.depth + 1);
                    piece.ground = candidate.pos.1;
                    let child = self.pieces.len();
                    self.pieces.push(piece);
                    self.expand(child)?;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn child_bottom(&self, source: &Part, layout: &Connector, element: &PoolEntry, candidate: &Connector) -> Probed<i32> {
        let source_rigid = self.data.entries[source.entry].grounding() == Grounding::Fixed;
        let step = layout.front.offset().1;
        if source_rigid && element.grounding() == Grounding::Fixed {
            Ok(layout.pos.1 - candidate.pos.1 + step)
        } else {
            let surface = (self.height)(layout.pos.0, layout.pos.2)?;
            Ok(layout.pos.1 - (source.bounds.y0 + candidate.pos.1) + surface - 1 + step)
        }
    }

    fn fits(&self, bounds: &BoundingBox, source: &BoundingBox, is_inside: bool, inside: &mut Vec<BoundingBox>) -> bool {
        let overlaps = |other: &BoundingBox| bounds.x1 >= other.x0 && bounds.x0 <= other.x1 && bounds.z1 >= other.z0 && bounds.z0 <= other.z1;
        if is_inside {
            let within = source.x0 <= bounds.x0 && bounds.x1 <= source.x1 && source.z0 <= bounds.z0 && bounds.z1 <= source.z1;
            if !within || inside.iter().any(overlaps) {
                return false;
            }
            inside.push(*bounds);
            true
        } else {
            !self.pieces.iter().any(|piece| overlaps(&piece.bounds))
        }
    }
}
