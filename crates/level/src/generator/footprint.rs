use rustc_hash::{FxHashMap, FxHashSet};

use crate::generator::dimension::Generator;
use crate::generator::phase::{PhaseHandle, PhaseId};
use glam::IVec2;

pub type Footprints = FxHashMap<(PhaseId, u32), Vec<(i32, i32)>>;

const ORIGINS: [(i32, i32); 4] = [(0, 0), (1, 0), (0, 1), (1, 1)];

pub fn footprints<G: Generator>() -> Footprints {
    let terminal = PhaseHandle::of::<G::Terminal>();
    let mut handles: FxHashMap<PhaseId, PhaseHandle<G>> = FxHashMap::default();
    let mut stack = vec![terminal];
    while let Some(handle) = stack.pop() {
        if handles.insert(handle.phase, handle).is_some() {
            continue;
        }
        stack.extend((handle.dependencies)().iter().map(|dependency| dependency.handle));
    }

    let mut union: FxHashMap<(PhaseId, u32), FxHashSet<(i32, i32)>> = FxHashMap::default();
    for origin in ORIGINS {
        for (key, reached) in reach(&handles, terminal.phase, origin) {
            union.entry(key).or_default().extend(reached);
        }
    }

    union
        .into_iter()
        .map(|(key, reached)| {
            let mut cells: Vec<(i32, i32)> = reached.into_iter().map(|(x, z)| (-x, -z)).collect();
            cells.sort_unstable();
            (key, cells)
        })
        .collect()
}

fn reach<G: Generator>(handles: &FxHashMap<PhaseId, PhaseHandle<G>>, terminal: PhaseId, origin: (i32, i32)) -> FxHashMap<(PhaseId, u32), FxHashSet<(i32, i32)>> {
    let mut reached_from_terminal: FxHashMap<(PhaseId, u32), FxHashSet<(i32, i32)>> = FxHashMap::default();
    reached_from_terminal.entry((terminal, 0)).or_default().insert((0, 0));
    let mut work = vec![(terminal, 0u32)];
    while let Some((phase, hop)) = work.pop() {
        let reached: Vec<(i32, i32)> = reached_from_terminal[&(phase, hop)].iter().copied().collect();
        for dependency in (handles[&phase].dependencies)() {
            let depends_on_itself = dependency.handle.phase == phase;
            let dependency_hop = match (depends_on_itself, dependency.max_hops) {
                (false, None) => 0,
                (true, Some(max_hops)) if hop < max_hops => hop + 1,
                _ => continue,
            };
            let target = reached_from_terminal.entry((dependency.handle.phase, dependency_hop)).or_default();
            let before = target.len();
            for &(sx, sz) in &reached {
                let consumer = IVec2::new(origin.0 + sx, origin.1 + sz);
                if !dependency.position_gate && dependency.if_dependent.is_some_and(|applies| !applies(consumer)) {
                    continue;
                }
                for &(dx, dz) in dependency.offsets {
                    let dependency_cell = consumer + IVec2::new(dx, dz);
                    if !dependency.position_gate && dependency.if_dependency.is_some_and(|applies| !applies(dependency_cell)) {
                        continue;
                    }
                    target.insert((sx + dx, sz + dz));
                }
            }
            if target.len() > before {
                work.push((dependency.handle.phase, dependency_hop));
            }
        }
    }
    reached_from_terminal
}

pub fn extent(footprints: &Footprints) -> f64 {
    footprints.values().flatten().map(|&(x, z)| ((x * x + z * z) as f64).sqrt()).fold(0.0, f64::max)
}

pub struct CellSet {
    half: i32,
    side: usize,
    bits: Box<[bool]>,
}

impl CellSet {
    pub fn new(footprint: &[(i32, i32)], radius: i32) -> Self {
        let reach = footprint.iter().map(|&(x, z)| x.abs().max(z.abs())).max().unwrap_or(0);
        let half = reach + radius;
        let side = (2 * half + 1) as usize;
        let mut bits = vec![false; side * side].into_boxed_slice();
        for &(tx, tz) in footprint {
            for ux in -radius..=radius {
                for uz in -radius..=radius {
                    if ux * ux + uz * uz <= radius * radius {
                        bits[((tx + ux + half) as usize) * side + (tz + uz + half) as usize] = true;
                    }
                }
            }
        }
        Self { half, side, bits }
    }

    pub fn half(&self) -> i32 {
        self.half
    }

    pub fn contains(&self, dx: i32, dz: i32) -> bool {
        dx.abs() <= self.half && dz.abs() <= self.half && self.bits[((dx + self.half) as usize) * self.side + (dz + self.half) as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_region_is_the_footprint_grown_by_the_demand_disc() {
        let region = CellSet::new(&[(0, 0)], 2);
        assert!(region.contains(2, 0) && region.contains(-2, 0) && region.contains(1, 1));
        assert!(!region.contains(2, 1), "5 is more than 4");
        assert!(!region.contains(3, 0));
    }

    #[test]
    fn a_region_unites_the_disc_around_every_footprint_cell() {
        let region = CellSet::new(&[(-3, 0), (3, 0)], 1);
        assert!(region.contains(-4, 0) && region.contains(-3, 1) && region.contains(4, 0) && region.contains(3, -1));
        assert!(!region.contains(0, 0), "between the two cells nothing is reached");
    }

    #[test]
    fn an_empty_footprint_reaches_nowhere() {
        let region = CellSet::new(&[], 3);
        assert!(!region.contains(0, 0));
    }
}
