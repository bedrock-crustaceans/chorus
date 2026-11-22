use super::site::put;
use super::spec::FancyTrunk;
use super::{Anchor, Tree};
use crate::bedrock::overworld::decoration::ore::Draw;
use crate::bedrock::overworld::decoration::sine;
use crate::shared::chunk::region::Region;
use glam::IVec3;

const ROUNDING: f32 = 0.5;
const CLUSTER_BASE: f32 = 1.382;
const CLUSTER_NEAR_OFFSET: f32 = 0.328;
const HALF: f32 = 0.5;

fn line(from: IVec3, delta: IVec3) -> impl Iterator<Item = (IVec3, IVec3)> {
    let span = delta.abs().max_element();
    let step = [delta.x, delta.y, delta.z].map(|axis| axis as f32 / span as f32);
    let last = if span == 0 { -1 } else { span };
    (0..=last).map(move |index| {
        let offset = IVec3::from_array(step.map(|axis| (index as f32 * axis + ROUNDING) as i32));
        (from + offset, offset)
    })
}

fn clear(tree: &Tree, region: &Region, from: IVec3, delta: IVec3) -> bool {
    tree.may_grow_through.is_empty() || line(from, delta).all(|(pos, _)| tree.grows_through(region, super::site::block(region, pos)))
}

impl FancyTrunk {
    pub fn place<D: Draw>(&self, tree: &Tree, region: &mut Region, random: &mut D, origin: IVec3) -> Option<IVec3> {
        let mut height = self.height_base + random.below(self.height_variance);
        let origin = tree.roots_then(region, random, origin, height)?;
        if !tree.may_grow_on.allows(region.blocks, super::site::block(region, origin - IVec3::Y)) {
            return None;
        }
        if !tree.may_grow_through.is_empty()
            && let Some(at) = line(origin, IVec3::new(0, height - 1, 0)).position(|(pos, _)| !tree.grows_through(region, super::site::block(region, pos)))
        {
            if at as i32 <= self.height_base {
                return None;
            }
            height = at as i32;
        }
        let scaled = height as f32;
        let stem = (self.height_scale * scaled) as i32;
        let stem_top = origin.y + if stem < height { stem } else { height - 1 };
        let span = self.height_variance + 1;
        let ratio = self.branch_density * scaled / span as f32;
        let clusters = ((ratio * ratio + CLUSTER_BASE) as i32).max(1);
        let rise = span / 3;
        let mut coords = vec![(IVec3::new(origin.x, origin.y + height - rise, origin.z), stem_top)];
        let half = scaled * HALF;
        for level in (0..=height - rise).rev() {
            if self.foliage_altitude_factor * scaled > level as f32 {
                continue;
            }
            let distance = half - level as f32;
            let profile = if distance == 0.0 {
                half
            } else if distance.abs() < half {
                (half * half - distance * distance).sqrt()
            } else {
                0.0
            };
            let reach = self.width_scale * (profile * HALF);
            for _ in 0..clusters {
                let near = random.float();
                let turn = random.float();
                let radius = (near + CLUSTER_NEAR_OFFSET) * reach;
                let angle = (turn + turn) * std::f32::consts::PI;
                let cluster = IVec3::new(
                    origin.x + (radius * sine::sin(angle) + ROUNDING) as i32,
                    origin.y + level - 1,
                    origin.z + (radius * sine::cos(angle) + ROUNDING) as i32,
                );
                if !clear(tree, region, cluster, IVec3::new(0, rise, 0)) {
                    continue;
                }
                let (hx, hz) = (origin.x - cluster.x, origin.z - cluster.z);
                let horizontal = ((hz * hz + hx * hx) as f64).sqrt() as i32 as f32;
                let dropped = cluster.y as f32 - horizontal * self.branch_slope;
                let attach = if dropped > stem_top as f32 { stem_top } else { dropped as i32 };
                let from = IVec3::new(origin.x, attach, origin.z);
                if !clear(tree, region, from, cluster - from) {
                    continue;
                }
                coords.push((cluster, attach));
            }
        }
        let anchors: Vec<Anchor> = coords.iter().map(|&(at, _)| Anchor::at(at)).collect();
        tree.place_canopies(region, random, &anchors);
        for dx in 0..self.width {
            for dz in 0..self.width {
                self.logs(region, origin + IVec3::new(dx, 0, dz), IVec3::new(0, stem_top - origin.y, 0));
            }
        }
        for &(cluster, attach) in &coords {
            let from = IVec3::new(
                if cluster.x > origin.x { origin.x + self.width - 1 } else { origin.x },
                attach,
                if cluster.z > origin.z { origin.z + self.width - 1 } else { origin.z },
            );
            if self.min_altitude_factor * scaled <= (attach - origin.y) as f32 {
                self.logs(region, from, cluster - from);
            }
        }
        Some(origin)
    }

    fn logs(&self, region: &mut Region, from: IVec3, delta: IVec3) {
        for (pos, offset) in line(from, delta) {
            put(region, pos, self.log.along(offset.x, offset.z));
        }
    }
}
