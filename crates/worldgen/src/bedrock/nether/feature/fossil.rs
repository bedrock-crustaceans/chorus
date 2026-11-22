use super::super::decorate::{View, opaque_solid};
use super::Materials;
use crate::bedrock::overworld::ore::Cells;
use crate::shared::blocks::AIR;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::{Rotation, StateKind};
use chorus_block::block_id::{BONE_BLOCK, LAVA};

const FOSSIL_BOTTOM: i32 = 32;

const FOSSIL_FLOAT_ODDS: f32 = 0.5;

const FOSSIL_DIRECTIONS: i32 = 4;

pub const FOSSILS: [&str; 14] = [
    "nether_fossils/fossil_1",
    "nether_fossils/fossil_2",
    "nether_fossils/fossil_3",
    "nether_fossils/fossil_4",
    "nether_fossils/fossil_5",
    "nether_fossils/fossil_6",
    "nether_fossils/fossil_7",
    "nether_fossils/fossil_8",
    "nether_fossils/fossil_9",
    "nether_fossils/fossil_10",
    "nether_fossils/fossil_11",
    "nether_fossils/fossil_12",
    "nether_fossils/fossil_13",
    "nether_fossils/fossil_14",
];

pub(super) fn fossil(view: &mut View, random: &mut MersenneTwister, [x, y0, z]: [i32; 3], materials: &Materials) -> bool {
    let rotation = Rotation::from_index(random.next_int_bounded(FOSSIL_DIRECTIONS));
    let template = materials.templates.get(FOSSILS[random.next_int_bounded(FOSSILS.len() as i32) as usize]);
    let Some(template) = template else { return false };
    let (size_x, size_y, size_z) = template.size;
    let (fx, _, fz) = rotation.apply((size_x - 1, 0, size_z - 1));
    let corners = [(x, z), (x + fx, z), (x, z + fz), (x + fx, z + fz)];
    let mut y = y0;
    let y = loop {
        if y - 1 <= FOSSIL_BOTTOM {
            return false;
        }
        y -= 1;
        if y + size_y + 1 > view.max_y() {
            return false;
        }
        if corners.iter().any(|&(cx, cz)| view.blocks().name_of(view.get(cx, y, cz)) == LAVA) {
            return false;
        }
        let fits = corners.iter().all(|&(cx, cz)| {
            let ground = view.get(cx, y, cz);
            view.get(cx, y + size_y - 1, cz) == AIR && (opaque_solid(view.blocks(), ground) || ground == materials.ground)
        });
        if fits {
            break y;
        }
    };
    for block in &template.blocks {
        if materials.templates.kind(block.state) != StateKind::Block {
            continue;
        }
        if materials.palette.block(materials.templates.rotated(block.state, 0)) == materials.air {
            continue;
        }
        let (dx, dy, dz) = rotation.apply((block.x as i32, block.y as i32, block.z as i32));
        let placed = materials.palette.turned_block(block.state, rotation as usize);
        view.set(x + dx, y + 1 + dy, z + dz, placed);
    }
    if random.next_float() > FOSSIL_FLOAT_ODDS {
        return true;
    }
    let ghast = view.ids().dried_ghasts[random.next_int_bounded(FOSSIL_DIRECTIONS) as usize];
    let (far_x, far_z) = (x + fx, z + fz);
    let gx = if x < far_x { x + random.next_int_bounded(far_x - x) } else { x };
    let gz = if z < far_z { z + random.next_int_bounded(far_z - z) } else { z };
    for gy in y + 1..y + 1 + size_y {
        let there = view.get(gx, gy, gz);
        if there == AIR {
            if view.get(gx, gy - 1, gz) == materials.ghast_ground {
                view.set(gx, gy, gz, ghast);
            }
            break;
        }
        if view.blocks().name_of(there) == BONE_BLOCK {
            break;
        }
    }
    true
}
