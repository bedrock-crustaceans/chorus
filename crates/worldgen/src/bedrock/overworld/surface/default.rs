use super::column::Column;
use super::data::Materials;
use super::palette::Palette;
use crate::shared::blocks::BlockId;
use crate::shared::rng::mersenne::MersenneTwister;

const NO_WATER: i32 = 0x8000_03e8u32 as i32;
const WATER_BAND_BELOW: i32 = 4;
const WATER_BAND_ABOVE: i32 = 1;
const ICE_TEMPERATURE: f32 = 0.15;
const SAND_RUN_RANDOM: i32 = 4;

pub struct DefaultColumn {
    pub start_y: i32,
    pub lower_bound: i32,
    pub run_depth: i32,
    pub materials: Materials,
    pub steep: Option<BlockId>,
    pub sea_floor_depth: i32,
    pub temperature: f32,
}

fn solidify_above(palette: &Palette, column: &mut Column, y: i32, foundation: BlockId) {
    let above_y = y + 1;
    if above_y > column.max_y() {
        return;
    }
    let above = column.get(above_y);
    if above == palette.sand {
        column.set(above_y, palette.sandstone);
    } else if above == palette.red_sand {
        column.set(above_y, palette.red_sandstone);
    } else if above == palette.gravel {
        column.set(above_y, foundation);
    }
}

pub fn build(palette: &Palette, column: &mut Column, spec: &DefaultColumn, random: &mut MersenneTwister) {
    let materials = &spec.materials;
    let foundation = materials.foundation;
    let (mut top_base, mut mid_base) = (materials.top, materials.mid);
    let (mut top_variable, mut mid_variable) = (top_base, mid_base);
    let mut run = -1;
    let mut water = NO_WATER;
    for y in (spec.lower_bound + 1..=spec.start_y).rev() {
        let block = column.get(y);
        if block == palette.bedrock {
            continue;
        }
        if run >= 0 && foundation != palette.netherrack && (block == palette.air || palette.is_liquid(block)) {
            solidify_above(palette, column, y, foundation);
        }
        if block == palette.air {
            run = -1;
            water = NO_WATER;
            continue;
        }
        if let Some(steep) = spec.steep {
            top_base = steep;
            mid_base = steep;
            top_variable = steep;
            mid_variable = steep;
        }
        let water_level = water;
        if !palette.is_foundation(block) {
            let mut current = block;
            if block == palette.water {
                current = materials.sea;
                column.set(y, current);
            }
            if palette.is_liquid(current) || current == palette.packed_ice || current == palette.blue_ice {
                water = water.max(y + 1);
            }
            continue;
        }
        if run == -1 {
            let (mut top_next, mid_next) = if spec.run_depth > 0 {
                let in_range = !(y < water_level - WATER_BAND_BELOW || y > water_level + WATER_BAND_ABOVE);
                if in_range { (top_base, mid_base) } else { (top_variable, mid_variable) }
            } else {
                (palette.air, foundation)
            };
            if y < water_level && top_next == palette.air {
                top_next = if materials.sea == palette.water && spec.temperature < ICE_TEMPERATURE {
                    palette.ice
                } else {
                    materials.sea
                };
            }
            run = spec.run_depth;
            if y >= water_level - 1 {
                column.set(y, top_next);
                mid_variable = mid_next;
                top_variable = top_next;
            } else if y < water_level - (spec.sea_floor_depth + spec.run_depth) || y == spec.lower_bound + 1 {
                column.set(y, materials.sea_floor);
                mid_variable = foundation;
                top_variable = palette.air;
            } else {
                column.set(y, mid_next);
                mid_variable = mid_next;
                top_variable = top_next;
            }
        } else if run > 0 {
            column.set(y, mid_variable);
            run -= 1;
            if run == 0 && palette.is_sand(mid_variable) {
                run = random.next_int_bounded(SAND_RUN_RANDOM) + (y - water_level).max(0);
                mid_variable = palette.sandstone;
            }
        } else {
            column.set(y, foundation);
        }
    }
}
