use rustc_hash::FxHashMap;

use crate::generator::dimension::Generator;
use crate::generator::phase::{PhaseHandle, PhaseId};

pub(super) fn offsets_step(offsets: &[(i32, i32)]) -> f64 {
    offsets.iter().map(|&(dx, dz)| ((dx * dx + dz * dz) as f64).sqrt()).fold(0.0, f64::max)
}

pub(super) fn phase_shape<G: Generator>() -> (FxHashMap<PhaseId, f64>, FxHashMap<PhaseId, u32>) {
    let mut reach = FxHashMap::default();
    let mut hop_limits = FxHashMap::default();
    let mut stack = vec![(PhaseHandle::of::<G::Terminal>(), 0.0)];
    while let Some((handle, distance)) = stack.pop() {
        let dependencies = (handle.dependencies)();
        let hops = dependencies
            .iter()
            .filter(|dependency| dependency.handle.phase == handle.phase)
            .map(|dependency| offsets_step(dependency.offsets) * dependency.max_hops.unwrap_or(0) as f64)
            .fold(0.0, f64::max);
        let hop_limit = dependencies
            .iter()
            .filter(|dependency| dependency.handle.phase == handle.phase)
            .map(|dependency| dependency.max_hops.unwrap_or(0))
            .max()
            .unwrap_or(0);
        hop_limits.insert(handle.phase, hop_limit);
        let distance = distance + hops;
        if reach.get(&handle.phase).is_some_and(|&known| known >= distance) {
            continue;
        }
        reach.insert(handle.phase, distance);
        for dependency in dependencies.iter().filter(|dependency| dependency.handle.phase != handle.phase) {
            stack.push((dependency.handle, distance + offsets_step(dependency.offsets)));
        }
    }
    (reach, hop_limits)
}
