use super::canyon::{Canyon, Step, max_y};
use super::cave::Segment;
use super::classes::Classes;
use super::fluids::Fluids;
use crate::shared::CHUNK_WIDTH;
use crate::shared::blocks::Blocks;
use crate::shared::chunk::proto::ProtoChunk;
use crate::shared::math::compare::{not_greater, not_less};

const CHUNK_MIDDLE: i32 = 8;
const NEIGHBOUR_MARGIN: f32 = 16.0;
const REACH_MARGIN: f32 = 2.0;
const SAND_CHECK_BELOW_TOP: i32 = 64;
const SAND_STACK_HEIGHT: i32 = 3;
const CELL_CENTRE: f32 = 0.5;
const CANYON_WIDTH_INDEX_OFFSET: i32 = 2;

struct Bounds {
    x: (i32, i32),
    y: (i32, i32),
    z: (i32, i32),
}

pub fn reaches(segment: &Segment, chunk_x: i32, chunk_z: i32) -> bool {
    let (middle_x, middle_z) = ((chunk_x * CHUNK_WIDTH + CHUNK_MIDDLE) as f32, (chunk_z * CHUNK_WIDTH + CHUNK_MIDDLE) as f32);
    let (dx, dz) = (segment.center[0] - middle_x, segment.center[2] - middle_z);
    let remaining = segment.length - segment.step;
    let distance = (dz * dz + dx * dx) - (remaining * remaining) as f32;
    let reach = segment.thickness + REACH_MARGIN + NEIGHBOUR_MARGIN;
    if reach * reach < distance {
        return false;
    }
    let margin = segment.horizontal_radius + segment.horizontal_radius + NEIGHBOUR_MARGIN;
    let floor_x = segment.center[0].floor() as i32 as f32;
    let floor_z = segment.center[2].floor() as i32 as f32;
    !(middle_x - margin > floor_x || floor_x > middle_x + margin || middle_z - margin > floor_z || floor_z > middle_z + margin)
}

fn horizontal_bounds(center: f32, radius: f32, chunk_origin: i32) -> (i32, i32) {
    let low = ((center - radius).floor() as i32 - chunk_origin - 1).max(0);
    let high = ((center + radius).floor() as i32 - chunk_origin).min(CHUNK_WIDTH - 1) + 1;
    (low, high)
}

fn cave_bounds(segment: &Segment, chunk_x: i32, chunk_z: i32, height_cap: i32) -> Bounds {
    let [x, y, z] = segment.center;
    let top = (y + segment.vertical_radius).floor() as i32;
    let high = if height_cap <= top { height_cap } else { top.max(0) + 1 };
    let bottom = (y - segment.vertical_radius).floor() as i32 - 1;
    let low = if bottom > high { high } else { bottom.max(1) };
    Bounds {
        x: horizontal_bounds(x, segment.horizontal_radius, chunk_x * CHUNK_WIDTH),
        y: (low, high),
        z: horizontal_bounds(z, segment.horizontal_radius, chunk_z * CHUNK_WIDTH),
    }
}

pub fn carve_cave(classes: &Classes, blocks: &Blocks, fluids: &mut Fluids, chunk: &mut ProtoChunk, segment: &Segment, height_cap: i32) {
    let bounds = cave_bounds(segment, chunk.x, chunk.z, height_cap);
    let (horizontal_radius, vertical_radius) = (segment.horizontal_radius, segment.vertical_radius);
    for x in bounds.x.0..bounds.x.1 {
        let dx = (((chunk.x * CHUNK_WIDTH + x) as f32 + CELL_CENTRE) - segment.center[0]) / horizontal_radius;
        let dx_squared = dx * dx;
        for z in bounds.z.0..bounds.z.1 {
            let dz = (((chunk.z * CHUNK_WIDTH + z) as f32 + CELL_CENTRE) - segment.center[2]) / horizontal_radius;
            let dz_squared = dz * dz;
            if not_greater(1.0, dx_squared + dz_squared) {
                continue;
            }
            for local_y in (bounds.y.0 + 1..=bounds.y.1).rev() {
                let dy = (((local_y - 1) as f32 + CELL_CENTRE) - segment.center[1]) / vertical_radius;
                if not_greater(dy, segment.floor_level) || not_less(dy * dy + dx_squared + dz_squared, 1.0) {
                    continue;
                }
                let y = local_y + chunk.min_y;
                let original = chunk.get(x, y, z);
                if !classes.cave_replaceable(blocks, original) {
                    continue;
                }
                if y + SAND_STACK_HEIGHT <= chunk.max_y() - SAND_CHECK_BELOW_TOP && (1..=SAND_STACK_HEIGHT).all(|offset| classes.is_sand(blocks, chunk.get(x, y + offset, z))) {
                    chunk.set(x, y + 1, z, classes.sandstone);
                }
                fluids.empty_cell(chunk, x, y, z);
                if classes.is_grass_or_mycelium(blocks, original) && classes.is_dirt(blocks, chunk.get(x, y - 1, z)) {
                    chunk.set(x, y - 1, z, original);
                }
            }
        }
    }
}

pub fn carve_canyon(classes: &Classes, blocks: &Blocks, fluids: &mut Fluids, chunk: &mut ProtoChunk, canyon: &Canyon, steps: &[Step]) {
    let (middle_x, middle_z) = ((chunk.x * CHUNK_WIDTH + CHUNK_MIDDLE) as f32, (chunk.z * CHUNK_WIDTH + CHUNK_MIDDLE) as f32);
    let reach = canyon.reach();
    for step in steps {
        let (dx, dz) = (step.center[0] - middle_x, step.center[2] - middle_z);
        let remaining = step.remaining as f32;
        if (dz * dz + dx * dx) - remaining * remaining > reach * reach {
            return;
        }
        let margin = step.horizontal_radius + step.horizontal_radius;
        let [x, _, z] = step.center;
        if (middle_x - NEIGHBOUR_MARGIN) - margin > x || (middle_z - NEIGHBOUR_MARGIN) - margin > z || x > (middle_x + NEIGHBOUR_MARGIN) + margin || z > margin + (NEIGHBOUR_MARGIN + middle_z) {
            continue;
        }
        carve_canyon_step(classes, blocks, fluids, chunk, canyon, step);
    }
}

fn canyon_horizontal_bounds(center: f32, radius: f32, chunk_origin: i32) -> (i32, i32) {
    let low = ((center - radius).floor() - chunk_origin as f32 + -1.0) as i32;
    let high = ((center + radius).floor() - chunk_origin as f32 + 1.0) as i32;
    (low.max(0), high.min(CHUNK_WIDTH))
}

fn carve_canyon_step(classes: &Classes, blocks: &Blocks, fluids: &mut Fluids, chunk: &mut ProtoChunk, canyon: &Canyon, step: &Step) {
    let (horizontal_radius, vertical_radius) = (step.horizontal_radius, step.vertical_radius);
    let [center_x, center_y, center_z] = step.center;
    let (x_low, x_high) = canyon_horizontal_bounds(center_x, horizontal_radius, chunk.x * CHUNK_WIDTH);
    let (z_low, z_high) = canyon_horizontal_bounds(center_z, horizontal_radius, chunk.z * CHUNK_WIDTH);
    let bottom = ((center_y - vertical_radius).floor() + -1.0) as i32;
    let top = ((center_y + vertical_radius).floor() + 1.0) as i32;
    let ceiling = max_y(chunk.height);
    let y_high = if ceiling < top { ceiling } else { top.max(1) };
    let y_low = if y_high < bottom { y_high } else { bottom.max(1) };
    for x in x_low..x_high {
        let dx = (((chunk.x * CHUNK_WIDTH + x) as f32 + CELL_CENTRE) - center_x) / horizontal_radius;
        for z in z_low..z_high {
            let dz = (((chunk.z * CHUNK_WIDTH + z) as f32 + CELL_CENTRE) - center_z) / horizontal_radius;
            let horizontal = dz * dz + dx * dx;
            if not_less(horizontal, 1.0) {
                continue;
            }
            let mut met_grass = false;
            for local_y in (y_low + 1..=y_high).rev() {
                let dy = (((local_y - 1) as f32 + CELL_CENTRE) - center_y) / vertical_radius;
                if not_less(
                    (dy * dy) / super::canyon::VERTICAL_DIVISOR + canyon.widths[(local_y - CANYON_WIDTH_INDEX_OFFSET) as usize] * horizontal,
                    1.0,
                ) {
                    continue;
                }
                let y = local_y + chunk.min_y;
                let block = chunk.get(x, y, z);
                met_grass |= classes.is_grass(blocks, block);
                if !classes.canyon_replaceable(blocks, block) {
                    continue;
                }
                fluids.empty_cell(chunk, x, y, z);
                if met_grass && classes.is_dirt(blocks, chunk.get(x, y - 1, z)) {
                    chunk.set(x, y - 1, z, classes.grass);
                }
            }
        }
    }
}
