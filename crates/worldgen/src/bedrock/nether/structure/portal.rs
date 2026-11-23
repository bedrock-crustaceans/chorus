use std::collections::HashMap;

use crate::shared::blocks::{AIR, BlockId, Blocks};
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::r#impl::ruined_portal::portal_age::{FULL, LAVA_NEIGHBOURS, SLABS, SPREAD, SPREAD_RADIUS};
use crate::shared::structure::{BoundingBox, Grid, LargeFeatureSeed, Library, PaletteBlocks, Rotation, Spread, StateKind, Template, TemplateLibrary};

use super::super::{CHUNK_MASK, NetherBlocks, NetherChunk, SEA_LEVEL, TERRAIN_HEIGHT};
use chorus_level::dimension_definition::DimensionDefinition;

const SPACING: i32 = 25;
const SEPARATION: i32 = 10;
const SALT: i32 = 40_552_231;
pub const REACH: i32 = 1;
const PORTAL_MIN_Y: i32 = 15;
const PORTAL_HEIGHT_RANGE: i32 = 85;
const GIANT_CHANCE: f32 = 0.05;
const REGULAR_PORTALS: i32 = 10;
const GIANT_PORTALS: i32 = 3;
const SPREAD_HEIGHT_LIMIT: i32 = 4;
const MAGMA_CHANCE: f32 = 0.05;
const NETHERRACK_MAGMA_CHANCE: f32 = 0.1;
const OBSIDIAN_CHANCE: f32 = 0.2;
const LAVA_MAGMA_CHANCE: f32 = 0.2;
const GOLD_CHANCE: f32 = 0.3;
const HALF: f32 = 0.5;
const COLUMN_FLOOR: i32 = 1;
const COLUMN_STEPS: i32 = 8;
const NAMESPACE_LENGTH: usize = "minecraft:".len();
const WALL_ALONE: &str = "wall_connection_type_east=none,wall_connection_type_north=none,wall_connection_type_south=none,wall_connection_type_west=none,wall_post_bit=0";
const BOTTOM_SLAB: &str = "minecraft:vertical_half=bottom";

pub const PORTAL_GRID: Grid = Grid::new(SPACING, SEPARATION, SALT, Spread::Linear);

pub struct PortalPlan {
    writes: Vec<(i32, i32, i32, BlockId)>,
}

impl PortalPlan {
    pub fn apply(&self, chunk: &mut NetherChunk, (chunk_x, chunk_z): (i32, i32)) {
        for &(x, y, z, block) in &self.writes {
            if x >> 4 == chunk_x && z >> 4 == chunk_z {
                chunk.set(x & CHUNK_MASK, y, z & CHUNK_MASK, block);
            }
        }
    }
}

pub fn is_start(level_seed: u32, chunk: (i32, i32)) -> bool {
    PORTAL_GRID.candidate(level_seed, chunk) == chunk
}

pub fn start_cells_near(level_seed: u32, chunk: (i32, i32)) -> Vec<(i32, i32)> {
    let region = |value: i32| value.div_euclid(SPACING);
    let mut cells = Vec::new();
    for region_x in region(chunk.0 - REACH)..=region(chunk.0 + REACH) {
        for region_z in region(chunk.1 - REACH)..=region(chunk.1 + REACH) {
            let candidate = PORTAL_GRID.candidate(level_seed, (region_x * SPACING, region_z * SPACING));
            if (candidate.0 - chunk.0).abs() <= REACH && (candidate.1 - chunk.1).abs() <= REACH {
                cells.push(candidate);
            }
        }
    }
    cells
}

struct View<'a> {
    start: (i32, i32),
    definition: DimensionDefinition,
    get: &'a dyn Fn(i32, i32, i32) -> BlockId,
    blocks: &'a Blocks,
    ids: &'a NetherBlocks,
    palette: &'a PaletteBlocks,
    overlay: HashMap<(i32, i32, i32), BlockId>,
    writes: Vec<(i32, i32, i32, BlockId)>,
}

impl View<'_> {
    fn in_world(&self, y: i32) -> bool {
        (self.definition.min_y..self.definition.top()).contains(&y)
    }

    fn in_view(&self, x: i32, z: i32) -> bool {
        ((x >> 4) - self.start.0).abs() <= REACH && ((z >> 4) - self.start.1).abs() <= REACH
    }

    fn get(&self, x: i32, y: i32, z: i32) -> BlockId {
        if !self.in_world(y) || !self.in_view(x, z) {
            return AIR;
        }
        self.overlay.get(&(x, y, z)).copied().unwrap_or_else(|| (self.get)(x, y, z))
    }

    fn set(&mut self, x: i32, y: i32, z: i32, block: BlockId) {
        if !self.in_world(y) || !self.in_view(x, z) {
            return;
        }
        self.overlay.insert((x, y, z), block);
        self.writes.push((x, y, z, block));
    }

    fn short_name(&self, block: BlockId) -> &str {
        &self.blocks.name_of(block)[NAMESPACE_LENGTH..]
    }

    fn solid(&self, block: BlockId) -> bool {
        self.blocks.entry(block).solid
    }

    fn top(&self, x: i32, z: i32) -> i32 {
        let water = self.ids.water;
        let mut y = TERRAIN_HEIGHT - 1;
        while y > self.definition.min_y && (self.solid(self.get(x, y, z)) || self.get(x, y, z) == water) {
            y -= 1;
        }
        while y >= self.definition.min_y && !(self.solid(self.get(x, y, z)) || self.get(x, y, z) == water) {
            y -= 1;
        }
        (y + 1).max(self.definition.min_y)
    }

    fn supports(&self, block: BlockId) -> bool {
        let name = self.short_name(block);
        !name.ends_with("leaves") && (block == self.ids.water || self.solid(block))
    }
}

pub fn plan(
    start: (i32, i32),
    level_seed: u32,
    definition: DimensionDefinition,
    get: &dyn Fn(i32, i32, i32) -> BlockId,
    ids: &NetherBlocks,
    blocks: &Blocks,
    palette: &PaletteBlocks,
    library: &TemplateLibrary,
) -> PortalPlan {
    let mut random = MersenneTwister::new(LargeFeatureSeed::new(level_seed).chunk(start.0, start.1));
    let mut view = View {
        definition,
        start,
        get,
        blocks,
        ids,
        palette,
        overlay: HashMap::new(),
        writes: Vec::new(),
    };
    let surface = random.next_float() >= HALF;
    let rotation = Rotation::from_index(random.next_int_bounded(4));
    let mirror = random.next_float() >= HALF;
    let (x0, z0) = (start.0 * 16, start.1 * 16);
    let h0 = view.top(x0, z0);
    let name = if random.next_float() >= GIANT_CHANCE {
        format!("ruined_portal/portal_{}", random.next_int_bounded(REGULAR_PORTALS) + 1)
    } else {
        format!("ruined_portal/giant_portal_{}", random.next_int_bounded(GIANT_PORTALS) + 1)
    };
    let template = library.get(&name).expect("ruined portal template");
    let (size_x, size_y, size_z) = template.size;
    let (dx, _, dz) = rotation.apply((size_x, 0, if mirror { -size_z } else { size_z }));
    let mut bounds = BoundingBox::new(x0.min(x0 + dx), h0 - 1, z0.min(z0 + dz), x0.max(x0 + dx), h0 - 1 + size_y, z0.max(z0 + dz));
    let y = random.next_int_bounded(PORTAL_HEIGHT_RANGE) + PORTAL_MIN_Y;
    bounds = bounds.shifted_y(y - bounds.y0);
    while PORTAL_MIN_Y < bounds.y0 {
        let corners = [(bounds.x0, bounds.z0), (bounds.x0, bounds.z1), (bounds.x1, bounds.z0), (bounds.x1, bounds.z1)];
        let held = corners.iter().filter(|&&(x, z)| view.supports(view.get(x, bounds.y0 - 1, z))).count();
        if held >= 3 {
            break;
        }
        bounds = bounds.shifted_y(-1);
    }
    spread_netherrack(&mut view, &mut random, &bounds, !surface);
    let ignore_air = bounds.y0 < SEA_LEVEL;
    let library_view = Library { templates: library, palette };
    let chests = place_template(&mut view, &library_view, &template, rotation, mirror, ignore_air, (x0, bounds.y0, z0));
    random.skip(chests);
    for x in bounds.x0 + 1..bounds.x1 {
        for z in bounds.z0 + 1..bounds.z1 {
            if view.short_name(view.get(x, bounds.y0, z)) == "netherrack" {
                nether_column(&mut view, &mut random, (x, bounds.y0 - 1, z));
            }
        }
    }
    age(&mut view, &mut random, &bounds, &library_view);
    PortalPlan { writes: view.writes }
}

fn spread_netherrack(view: &mut View, random: &mut MersenneTwister, bounds: &BoundingBox, buried: bool) {
    let n = (bounds.x1 - bounds.x0 + bounds.z1 - bounds.z0 + 2) / 4;
    let extra = random.next_int_bounded((-7).max(-n) + 8);
    let centre_x = bounds.x0 + (bounds.x1 - bounds.x0 + 1) / 2;
    let centre_z = bounds.z0 + (bounds.z1 - bounds.z0 + 1) / 2;
    for x in centre_x - SPREAD_RADIUS..=centre_x + SPREAD_RADIUS {
        for z in centre_z - SPREAD_RADIUS..=centre_z + SPREAD_RADIUS {
            let distance = (centre_x - x).abs() + (centre_z - z).abs() + extra;
            if distance >= SPREAD_RADIUS || random.next_float() >= SPREAD[distance.max(0) as usize] {
                continue;
            }
            let height = view.top(x, z) - 1;
            let y = if buried { height.min(bounds.y0) } else { height };
            if (bounds.y0 - y).abs() < SPREAD_HEIGHT_LIMIT {
                let block = nether_block(view, random);
                view.set(x, y, z, block);
                nether_column(view, random, (x, y - 1, z));
            }
        }
    }
}

fn nether_block(view: &View, random: &mut MersenneTwister) -> BlockId {
    if random.next_float() < MAGMA_CHANCE {
        view.palette.named("magma", "", false)
    } else {
        view.palette.named("netherrack", "", false)
    }
}

fn nether_column(view: &mut View, random: &mut MersenneTwister, (x, y, z): (i32, i32, i32)) {
    let block = nether_block(view, random);
    view.set(x, y, z, block);
    for step in COLUMN_FLOOR..=COLUMN_STEPS {
        if random.next_float() >= HALF {
            break;
        }
        let block = nether_block(view, random);
        view.set(x, y - step, z, block);
    }
}

fn place_template(view: &mut View, library: &Library, template: &Template, rotation: Rotation, mirror: bool, ignore_air: bool, origin: (i32, i32, i32)) -> usize {
    let rotation_index = rotation as usize;
    let mut chests = 0;
    for (index, block) in template.blocks.iter().enumerate() {
        if library.templates.kind(block.state) != StateKind::Block {
            continue;
        }
        if ignore_air && library.templates.entry(library.templates.rotated(block.state, 0)).name == "air" {
            continue;
        }
        let local = (block.x as i32, block.y as i32, block.z as i32);
        let local = if mirror { (local.0, local.1, -local.2) } else { local };
        let (dx, dy, dz) = rotation.apply(local);
        let mut palette_index = library.templates.rotated(block.state, rotation_index);
        if mirror {
            palette_index = library.mirror(palette_index, rotation);
        }
        view.set(origin.0 + dx, origin.1 + dy, origin.2 + dz, library.palette.block(palette_index));
        chests += usize::from(template.loot_chests.contains(&index));
    }
    chests
}

fn state_value<'a>(states: &'a str, key: &str) -> &'a str {
    states.split(',').filter_map(|pair| pair.split_once('=')).find(|(name, _)| *name == key).map_or("0", |(_, value)| value)
}

fn stairs(view: &View, name: &str, upside_down: &str, weirdo: &str) -> BlockId {
    view.palette
        .named(name, &format!("minecraft:corner=none,upside_down_bit={upside_down},weirdo_direction={weirdo}"), false)
}

fn random_stairs(view: &View, random: &mut MersenneTwister, name: &str) -> BlockId {
    let weirdo = random.next_int_bounded(4);
    let upside_down = if random.next_float() < HALF { "1" } else { "0" };
    stairs(view, name, upside_down, &weirdo.to_string())
}

fn blackstone(view: &View, block: BlockId, random: &mut MersenneTwister) -> Option<BlockId> {
    let name = view.short_name(block);
    let plain = |name: &str| view.palette.named(name, "", false);
    Some(if name == "stone" {
        let choices = [plain("polished_blackstone"), random_stairs(view, random, "polished_blackstone_stairs")];
        choices[random.next_int_bounded(2) as usize]
    } else if name == "chiseled_stone_bricks" {
        plain("chiseled_polished_blackstone")
    } else if FULL.contains(&name) {
        let choices = [
            plain("polished_blackstone_bricks"),
            plain("cracked_polished_blackstone_bricks"),
            random_stairs(view, random, "polished_blackstone_brick_stairs"),
        ];
        choices[random.next_int_bounded(3) as usize]
    } else if name == "stone_brick_stairs" {
        let states = view.palette.states_of(block).unwrap_or("");
        let (weirdo, upside_down) = (state_value(states, "weirdo_direction"), state_value(states, "upside_down_bit"));
        if random.next_float() >= HALF {
            view.palette.named("polished_blackstone_brick_slab", BOTTOM_SLAB, false)
        } else {
            stairs(view, "polished_blackstone_brick_stairs", upside_down, weirdo)
        }
    } else if SLABS.contains(&name) {
        view.palette.named("polished_blackstone_brick_slab", BOTTOM_SLAB, false)
    } else if name.ends_with("_wall") && view.palette.states_of(block).unwrap_or("").contains("wall_post_bit") {
        view.palette.named("polished_blackstone_brick_wall", WALL_ALONE, false)
    } else if name == "iron_bars" {
        view.palette.named("iron_chain", "pillar_axis=y", false)
    } else {
        return None;
    })
}

fn age(view: &mut View, random: &mut MersenneTwister, bounds: &BoundingBox, library: &Library) {
    let plain = |name: &str| library.palette.named(name, "", false);
    for x in bounds.x0..=bounds.x1 {
        for y in bounds.y0..=bounds.y1 {
            for z in bounds.z0..=bounds.z1 {
                let block = view.get(x, y, z);
                let name = view.short_name(block);
                let aged = if let Some(aged) = blackstone(view, block, random) {
                    aged
                } else if name == "netherrack" {
                    if random.next_float() >= NETHERRACK_MAGMA_CHANCE {
                        continue;
                    }
                    plain("magma")
                } else if name == "obsidian" {
                    if random.next_float() >= OBSIDIAN_CHANCE {
                        continue;
                    }
                    plain("crying_obsidian")
                } else if name == "lava" || name == "flowing_lava" {
                    if random.next_float() >= LAVA_MAGMA_CHANCE {
                        continue;
                    }
                    plain("magma")
                } else if name == "gold_block" {
                    if random.next_float() >= GOLD_CHANCE {
                        continue;
                    }
                    let lava = LAVA_NEIGHBOURS.iter().any(|&(dx, dy, dz)| {
                        let other = view.short_name(view.get(x + dx, y + dy, z + dz));
                        other == "lava" || other == "flowing_lava"
                    });
                    if lava { view.ids.lava } else { AIR }
                } else {
                    continue;
                };
                view.set(x, y, z, aged);
            }
        }
    }
}
