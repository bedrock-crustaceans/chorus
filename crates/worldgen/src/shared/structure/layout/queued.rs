use std::cmp::Reverse;
use std::collections::BinaryHeap;

use super::catalogue::PoolCatalogue;
use super::entry::{Connector, Grounding};
use super::grow::{Part, middle};
use super::junction::Joint;
use super::legacy_seeds::{forked_at, shuffle};
use crate::shared::rng::legacy::Legacy;
use crate::shared::rng::random::RandomSource;
use crate::shared::structure::place::Rotation;
use crate::shared::structure::template::TemplateLibrary;
use crate::shared::structure::{BoundingBox, Probed};
use chorus_level::dimension_definition::DimensionDefinition;

const NAMESPACE: &str = "minecraft:";
const ROTATION_COUNT: i32 = 4;

pub enum StartHeight {
    Uniform(i32, i32),
    Constant(i32),
}

pub struct StartSpec<'a> {
    pub pool: &'a str,
    pub max_depth: i32,
    pub horizontal: i32,
    pub vertical: i32,
    pub height: StartHeight,
    pub project: bool,
    pub padding: i32,
    pub aliases: Option<&'a str>,
}

#[derive(Default)]
struct Room {
    allowed: Vec<BoundingBox>,
    taken: Vec<BoundingBox>,
}

impl Room {
    fn admits(&self, bounds: &BoundingBox) -> bool {
        let inside =
            |allowed: &BoundingBox| allowed.x0 <= bounds.x0 && bounds.x1 <= allowed.x1 && allowed.y0 <= bounds.y0 && bounds.y1 <= allowed.y1 && allowed.z0 <= bounds.z0 && bounds.z1 <= allowed.z1;
        self.allowed.iter().any(inside) && !self.taken.iter().any(|taken| bounds.intersects(taken))
    }

    fn take(&mut self, bounds: &BoundingBox) {
        let inverted = bounds.x1 + 1 == bounds.x0 && bounds.y1 + 1 == bounds.y0 && bounds.z1 + 1 == bounds.z0;
        self.taken.push(if inverted {
            BoundingBox::new(bounds.x0, bounds.y0, bounds.z0, bounds.x0 + 1, bounds.y0 + 1, bounds.z0 + 1)
        } else {
            *bounds
        });
    }
}

pub struct Assembly<'a> {
    catalogue: &'a PoolCatalogue,
    library: &'a TemplateLibrary,
    random: &'a mut Legacy,
    height: &'a dyn Fn(i32, i32) -> Probed<i32>,
    aliases: Vec<(&'a str, &'a str)>,
    max_depth: i32,
    pieces: Vec<Part>,
    rooms: Vec<Room>,
    queue: BinaryHeap<(i32, Reverse<usize>, usize, usize)>,
    queued: usize,
}

impl StartSpec<'_> {
    pub fn start_part(&self, catalogue: &PoolCatalogue, random: &mut Legacy, chunk: (i32, i32), height: &dyn Fn(i32, i32) -> Probed<i32>) -> Probed<(Part, (i32, i32, i32), i32)> {
        let start_y = match self.height {
            StartHeight::Uniform(low, high) => random.next_int_bounded(high - low + 1) + low,
            StartHeight::Constant(level) => level,
        };
        let rotation = Rotation::from_index(random.next_int_bounded(ROTATION_COUNT));
        let pool = catalogue.pool(self.pool).expect("start pool");
        let entry = pool.list[random.next_int_bounded(pool.list.len() as i32) as usize];
        let mut part = Part::new(catalogue, entry, (chunk.0 * 16, start_y, chunk.1 * 16), rotation, 0);
        let bounds = part.bounds;
        let (x, z) = (middle(bounds.x0, bounds.x1), middle(bounds.z0, bounds.z1));
        let y = if self.project { start_y + height(x, z)? } else { start_y };
        part.shift_y(y - 1 - bounds.y0);
        Ok((part, (x, y, z), start_y))
    }

    pub fn assemble(
        &self,
        catalogue: &PoolCatalogue,
        library: &TemplateLibrary,
        random: &mut Legacy,
        level_seed: i64,
        chunk: (i32, i32),
        definition: DimensionDefinition,
        height: &dyn Fn(i32, i32) -> Probed<i32>,
    ) -> Probed<Vec<Part>> {
        let (part, (x, y, z), start_y) = self.start_part(catalogue, random, chunk, height)?;
        let limit = BoundingBox::new(
            x - self.horizontal,
            (y - self.vertical).max(definition.min_y + self.padding),
            z - self.horizontal,
            x + self.horizontal,
            (y + self.vertical).min(definition.max_y() - self.padding),
            z + self.horizontal,
        );
        let aliases = self.resolve_aliases(catalogue, level_seed, (chunk.0 * 16, start_y, chunk.1 * 16));
        Assembly::new(catalogue, library, random, height, aliases, self.max_depth).run(part, limit)
    }

    fn resolve_aliases<'c>(&self, catalogue: &'c PoolCatalogue, level_seed: i64, at: (i32, i32, i32)) -> Vec<(&'c str, &'c str)> {
        let Some(bindings) = self.aliases.and_then(|set| catalogue.bindings.get(set)) else {
            return Vec::new();
        };
        let mut random = forked_at(level_seed, at);
        let mut resolved = Vec::new();
        for choices in bindings {
            let pick = if choices.len() > 1 { random.next_int_bounded(choices.len() as i32) as usize } else { 0 };
            resolved.extend(choices[pick].iter().map(|(from, to)| (from.as_str(), to.as_str())));
        }
        resolved
    }
}

impl<'a> Assembly<'a> {
    pub fn new(
        catalogue: &'a PoolCatalogue,
        library: &'a TemplateLibrary,
        random: &'a mut Legacy,
        height: &'a dyn Fn(i32, i32) -> Probed<i32>,
        aliases: Vec<(&'a str, &'a str)>,
        max_depth: i32,
    ) -> Self {
        Self {
            catalogue,
            library,
            random,
            height,
            aliases,
            max_depth,
            pieces: Vec::new(),
            rooms: Vec::new(),
            queue: BinaryHeap::new(),
            queued: 0,
        }
    }

    pub fn run(mut self, start: Part, limit: BoundingBox) -> Probed<Vec<Part>> {
        self.rooms.push(Room {
            allowed: vec![limit],
            taken: vec![start.bounds],
        });
        self.pieces.push(start);
        self.enqueue(0, 0, 0);
        while let Some((_, _, piece, room)) = self.queue.pop() {
            self.expand(piece, room)?;
        }
        Ok(self.pieces)
    }

    fn enqueue(&mut self, priority: i32, piece: usize, room: usize) {
        self.queued += 1;
        self.queue.push((priority, Reverse(self.queued), piece, room));
    }

    fn rigid(&self, entry: usize) -> bool {
        self.catalogue.entries[entry].grounding() == Grounding::Fixed
    }

    fn expand(&mut self, index: usize, room: usize) -> Probed<()> {
        let piece = self.pieces[index].clone();
        if piece.depth > self.max_depth {
            return Ok(());
        }
        let catalogue = self.catalogue;
        let mut connectors = catalogue.entries[piece.entry].jigsaws(self.library, piece.origin, piece.rotation);
        shuffle(self.random, &mut connectors);
        connectors.sort_by_key(|connector| Reverse(connector.selection_priority));
        let mut own_room = None;
        for connector in &connectors {
            let wanted = connector.pool.strip_prefix(NAMESPACE).unwrap_or(connector.pool);
            let name = self.aliases.iter().find(|(from, _)| *from == wanted).map_or(wanted, |(_, to)| *to);
            let Some(pool) = catalogue.pool(name).filter(|pool| !pool.list.is_empty()) else { continue };
            let fallback = pool.fallback.as_deref().and_then(|name| catalogue.pool(name)).filter(|fallback| !fallback.list.is_empty());
            let mut candidates = Vec::new();
            for source in [(piece.depth != self.max_depth).then_some(pool), fallback].into_iter().flatten() {
                let mut shuffled = source.list.clone();
                shuffle(self.random, &mut shuffled);
                candidates.extend(shuffled);
            }
            self.attach(index, &piece, connector, &candidates, room, &mut own_room)?;
        }
        Ok(())
    }

    fn attach(&mut self, index: usize, source: &Part, jigsaw: &Connector, candidates: &[usize], room: usize, own_room: &mut Option<usize>) -> Probed<()> {
        let offset = jigsaw.front.offset();
        let target = (jigsaw.pos.0 + offset.0, jigsaw.pos.1 + offset.1, jigsaw.pos.2 + offset.2);
        let mut room = room;
        if source.bounds.contains(target.0, target.1, target.2) {
            room = *own_room.get_or_insert_with(|| {
                self.rooms.push(Room {
                    allowed: vec![source.bounds],
                    taken: Vec::new(),
                });
                self.rooms.len() - 1
            });
        }
        let catalogue = self.catalogue;
        for &entry_index in candidates {
            let entry = &catalogue.entries[entry_index];
            if !entry.is_valid() {
                return Ok(());
            }
            let mut rotations = Rotation::ALL;
            shuffle(self.random, &mut rotations);
            for rotation in rotations {
                let mut options = entry.jigsaws(self.library, (0, 0, 0), rotation);
                shuffle(self.random, &mut options);
                options.sort_by_key(|option| Reverse(option.selection_priority));
                for candidate in &options {
                    if !jigsaw.can_attach(candidate, entry.is_feature()) {
                        continue;
                    }
                    let mut origin = (target.0 - candidate.pos.0, target.1 - candidate.pos.1, target.2 - candidate.pos.2);
                    let (source_rigid, child_rigid) = (self.rigid(source.entry), self.rigid(entry_index));
                    let mut bounds = entry.bounds(origin, rotation);
                    let bottom = if source_rigid && child_rigid {
                        jigsaw.pos.1 - candidate.pos.1 + offset.1
                    } else {
                        (self.height)(jigsaw.pos.0, jigsaw.pos.2)? - candidate.pos.1
                    };
                    let shift = bottom - bounds.y0;
                    bounds = bounds.shifted_y(shift);
                    origin.1 += shift;
                    if !self.rooms[room].admits(&bounds) {
                        continue;
                    }
                    self.rooms[room].take(&bounds);
                    let mut child = Part::new(catalogue, entry_index, origin, rotation, source.depth + 1);
                    let height = self.height;
                    let joint = Joint {
                        source,
                        source_rigid,
                        jigsaw,
                        candidate,
                        child_rigid,
                    };
                    let junction = joint.link(&mut child, || height(jigsaw.pos.0, jigsaw.pos.2))?;
                    self.pieces[index].junctions.push(junction);
                    let at = self.pieces.len();
                    self.pieces.push(child);
                    self.enqueue(jigsaw.placement_priority, at, room);
                    return Ok(());
                }
            }
        }
        Ok(())
    }
}
