use crate::shared::biome::Biome;
use crate::shared::structure::data::climate::BiomeClimate;

const CLIMATE_KEY: &str = "climate/biome_climate.json";
const DIMENSIONS: usize = 6;
const INNER_NODE: i64 = -1;

pub type Point = [i64; DIMENSIONS];

#[derive(Debug)]
pub enum BiomeTreeError {
    Malformed(String),
    UnknownBiome(String),
}

impl std::fmt::Display for BiomeTreeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed(reason) => write!(formatter, "malformed biome climate: {reason}"),
            Self::UnknownBiome(name) => write!(formatter, "the biome climate names an unknown biome: {name}"),
        }
    }
}

impl std::error::Error for BiomeTreeError {}

pub struct Entry {
    pub biome: Biome,
    pub id: i32,
    pub low: Point,
    pub high: Point,
}

struct Node {
    low: Point,
    high: Point,
    entry: Option<u32>,
    children_start: u32,
    children_len: u32,
}

#[derive(Clone, Copy)]
pub struct Nearest {
    node: u32,
    distance: i64,
}

pub struct BiomeTree {
    entries: Vec<Entry>,
    nodes: Vec<Node>,
    children: Vec<u32>,
}

fn distance(low: &Point, high: &Point, target: &Point) -> i64 {
    let mut sum = 0;
    for dimension in 0..DIMENSIONS {
        let value = target[dimension];
        let gap = if value >= high[dimension] {
            value - high[dimension]
        } else if value <= low[dimension] {
            low[dimension] - value
        } else {
            0
        };
        sum += gap * gap;
    }
    sum
}

fn parse_entries(climate: &BiomeClimate) -> Result<Vec<Entry>, BiomeTreeError> {
    let quantised = |value: f64| (value * 10000.0).round() as i64;
    let mut entries = Vec::new();
    for (name, points) in &climate.0 {
        let biome = Biome::from_name(name).ok_or_else(|| BiomeTreeError::UnknownBiome(name.clone()))?;
        let id = biome.bedrock_id();
        entries.extend(points.iter().map(|point| Entry {
            biome: biome.clone(),
            id,
            low: std::array::from_fn(|dimension| quantised(point[dimension].0)),
            high: std::array::from_fn(|dimension| quantised(point[dimension].1)),
        }));
    }
    Ok(entries)
}

const BRANCHING: usize = 6;

struct Space {
    low: Point,
    high: Point,
}

struct Arena {
    spaces: Vec<Space>,
    entries: Vec<Option<u32>>,
    children: Vec<Vec<usize>>,
}

fn merged(spaces: &[&Space]) -> Space {
    let mut low = [i64::MAX; DIMENSIONS];
    let mut high = [i64::MIN; DIMENSIONS];
    for space in spaces {
        for dimension in 0..DIMENSIONS {
            low[dimension] = low[dimension].min(space.low[dimension]);
            high[dimension] = high[dimension].max(space.high[dimension]);
        }
    }
    Space { low, high }
}

fn center(space: &Space, dimension: usize, absolute: bool) -> i64 {
    let value = (space.low[dimension] + space.high[dimension]) / 2;
    if absolute { value.abs() } else { value }
}

fn order(a: &Space, b: &Space, dimension: usize, absolute: bool) -> std::cmp::Ordering {
    (0..DIMENSIONS)
        .map(|step| (dimension + step) % DIMENSIONS)
        .map(|axis| center(a, axis, absolute).cmp(&center(b, axis, absolute)))
        .find(|ordering| ordering.is_ne())
        .unwrap_or(std::cmp::Ordering::Equal)
}

fn volume(space: &Space) -> i64 {
    (0..DIMENSIONS).map(|dimension| (space.high[dimension] - space.low[dimension]).abs()).sum()
}

impl Arena {
    fn inner(&mut self, children: Vec<usize>) -> usize {
        let space = merged(&children.iter().map(|&child| &self.spaces[child]).collect::<Vec<_>>());
        self.spaces.push(space);
        self.entries.push(None);
        self.children.push(children);
        self.spaces.len() - 1
    }

    fn buckets(&self, ids: &[usize]) -> Vec<Vec<usize>> {
        let per_bucket = (BRANCHING as f64).powf(((ids.len() as f64 - 0.01).ln() / (BRANCHING as f64).ln()).floor()) as usize;
        let mut buckets = Vec::new();
        let mut current = Vec::new();
        for &id in ids {
            current.push(id);
            if current.len() >= per_bucket {
                buckets.push(std::mem::take(&mut current));
            }
        }
        if !current.is_empty() {
            buckets.push(current);
        }
        buckets
    }

    fn bucket_space(&self, bucket: &[usize]) -> Space {
        merged(&bucket.iter().map(|&id| &self.spaces[id]).collect::<Vec<_>>())
    }

    fn build(&mut self, mut ids: Vec<usize>) -> usize {
        if ids.len() == 1 {
            return ids[0];
        }
        if ids.len() <= BRANCHING {
            return self.inner(ids);
        }
        let mut best: Option<(i64, Vec<Vec<usize>>, usize)> = None;
        for dimension in 0..DIMENSIONS {
            ids.sort_by(|&a, &b| order(&self.spaces[a], &self.spaces[b], dimension, false));
            let buckets = self.buckets(&ids);
            let cost: i64 = buckets.iter().map(|bucket| volume(&self.bucket_space(bucket))).sum();
            if best.as_ref().is_none_or(|(top, _, _)| *top > cost) {
                best = Some((cost, buckets, dimension));
            }
        }
        let (_, mut buckets, dimension) = best.expect("a best split");
        let spaces: Vec<Space> = buckets.iter().map(|bucket| self.bucket_space(bucket)).collect();
        let mut ranked: Vec<usize> = (0..buckets.len()).collect();
        ranked.sort_by(|&a, &b| order(&spaces[a], &spaces[b], dimension, true));
        let children = ranked.into_iter().map(|index| self.build(std::mem::take(&mut buckets[index]))).collect();
        self.inner(children)
    }

    fn listing(&self, id: usize, depth: usize, out: &mut Vec<(usize, i64)>) {
        out.push((depth, self.entries[id].map_or(INNER_NODE, i64::from)));
        for &child in &self.children[id] {
            self.listing(child, depth + 1, out);
        }
    }
}

fn built_listing(entries: &[Entry]) -> Vec<(usize, i64)> {
    let mut arena = Arena {
        spaces: Vec::new(),
        entries: Vec::new(),
        children: Vec::new(),
    };
    for (index, entry) in entries.iter().enumerate() {
        arena.spaces.push(Space { low: entry.low, high: entry.high });
        arena.entries.push(Some(index as u32));
        arena.children.push(Vec::new());
    }
    let root = arena.build((0..entries.len()).collect());
    let mut out = Vec::new();
    arena.listing(root, 0, &mut out);
    out
}

impl BiomeTree {
    pub fn parse() -> Result<Self, BiomeTreeError> {
        let climate = BiomeClimate::parse(&crate::shared::resources::text(CLIMATE_KEY)).map_err(|error| BiomeTreeError::Malformed(error.to_string()))?;
        let entries = parse_entries(&climate)?;
        let listing = built_listing(&entries);
        Ok(Self::from_listing(entries, &listing))
    }

    fn from_listing(entries: Vec<Entry>, listing: &[(usize, i64)]) -> Self {
        let mut nodes: Vec<Node> = Vec::with_capacity(listing.len());
        let mut members: Vec<Vec<u32>> = Vec::with_capacity(listing.len());
        let mut path: Vec<u32> = Vec::new();
        for (index, item) in listing.iter().enumerate() {
            let (depth, entry) = *item;
            path.truncate(depth);
            if let Some(&parent) = path.last() {
                members[parent as usize].push(index as u32);
            }
            let (low, high, entry) = match entry {
                INNER_NODE => ([i64::MAX; DIMENSIONS], [i64::MIN; DIMENSIONS], None),
                id => (entries[id as usize].low, entries[id as usize].high, Some(id as u32)),
            };
            nodes.push(Node {
                low,
                high,
                entry,
                children_start: 0,
                children_len: 0,
            });
            members.push(Vec::new());
            path.push(index as u32);
        }
        for index in (0..nodes.len()).rev() {
            for &child in &members[index] {
                let (child_low, child_high) = (nodes[child as usize].low, nodes[child as usize].high);
                for dimension in 0..DIMENSIONS {
                    nodes[index].low[dimension] = nodes[index].low[dimension].min(child_low[dimension]);
                    nodes[index].high[dimension] = nodes[index].high[dimension].max(child_high[dimension]);
                }
            }
        }
        let mut children = Vec::with_capacity(nodes.len());
        for (node, list) in nodes.iter_mut().zip(&members) {
            node.children_start = children.len() as u32;
            node.children_len = list.len() as u32;
            children.extend_from_slice(list);
        }
        Self { entries, nodes, children }
    }

    fn leaf_entry(&self, nearest: Nearest) -> &Entry {
        &self.entries[self.nodes[nearest.node as usize].entry.expect("nearest is a leaf") as usize]
    }

    pub fn biome(&self, nearest: Nearest) -> Biome {
        self.leaf_entry(nearest).biome.clone()
    }

    pub fn biome_id(&self, nearest: Nearest) -> i32 {
        self.leaf_entry(nearest).id
    }

    pub fn find(&self, target: &Point, previous: Option<Nearest>) -> Nearest {
        let mut best = previous.map(|nearest| {
            let node = &self.nodes[nearest.node as usize];
            Nearest {
                node: nearest.node,
                distance: distance(&node.low, &node.high, target),
            }
        });
        self.descend(0, target, &mut best);
        best.expect("the tree has a leaf")
    }

    fn descend(&self, parent: u32, target: &Point, best: &mut Option<Nearest>) {
        let node = &self.nodes[parent as usize];
        let start = node.children_start as usize;
        for &child in &self.children[start..start + node.children_len as usize] {
            let candidate = &self.nodes[child as usize];
            let candidate_distance = distance(&candidate.low, &candidate.high, target);
            if best.is_some_and(|nearest| candidate_distance >= nearest.distance) {
                continue;
            }
            if candidate.entry.is_some() {
                *best = Some(Nearest {
                    node: child,
                    distance: candidate_distance,
                });
            } else {
                self.descend(child, target, best);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl BiomeTree {
        fn entries(&self) -> &[Entry] {
            &self.entries
        }
    }

    const RANDOM_TARGETS: usize = 4000;
    const TARGET_SPAN: i64 = 24000;

    fn brute_force(tree: &BiomeTree, target: &Point) -> i64 {
        tree.entries().iter().map(|entry| distance(&entry.low, &entry.high, target)).min().expect("entries")
    }

    fn lcg(state: &mut u64) -> u64 {
        *state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        *state >> 33
    }

    #[test]
    fn search_matches_brute_force_fitness() {
        let tree = BiomeTree::parse().expect("the biome tree parses");
        let mut state = 12345u64;
        let mut previous = None;
        for _ in 0..RANDOM_TARGETS {
            let target: Point = std::array::from_fn(|_| (lcg(&mut state) % TARGET_SPAN as u64) as i64 - TARGET_SPAN / 2);
            let cold = tree.find(&target, None);
            let warm = tree.find(&target, previous);
            let expected_distance = brute_force(&tree, &target);
            assert_eq!(cold.distance, expected_distance);
            assert_eq!(warm.distance, expected_distance);
            previous = Some(warm);
        }
    }
}
