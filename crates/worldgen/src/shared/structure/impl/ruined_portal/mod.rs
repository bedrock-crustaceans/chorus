pub mod portal_age;

use crate::shared::blocks::BlockId;
use crate::shared::blocks::Blocks;
use crate::shared::chunk::proto::ProtoChunk;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::bounds::BoundingBox;
use crate::shared::structure::builder::{Built, Region, StructureBuilder, StructureContext, place_plan};
use crate::shared::structure::place::{Library, Options, Rotation};
use crate::shared::structure::plan::Plan;
use crate::shared::structure::plan::PlanWrites;
use crate::shared::structure::probe::{Probed, StartProbe};
use crate::shared::structure::{Reaching, StructureKind, Structures};
pub use portal_age::declare_states;
use portal_age::{SPREAD, SPREAD_RADIUS};
use std::sync::Arc;

pub const OCEAN: &[i32] = &[0, 24, 40, 41, 42, 43, 44, 45, 46, 47];
pub const DESERT: &[i32] = &[2];
pub const JUNGLE: &[i32] = &[21, 23, 48];
pub const SWAMP: &[i32] = &[6];
pub const ICE: &[i32] = &[12];
const HIGH: &[i32] = &[3, 34, 131];
const SUNK_FLOOR: i32 = 15;
const HIGH_SINK_ABOVE_SEA: i32 = 7;
const MAGMA_CHANCE: f32 = 0.05;
pub const SURFACE_CHANCE: f32 = 0.5;
pub const GIANT_CHANCE: f32 = 0.05;
const BURIED_AIR_CHANCE: f32 = 0.5;
const COLUMN_DEPTH: i32 = 8;
const SUPPORT_CORNERS: usize = 3;

pub struct Site {
    pub centre_biome: i32,
}

struct Context<'a, 'b> {
    plan: &'b mut Plan<'a>,
    library: &'b Library<'b>,
    probe: &'b dyn StartProbe,
    start: (i32, i32),
    desert: bool,
    sea_floor: bool,
}

impl Context<'_, '_> {
    fn top(&self, x: i32, z: i32) -> Probed<i32> {
        if ((x >> 4) - self.start.0).abs() > 1 || ((z >> 4) - self.start.1).abs() > 1 {
            return Ok(self.probe.definition().min_y);
        }
        let water = !self.sea_floor;
        let definition = self.probe.definition();
        for y in (definition.min_y..definition.top()).rev() {
            let block = self.plan.world(self.probe, x, y, z)?;
            if self.plan.is_solid(block) || water && self.plan.is_water(block) {
                return Ok(y + 1);
            }
        }
        Ok(definition.min_y)
    }

    fn nether_block(&self, random: &mut MersenneTwister) -> BlockId {
        self.library.palette.named(if random.next_float() < MAGMA_CHANCE { "magma" } else { "netherrack" }, "", false)
    }

    fn nether_column(&mut self, random: &mut MersenneTwister, (x, y, z): (i32, i32, i32)) {
        let block = self.nether_block(random);
        self.plan.set(x, y, z, block);
        for depth in 1..=COLUMN_DEPTH {
            if random.next_float() >= 0.5 {
                break;
            }
            let block = self.nether_block(random);
            self.plan.set(x, y - depth, z, block);
        }
    }

    fn spread_netherrack(&mut self, random: &mut MersenneTwister, b: &BoundingBox, buried: bool) -> Probed<()> {
        let n = (b.x1 - b.x0 + b.z1 - b.z0 + 2) / 4;
        let extra = random.next_int_bounded((-7).max(-n) + 8);
        let (ux, uz) = (b.x0 + (b.x1 - b.x0 + 1) / 2, b.z0 + (b.z1 - b.z0 + 1) / 2);
        for x in ux - SPREAD_RADIUS..=ux + SPREAD_RADIUS {
            for z in uz - SPREAD_RADIUS..=uz + SPREAD_RADIUS {
                let d = (ux - x).abs() + (uz - z).abs() + extra;
                if d >= SPREAD_RADIUS || random.next_float() >= SPREAD[d.max(0) as usize] {
                    continue;
                }
                let height = self.top(x, z)? - 1;
                let y = if buried { height.min(b.y0) } else { height };
                if (b.y0 - y).abs() < 4 {
                    let block = self.nether_block(random);
                    self.plan.set(x, y, z, block);
                    self.nether_column(random, (x, y - 1, z));
                }
            }
        }
        Ok(())
    }

    fn supports(&self, block: BlockId) -> bool {
        !self.plan.is_leaves(block) && (self.plan.is_water(block) && !self.sea_floor || self.plan.is_solid(block))
    }

    fn underwater(&self, b: &BoundingBox) -> Probed<bool> {
        for x in [b.x0, b.x1] {
            for y in [b.y0, b.y1] {
                for z in [b.z0, b.z1] {
                    if self.plan.is_water(self.plan.get(x, y, z)?) {
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }

    fn overworld_y(&self, random: &mut MersenneTwister, b: &BoundingBox, surface: bool) -> Probed<i32> {
        let middle = |a: i32, b: i32| a + (b - a + 1) / 2;
        let height = self.top(middle(b.x0, b.x1), middle(b.z0, b.z1))?;
        if surface {
            return Ok(if self.desert { b.y0 + random.next_int_bounded(6) + (height - b.y1) } else { height - 1 });
        }
        let high = HIGH.contains(&self.probe.biome(middle(b.x0, b.x1), middle(b.y0, b.y1), middle(b.z0, b.z1)));
        let base = if high {
            self.probe.sea_level() + HIGH_SINK_ABOVE_SEA
        } else {
            self.probe.definition().min_y + SUNK_FLOOR
        };
        let y = height - 2 - (b.y1 - b.y0);
        Ok(if y > base { base + random.next_int_bounded(y - base + 1) } else { y })
    }
}

pub fn place(plan: &mut Plan, library: &Library, probe: &dyn StartProbe, random: &mut MersenneTwister, start: (i32, i32), site: &Site) -> Probed<()> {
    let biome = site.centre_biome;
    let (desert, sea_floor) = (DESERT.contains(&biome), OCEAN.contains(&biome) || SWAMP.contains(&biome));
    let cold = ICE.contains(&biome);
    let surface = random.next_float() >= SURFACE_CHANCE;
    let rotation = Rotation::from_index(random.next_int_bounded(4));
    let mirror = random.next_float() >= SURFACE_CHANCE;
    let (x0, z0) = (start.0 * 16, start.1 * 16);
    let mut context = Context {
        plan,
        library,
        probe,
        start,
        desert,
        sea_floor,
    };
    let h0 = context.top(x0, z0)?;
    let name = if random.next_float() >= GIANT_CHANCE {
        format!("portal_{}", random.next_int_bounded(10) + 1)
    } else {
        format!("giant_portal_{}", random.next_int_bounded(3) + 1)
    };
    let Some(template) = library.templates.get(&format!("ruined_portal/{name}")) else { return Ok(()) };
    let (sx, sy, sz) = template.size;
    let (dx, _, dz) = rotation.apply((sx, 0, if mirror { -sz } else { sz }));
    let mut b = BoundingBox::new(x0.min(x0 + dx), h0 - 1, z0.min(z0 + dz), x0.max(x0 + dx), h0 - 1 + sy, z0.max(z0 + dz));
    let y = context.overworld_y(random, &b, surface)?;
    b = b.shifted_y(y - b.y0);
    while context.probe.definition().min_y + SUNK_FLOOR < b.y0 {
        let corners = [(b.x0, b.z0), (b.x0, b.z1), (b.x1, b.z0), (b.x1, b.z1)];
        let mut held = 0;
        for &(x, z) in &corners {
            if context.supports(context.plan.world(context.probe, x, b.y0 - 1, z)?) {
                held += 1;
            }
        }
        if held >= SUPPORT_CORNERS {
            break;
        }
        b = b.shifted_y(-1);
    }
    context.spread_netherrack(random, &b, !surface || desert)?;
    let ignore_air = surface && (desert || random.next_float() >= BURIED_AIR_CHANCE);
    let flood = context.underwater(&b)?;
    let options = Options {
        mirror,
        ignore_air,
        flood,
        wood: None,
        integrity: None,
    };
    let chests = library.place(context.plan, &template, (x0, b.y0, z0), rotation, options);
    random.skip(chests);
    let netherrack = library.palette.named("netherrack", "", false);
    for x in b.x0 + 1..b.x1 {
        for z in b.z0 + 1..b.z1 {
            if context.plan.get(x, b.y0, z)? == netherrack {
                context.nether_column(random, (x, b.y0 - 1, z));
            }
        }
    }
    let mossiness = match () {
        _ if flood => 0.8,
        _ if desert => 0.0,
        _ if JUNGLE.contains(&biome) => 0.8,
        _ if SWAMP.contains(&biome) => 0.5,
        _ => 0.2,
    };
    portal_age::age(context.plan, library, probe, random, &b, portal_age::Weathering { mossiness, cold, sea_floor })
}

pub struct RuinedPortalBuilder;

impl StructureBuilder for RuinedPortalBuilder {
    fn is_start(&self, _: &StructureContext, _: StructureKind, _: (i32, i32)) -> Probed<bool> {
        Ok(true)
    }

    fn regions(&self, context: &StructureContext, blocks: &Blocks, kind: StructureKind, start: (i32, i32)) -> Probed<Vec<Region>> {
        let Some(plan) = context.structures.plan(context.probe, blocks, kind, start)? else {
            return Ok(Vec::new());
        };
        let bounds = plan.writes.iter().map(|&(x, y, z, _)| BoundingBox::new(x, y, z, x, y, z)).reduce(|all, cell| all.union(&cell));
        Ok(bounds
            .map(|bounds| {
                vec![Region {
                    bounds,
                    template: true,
                    settled_walls: false,
                    keep_template_post: false,
                }]
            })
            .unwrap_or_default())
    }

    fn build(&self, context: &StructureContext, kind: StructureKind, chunk: (i32, i32)) -> Probed<Option<Built>> {
        let Some(blocks) = context.structures.table.get() else { return Ok(None) };
        Ok(self.plan(context, blocks, kind, chunk)?.map(|plan| plan as Built))
    }

    fn plan(&self, context: &StructureContext, blocks: &Blocks, _: StructureKind, start: (i32, i32)) -> Probed<Option<Arc<PlanWrites>>> {
        let probe = context.probe;
        context.structures.build_plan(probe, blocks, start, |plan, library, random| {
            let site = Site {
                centre_biome: Structures::centre_biome(probe, start),
            };
            place(plan, library, probe, random, start, &site)
        })
    }

    fn place(&self, context: &StructureContext, blocks: &Blocks, reaching: &Reaching, chunk: &mut ProtoChunk) -> Probed<()> {
        for &(kind, start) in reaching.starts.iter().filter(|&&(kind, _)| kind == StructureKind::RuinedPortal) {
            place_plan(context, blocks, kind, start, chunk)?;
        }
        Ok(())
    }
}
