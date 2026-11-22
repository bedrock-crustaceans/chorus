use super::Tree;
use super::site::{is_air, put};
use super::spec::Canopy;
use crate::bedrock::overworld::decoration::ore::Draw;
use crate::shared::chunk::region::Region;
use glam::IVec3;

const ROOF_DEPTH_OFFSET: i32 = 3;

fn upper_corner_allowed(j: i32, k: i32, outer: i32) -> bool {
    !(j == -outer && k <= 1 - outer || k == -outer && j == 1 - outer)
}

pub fn place_roofed<D: Draw>(canopy: &Canopy, _tree: &Tree, region: &mut Region, random: &mut D, at: IVec3) {
    let &Canopy::Roofed {
        leaf,
        height,
        core_width,
        outer_radius,
        inner_radius,
    } = canopy
    else {
        return;
    };
    let core = core_width.max(1);
    let walls = height - ROOF_DEPTH_OFFSET;
    let fill = |region: &mut Region, pos: IVec3| {
        if is_air(region, pos) {
            put(region, pos, leaf);
        }
    };
    if outer_radius != -1 {
        for j in -outer_radius..=0 {
            for k in -outer_radius..=0 {
                let (mirror_j, mirror_k) = (core - 1 - j, core - 1 - k);
                let corners = [(j, k), (mirror_j, k), (j, mirror_k), (mirror_j, mirror_k)];
                for (dx, dz) in corners {
                    fill(region, at + IVec3::new(dx, -1, dz));
                }
                if upper_corner_allowed(j, k, outer_radius) {
                    for (dx, dz) in corners {
                        fill(region, at + IVec3::new(dx, walls, dz));
                    }
                }
            }
        }
    }
    if random.boolean() {
        for dx in 0..core {
            for dz in 0..core {
                fill(region, at + IVec3::new(dx, walls + 1, dz));
            }
        }
    }
    let far = core - 1 + inner_radius;
    for layer in 0..walls.max(0) {
        for dx in -inner_radius..=far {
            for dz in -inner_radius..=far {
                let corner = (dx == -inner_radius || dx == far) && (dz == -inner_radius || dz == far);
                if corner || dx.abs() >= inner_radius && dz.abs() >= inner_radius {
                    continue;
                }
                fill(region, at + IVec3::new(dx, layer, dz));
            }
        }
    }
}
