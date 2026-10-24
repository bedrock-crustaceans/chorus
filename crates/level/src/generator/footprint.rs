use rustc_hash::{FxHashMap, FxHashSet};

use crate::generator::dimension::Generator;
use crate::generator::phase::{PhaseDescriptor, PhaseId};

pub type Footprints = FxHashMap<(PhaseId, u32), Vec<(i32, i32)>>;

pub fn footprints<G: Generator>() -> Footprints {
    let terminal = PhaseDescriptor::of::<G::Terminal>();
    let mut descriptors: FxHashMap<PhaseId, PhaseDescriptor<G>> = FxHashMap::default();
    let mut stack = vec![terminal];
    while let Some(descriptor) = stack.pop() {
        if descriptors.insert(descriptor.phase, descriptor).is_some() {
            continue;
        }
        stack.extend((descriptor.requires)().iter().map(|requirement| requirement.descriptor));
    }

    let mut offsets_from_terminal: FxHashMap<(PhaseId, u32), FxHashSet<(i32, i32)>> = FxHashMap::default();
    offsets_from_terminal.entry((terminal.phase, 0)).or_default().insert((0, 0));
    let mut work = vec![(terminal.phase, 0u32)];
    while let Some((phase, hop)) = work.pop() {
        let reached: Vec<(i32, i32)> = offsets_from_terminal[&(phase, hop)].iter().copied().collect();
        for requirement in (descriptors[&phase].requires)() {
            let requires_itself = requirement.descriptor.phase == phase;
            let dependency_hop = match (requires_itself, requirement.max_hops) {
                (false, None) => 0,
                (true, Some(max_hops)) if hop < max_hops => hop + 1,
                _ => continue,
            };
            let target = offsets_from_terminal.entry((requirement.descriptor.phase, dependency_hop)).or_default();
            let before = target.len();
            for &(sx, sz) in &reached {
                for &(dx, dz) in requirement.offsets {
                    target.insert((sx + dx, sz + dz));
                }
            }
            if target.len() > before {
                work.push((requirement.descriptor.phase, dependency_hop));
            }
        }
    }

    offsets_from_terminal
        .into_iter()
        .map(|(key, reached)| {
            let mut cells: Vec<(i32, i32)> = reached.into_iter().map(|(x, z)| (-x, -z)).collect();
            cells.sort_unstable();
            (key, cells)
        })
        .collect()
}

pub fn extent(footprints: &Footprints) -> f64 {
    footprints.values().flatten().map(|&(x, z)| ((x * x + z * z) as f64).sqrt()).fold(0.0, f64::max)
}

pub struct Region {
    half: i32,
    side: usize,
    bits: Box<[bool]>,
}

impl Region {
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
        let region = Region::new(&[(0, 0)], 2);
        assert!(region.contains(2, 0) && region.contains(-2, 0) && region.contains(1, 1));
        assert!(!region.contains(2, 1), "5 is more than 4");
        assert!(!region.contains(3, 0));
    }

    #[test]
    fn a_region_unites_the_disc_around_every_footprint_cell() {
        let region = Region::new(&[(-3, 0), (3, 0)], 1);
        assert!(region.contains(-4, 0) && region.contains(-3, 1) && region.contains(4, 0) && region.contains(3, -1));
        assert!(!region.contains(0, 0), "between the two cells nothing is reached");
    }

    #[test]
    fn an_empty_footprint_reaches_nowhere() {
        let region = Region::new(&[], 3);
        assert!(!region.contains(0, 0));
    }
}
