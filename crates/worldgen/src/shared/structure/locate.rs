use super::placement;
use super::{Probed, StartProbe, StructureKind, Structures};

const STRONGHOLD_CELL_MARGIN: i32 = placement::STRONGHOLD_SPREAD_OFFSET + 1;

impl Structures {
    pub fn locate(&self, probe: &dyn StartProbe, kind: StructureKind, from: (i32, i32)) -> Probed<Option<(i32, i32)>> {
        let Some(grid) = self.registry().any_grid(kind) else { return Ok(None) };
        let spacing = grid.spacing;
        let distance = |(x, z): (i32, i32)| (x as i64 - from.0 as i64).pow(2) + (z as i64 - from.1 as i64).pow(2);
        let region = (from.0.div_euclid(spacing), from.1.div_euclid(spacing));
        let mut best: Option<(i32, i32)> = None;
        let mut ring: i32 = 0;
        loop {
            for dx in -ring..=ring {
                for dz in -ring..=ring {
                    if dx.abs() != ring && dz.abs() != ring {
                        continue;
                    }
                    let anchor = ((region.0 + dx) * spacing, (region.1 + dz) * spacing);
                    let Some(candidate) = self.candidate(kind, anchor) else { continue };
                    if !self.is_start(probe, kind, candidate)? {
                        continue;
                    }
                    if best.is_none_or(|current| distance(candidate) < distance(current)) {
                        best = Some(candidate);
                    }
                }
            }
            let unscanned = ring as i64 * spacing as i64 + 1;
            if let Some(found) = best
                && distance(found) <= unscanned * unscanned
            {
                return Ok(Some(found));
            }
            ring += 1;
        }
    }

    pub fn locate_stronghold(&self, probe: &dyn StartProbe, from: (i32, i32)) -> Probed<Option<(i32, i32)>> {
        let squared = |(x, z): (i32, i32)| (x as i64 - from.0 as i64).pow(2) + (z as i64 - from.1 as i64).pow(2);
        let mut best: Option<(i32, i32)> = None;
        let consider = |start: (i32, i32), best: &mut Option<(i32, i32)>| {
            if best.is_none_or(|current| squared(start) < squared(current)) {
                *best = Some(start);
            }
        };
        for &start in self.stronghold_spiral(probe)? {
            consider(start, &mut best);
        }
        let cell = (placement::stronghold_cell(from.0), placement::stronghold_cell(from.1));
        let size = placement::STRONGHOLD_CELL_SIZE;
        let centre_offset = size / 2;
        let mut ring: i32 = 0;
        loop {
            for dx in -ring..=ring {
                for dz in -ring..=ring {
                    if dx.abs() != ring && dz.abs() != ring {
                        continue;
                    }
                    let centre = ((cell.0 + dx) * size + centre_offset, (cell.1 + dz) * size + centre_offset);
                    if let Some(start) = placement::stronghold_grid_start(self.level_seed, centre) {
                        consider(start, &mut best);
                    }
                }
            }
            let unscanned = ring as i64 * size as i64 + STRONGHOLD_CELL_MARGIN as i64;
            if let Some(found) = best
                && squared(found) <= unscanned * unscanned
            {
                return Ok(Some(found));
            }
            ring += 1;
        }
    }
}
