use super::canvas::{Canvas, Cell};
use super::frame::{EAST, Frame, NORTH, SOUTH, Scatter, WEST};
use super::{Builder, Kind, Materials, Piece, fence_at};
use crate::shared::blocks::BlockId;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::{BoundingBox, Probed};

const SECTION_LENGTH: i32 = 5;
const MIN_SECTIONS: i32 = 2;
const SECTION_RANGE: i32 = 3;
const RAIL_ODDS: i32 = 3;
const SPIDER_ODDS: i32 = 23;
const CEILING_CHANCE: f32 = 0.8;
const WEB_CHANCE: f32 = 0.6;
const PLANK_ENDS_ODDS: i32 = 4;
const TORCH_CHANCE: f32 = 0.05;
const INTERIOR_RAIL_CHANCE: f32 = 0.7;
const BURIED_RAIL_CHANCE: f32 = 0.9;
const PILLAR_REACH_DOWN: i32 = 10;
const CHAIN_REACH_UP: i32 = 20;
const MAX_BUILD_Y: i32 = 319;
const SPAWNER_RANGE: i32 = 3;
const SIDE_WEBS: [(f32, i32, i32); 8] = [(0.1, 0, 2), (0.1, 2, 1), (0.1, 0, 3), (0.1, 2, 3), (0.05, 0, 0), (0.05, 2, 0), (0.05, 0, 4), (0.05, 2, 4)];
const SIDE_BRANCH_SPACING: i32 = 5;
const SIDE_BRANCH_START: i32 = 6;
const SIDE_BRANCH_ODDS: i32 = 5;
const END_BRANCH_ODDS: i32 = 4;
const END_BRANCH_HEIGHTS: i32 = 3;

#[derive(Debug, Clone)]
pub struct Corridor {
    rails: bool,
    spider: bool,
    sections: i32,
}

impl Corridor {
    pub fn find(builder: &Builder, random: &mut MersenneTwister, (x, y, z): Cell, direction: u8, depth: i32) -> Option<Piece> {
        let count = random.next_int_bounded(SECTION_RANGE) + MIN_SECTIONS;
        let bounds = (1..=count).rev().map(|sections| sections * SECTION_LENGTH).find_map(|length| {
            let bounds = match direction {
                SOUTH => BoundingBox::new(x, y, z, x + 2, y + 2, z + length - 1),
                WEST => BoundingBox::new(x + 1 - length, y, z, x, y + 2, z + 2),
                NORTH => BoundingBox::new(x, y, z + 1 - length, x + 2, y + 2, z),
                _ => BoundingBox::new(x, y, z, x + length - 1, y + 2, z + 2),
            };
            (!builder.collides(&bounds)).then_some(bounds)
        })?;
        let rails = random.next_int_bounded(RAIL_ODDS) == 0;
        let spider = !rails && random.next_int_bounded(SPIDER_ODDS) == 0;
        let span = if direction == SOUTH || direction == NORTH {
            bounds.z1 - bounds.z0 + 1
        } else {
            bounds.x1 - bounds.x0 + 1
        };
        let corridor = Corridor {
            rails,
            spider,
            sections: span / SECTION_LENGTH,
        };
        Some(Piece {
            frame: Frame {
                bounds,
                orientation: Some(direction),
                depth,
            },
            kind: Kind::Corridor(corridor),
        })
    }

    pub fn post_process(&self, frame: &Frame, materials: Materials, canvas: &mut Canvas, random: &mut MersenneTwister) -> Probed<()> {
        let planks = materials.planks(canvas.shaft);
        let length = self.sections * SECTION_LENGTH;
        self.carve(frame, canvas, random, length)?;
        let mut spawner_placed = false;
        for section in 0..self.sections {
            let z = section * SECTION_LENGTH + 2;
            if supported(frame, canvas, z)? {
                place_supports(frame, materials, canvas, random, z)?;
            }
            let base = section * SECTION_LENGTH;
            place_side_webs(frame, canvas, random, base)?;
            if self.spider && !spawner_placed {
                spawner_placed = place_spawner(frame, canvas, random, base)?;
            }
        }
        lay_floor(frame, canvas, length, planks)?;
        self.place_pillars(frame, materials, canvas, length)?;
        if self.rails {
            lay_rails(frame, canvas, random, length)?;
        }
        Ok(())
    }

    fn carve(&self, frame: &Frame, canvas: &mut Canvas, random: &mut MersenneTwister, length: i32) -> Probed<()> {
        let air = canvas.ids.air;
        frame.generate_box(canvas, (0, 0, 0), (2, 1, length - 1), air, air)?;
        frame.generate_maybe_box(
            canvas,
            random,
            Scatter {
                chance: CEILING_CHANCE,
                interior_only: false,
            },
            (0, 2, 0),
            (2, 2, length - 1),
            (air, air),
        )?;
        if self.spider {
            frame.generate_maybe_box(
                canvas,
                random,
                Scatter {
                    chance: WEB_CHANCE,
                    interior_only: true,
                },
                (0, 0, 0),
                (2, 1, length - 1),
                (canvas.shaft.web, air),
            )?;
        }
        Ok(())
    }

    fn place_pillars(&self, frame: &Frame, materials: Materials, canvas: &mut Canvas, length: i32) -> Probed<()> {
        let planks = materials.planks(canvas.shaft);
        let mut pillars = vec![2];
        if self.sections > 1 {
            pillars.push(length - 3);
        }
        for z in pillars {
            for x in [0, 2] {
                if frame.get(canvas, (x, -1, z))? == planks {
                    pillar_or_chain(frame, materials, canvas, (x, z))?;
                }
            }
        }
        Ok(())
    }
}

fn place_supports(frame: &Frame, materials: Materials, canvas: &mut Canvas, random: &mut MersenneTwister, z: i32) -> Probed<()> {
    let (air, planks) = (canvas.ids.air, materials.planks(canvas.shaft));
    let orientation = frame.orientation.unwrap_or(SOUTH) as usize;
    for x in [0, 2] {
        for y in 0..=1 {
            let at = frame.world((x, y, z));
            let fence = fence_at(materials, canvas, at)?;
            frame.place(canvas, fence, (x, y, z))?;
            if canvas.bounds.contains(at.0, at.1, at.2) && materials.is_fence(canvas.shaft, canvas.get(at.0, at.1, at.2)?) {
                canvas.fences.insert(at);
            }
        }
    }
    if random.next_int_bounded(PLANK_ENDS_ODDS) == 0 {
        frame.generate_box(canvas, (0, 2, z), (0, 2, z), planks, air)?;
        frame.generate_box(canvas, (2, 2, z), (2, 2, z), planks, air)?;
    } else {
        frame.generate_box(canvas, (0, 2, z), (2, 2, z), planks, air)?;
        if random.next_float() < TORCH_CHANCE {
            frame.place(canvas, canvas.shaft.torches[orientation], (1, 2, z - 1))?;
        }
        if random.next_float() < TORCH_CHANCE {
            frame.place(canvas, canvas.shaft.torches[(orientation + 2) % 4], (1, 2, z + 1))?;
        }
    }
    Ok(())
}

fn place_side_webs(frame: &Frame, canvas: &mut Canvas, random: &mut MersenneTwister, base: i32) -> Probed<()> {
    for (chance, x, at) in SIDE_WEBS.map(|(chance, x, along)| (chance, x, base + along)) {
        if frame.interior(canvas, (x, 3, at))? {
            frame.maybe_block_if_not_floating(canvas, random, chance, (x, 2, at - 1), canvas.shaft.web)?;
        }
    }
    Ok(())
}

fn place_spawner(frame: &Frame, canvas: &mut Canvas, random: &mut MersenneTwister, base: i32) -> Probed<bool> {
    let spawner = random.next_int_bounded(SPAWNER_RANGE) + base + 1;
    let (wx, wy, wz) = frame.world((1, 0, spawner));
    if canvas.bounds.contains(wx, wy, wz) && canvas.interior(wx, wy, wz)? {
        canvas.set(wx, wy, wz, canvas.shaft.spawner);
        return Ok(true);
    }
    Ok(false)
}

fn lay_floor(frame: &Frame, canvas: &mut Canvas, length: i32, planks: BlockId) -> Probed<()> {
    for x in 0..=2 {
        for z in 0..length.max(1) {
            let (wx, wy, wz) = frame.world((x, -1, z));
            let below = canvas.get(wx, wy, wz)?;
            if canvas.interior(wx, wy + 1, wz)? && (canvas.is_air(below) || below == canvas.shaft.chain) {
                canvas.set(wx, wy, wz, planks);
            }
        }
    }
    Ok(())
}

fn lay_rails(frame: &Frame, canvas: &mut Canvas, random: &mut MersenneTwister, length: i32) -> Probed<()> {
    let rail = if matches!(frame.orientation, Some(WEST) | Some(EAST)) {
        canvas.shaft.rail_east_west
    } else {
        canvas.shaft.rail_north_south
    };
    for z in 0..length.max(1) {
        if canvas.full_solid(frame.get(canvas, (1, -1, z))?) {
            let chance = if frame.interior(canvas, (1, 0, z))? { INTERIOR_RAIL_CHANCE } else { BURIED_RAIL_CHANCE };
            if random.next_float() < chance {
                frame.place(canvas, rail, (1, 0, z))?;
            }
        }
    }
    Ok(())
}

fn supported(frame: &Frame, canvas: &Canvas, z: i32) -> Probed<bool> {
    let ((x0, y, z0), (x1, _, z1)) = (frame.world((0, 3, z)), frame.world((2, 3, z)));
    for x in x0.min(x1)..=x0.max(x1) {
        for z in z0.min(z1)..=z0.max(z1) {
            if canvas.is_air(canvas.get(x, y, z)?) {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

fn holds_chain(canvas: &Canvas, materials: Materials, block: BlockId) -> bool {
    (canvas.full_solid(block) || materials.is_fence(canvas.shaft, block) || block == canvas.shaft.chain) && !canvas.falls(block)
}

fn pillar_or_chain(frame: &Frame, materials: Materials, canvas: &mut Canvas, (x, z): (i32, i32)) -> Probed<()> {
    let (wx, top, wz) = frame.world((x, 0, z));
    if !canvas.bounds.contains(wx, top - 1, wz) {
        return Ok(());
    }
    let (mut down, mut up) = (true, true);
    let mut reach = 1;
    let lava = canvas.ids.lava;
    while down || up {
        if down {
            let y = top - 1 - reach;
            let block = canvas.get(wx, y, wz)?;
            if !canvas.replaceable(block) || block == lava {
                down = false;
                if block != lava && !canvas.shaft.is_rail(block) {
                    let log = materials.log(canvas.shaft);
                    (y + 1..=top - 2).for_each(|y| canvas.set(wx, y, wz, log));
                    return Ok(());
                }
            } else {
                down = reach <= PILLAR_REACH_DOWN && canvas.definition.min_y + 1 < y;
            }
        }
        if up {
            let y = top - 1 + reach;
            let block = canvas.get(wx, y, wz)?;
            if !canvas.replaceable(block) && holds_chain(canvas, materials, block) {
                let fence = fence_at(materials, canvas, (wx, top, wz))?;
                canvas.set(wx, top, wz, fence);
                canvas.fences.insert((wx, top, wz));
                let chain = canvas.shaft.chain;
                (top + 1..=top + reach - 2).for_each(|y| canvas.set(wx, y, wz, chain));
                return Ok(());
            }
            up = canvas.replaceable(block) && y < MAX_BUILD_Y && reach < CHAIN_REACH_UP + 1;
        }
        reach += 1;
    }
    Ok(())
}

pub fn children(builder: &mut Builder, index: usize, random: &mut MersenneTwister) {
    let (bounds, orientation, depth) = {
        let frame = &builder.pieces[index].frame;
        (frame.bounds, frame.orientation.unwrap_or(SOUTH), frame.depth)
    };
    let pick = random.next_int_bounded(END_BRANCH_ODDS);
    let y = random.next_int_bounded(END_BRANCH_HEIGHTS) + bounds.y0 - 1;
    let (at, direction) = match (orientation, pick) {
        (SOUTH, 0 | 1) => ((bounds.x0, y, bounds.z1 + 1), SOUTH),
        (SOUTH, 2) => ((bounds.x0 - 1, y, bounds.z1 - 3), WEST),
        (SOUTH, _) => ((bounds.x1 + 1, y, bounds.z1 - 3), EAST),
        (WEST, 0 | 1) => ((bounds.x0 - 1, y, bounds.z0), WEST),
        (WEST, 2) => ((bounds.x0, y, bounds.z0 - 1), NORTH),
        (WEST, _) => ((bounds.x0, y, bounds.z1 + 1), SOUTH),
        (NORTH, 0 | 1) => ((bounds.x0, y, bounds.z0 - 1), NORTH),
        (NORTH, 2) => ((bounds.x0 - 1, y, bounds.z0), WEST),
        (NORTH, _) => ((bounds.x1 + 1, y, bounds.z0), EAST),
        (_, 0 | 1) => ((bounds.x1 + 1, y, bounds.z0), EAST),
        (_, 2) => ((bounds.x1 - 3, y, bounds.z0 - 1), NORTH),
        (_, _) => ((bounds.x1 - 3, y, bounds.z1 + 1), SOUTH),
    };
    builder.add(random, at, direction, depth);
    if depth >= super::MAX_DEPTH {
        return;
    }
    if orientation == SOUTH || orientation == NORTH {
        let mut z = bounds.z0 + SIDE_BRANCH_START;
        while z <= bounds.z1 {
            match random.next_int_bounded(SIDE_BRANCH_ODDS) {
                0 => builder.add(random, (bounds.x0 - 1, bounds.y0, z - 3), WEST, depth + 1),
                1 => builder.add(random, (bounds.x1 + 1, bounds.y0, z - 3), EAST, depth + 1),
                _ => {}
            }
            z += SIDE_BRANCH_SPACING;
        }
    } else {
        let mut x = bounds.x0 + SIDE_BRANCH_START;
        while x <= bounds.x1 {
            match random.next_int_bounded(SIDE_BRANCH_ODDS) {
                0 => builder.add(random, (x - 3, bounds.y0, bounds.z0 - 1), NORTH, depth + 1),
                1 => builder.add(random, (x - 3, bounds.y0, bounds.z1 + 1), SOUTH, depth + 1),
                _ => {}
            }
            x += SIDE_BRANCH_SPACING;
        }
    }
}
