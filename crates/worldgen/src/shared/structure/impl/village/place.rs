use crate::shared::blocks::{AIR, BlockId, BlockTable, Blocks};
use crate::shared::chunk::proto::ProtoChunk;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::layout::{Connector, EntryKind, Grounding, Palette, Part, PoolCatalogue, PoolEntry, Sample, middle, process};
use crate::shared::structure::palette::PaletteBlocks;
use crate::shared::structure::plan::Plan;
use crate::shared::structure::template::{StateKind, TemplateLibrary};
use crate::shared::structure::{BoundingBox, CHESTS, Probed, StartProbe, StructureBlocks, is_falling_block};
use crate::shared::tags::Tag;
use crate::shared::{CHUNK_MASK, CHUNK_WIDTH};
use chorus_block::block_id::{GRASS_BLOCK, RED_SAND, RED_SANDSTONE, SAND, SANDSTONE};

const NAMESPACE: &str = "minecraft:";
const NAMESPACE_LENGTH: usize = NAMESPACE.len();
const VIEW_RADIUS: i32 = 1;
const FILL_BELOW_FLAG: u8 = 1;
const PLUG_HOLES_FLAG: u8 = 2;
const FILL_DEPTH_CHOICES: i32 = 3;
const FILL_MIN_DEPTH: i32 = 3;
const PLUG_RADIUS: i32 = 12;

pub struct VillageBlocks {
    pub sand: BlockId,
    pub red_sand: BlockId,
    pub sandstone: BlockId,
    pub red_sandstone: BlockId,
    pub grass_block: BlockId,
}

impl Default for VillageBlocks {
    fn default() -> Self {
        Self {
            sand: AIR,
            red_sand: AIR,
            sandstone: AIR,
            red_sandstone: AIR,
            grass_block: AIR,
        }
    }
}

impl VillageBlocks {
    pub fn register(table: &mut BlockTable) -> Self {
        Self {
            sand: table.name(SAND),
            red_sand: table.name(RED_SAND),
            sandstone: table.name(SANDSTONE),
            red_sandstone: table.name(RED_SANDSTONE),
            grass_block: table.name(GRASS_BLOCK),
        }
    }
}

pub struct Surface<'a> {
    pub probe: &'a dyn StartProbe,
    pub chunk: &'a mut ProtoChunk,
    pub blocks: &'a Blocks,
    pub ids: &'a StructureBlocks,
    pub palette: &'a PaletteBlocks,
    pub library: &'a TemplateLibrary,
    pub data: &'a PoolCatalogue,
    pub beyond: &'a Plan<'a>,
    pub bounds: BoundingBox,
    pub tops: &'a dyn Fn(i32, i32) -> i32,
    pub top_material: &'a dyn Fn(i32, i32, i32) -> BlockId,
    pub rejects_floating: bool,
}

fn short_name(blocks: &Blocks, id: BlockId) -> &str {
    &blocks.name_of(id)[NAMESPACE_LENGTH..]
}

impl Surface<'_> {
    fn in_world(&self, y: i32) -> bool {
        (self.chunk.min_y..=self.chunk.max_y()).contains(&y)
    }

    fn inside(&self, x: i32, z: i32) -> bool {
        (self.bounds.x0..=self.bounds.x1).contains(&x) && (self.bounds.z0..=self.bounds.z1).contains(&z)
    }

    fn in_view(&self, x: i32, z: i32) -> bool {
        let centre = (self.bounds.x0.div_euclid(CHUNK_WIDTH), self.bounds.z0.div_euclid(CHUNK_WIDTH));
        (x.div_euclid(CHUNK_WIDTH) - centre.0).abs() <= VIEW_RADIUS && (z.div_euclid(CHUNK_WIDTH) - centre.1).abs() <= VIEW_RADIUS
    }

    fn get(&self, x: i32, y: i32, z: i32) -> Probed<BlockId> {
        if !self.in_world(y) {
            Ok(AIR)
        } else if self.inside(x, z) {
            Ok(self.chunk.get(x & CHUNK_MASK, y, z & CHUNK_MASK))
        } else if self.in_view(x, z) {
            self.beyond.staged(self.probe, x, y, z)
        } else {
            Ok(AIR)
        }
    }

    fn set(&mut self, x: i32, y: i32, z: i32, block: BlockId) {
        if !self.in_world(y) || !self.inside(x, z) {
            return;
        }
        let (local_x, local_z) = (x & CHUNK_MASK, z & CHUNK_MASK);
        let block = if self.chunk.get(local_x, y, local_z) == self.ids.water {
            self.blocks.flooded(block)
        } else {
            block
        };
        self.chunk.set(local_x, y, local_z, block);
    }

    fn falls(&self, block: BlockId) -> bool {
        let name = short_name(self.blocks, block);
        is_falling_block(name)
    }

    fn world_subject(&self, block: BlockId) -> Sample<'_> {
        Sample {
            name: short_name(self.blocks, block),
            default_state: true,
        }
    }

    fn rule_palette(&self) -> Palette<'_> {
        Palette {
            entries: self.library.entries(),
            default_states: &self.data.default_states,
        }
    }

    fn jigsaw_block(&self, processors: &[crate::shared::structure::layout::Processor], layout: &Connector, world: BlockId, seed: u32) -> BlockId {
        if processors.is_empty() {
            return self.palette.block(layout.final_state);
        }
        let index = process(
            processors,
            layout.final_state,
            &mut MersenneTwister::new(seed),
            &self.rule_palette(),
            &self.world_subject(world),
            (0, 0, 0),
        );
        self.palette.block(index)
    }

    fn place_feature(&mut self, piece: &Part, name: &str, random: &mut MersenneTwister) -> Probed<()> {
        let id = format!("{NAMESPACE}{name}");
        self.probe.run_feature(self.chunk, &id, piece.origin, random)?;
        self.plug_holes(&piece.bounds)
    }

    pub fn place(&mut self, piece: &Part, seed: u32, random: &mut MersenneTwister) -> Probed<()> {
        let element = &self.data.entries[piece.entry];
        if let EntryKind::Feature(name) = &element.kind {
            return self.place_feature(piece, name, random);
        }
        let EntryKind::Template {
            template,
            processors,
            grounding,
            foundation,
            ..
        } = &element.kind
        else {
            return Ok(());
        };
        if *grounding == Grounding::FollowsTerrain {
            return self.place_matching(piece, element, seed);
        }
        if self.rejects_floating && !self.can_place(&piece.bounds)? {
            return Ok(());
        }
        for layout in element.jigsaws(self.library, piece.origin, piece.rotation) {
            let (x, y, z) = layout.pos;
            if self.inside(x, z) && self.in_world(y) {
                let block = self.jigsaw_block(processors, &layout, self.get(x, y, z)?, seed);
                self.set(x, y, z, block);
            }
        }
        let mut rule_random = (!processors.is_empty()).then(|| MersenneTwister::new(seed));
        for (index, block) in template.blocks.iter().enumerate() {
            let kind = self.library.kind(block.state);
            if kind == StateKind::Jigsaw {
                continue;
            }
            let (dx, dy, dz) = piece.rotation.apply((block.x as i32, block.y as i32, block.z as i32));
            let at = (piece.origin.0 + dx, piece.origin.1 + dy, piece.origin.2 + dz);
            if !self.inside(at.0, at.2) || !self.in_world(at.1) {
                continue;
            }
            let mut palette_index = self.library.rotated(block.state, piece.rotation as usize);
            if let Some(rule_random) = rule_random.as_mut() {
                let world = self.world_subject(self.get(at.0, at.1, at.2)?);
                palette_index = process(processors, palette_index, rule_random, &self.rule_palette(), &world, (0, 0, 0));
            }
            let id = self.palette.block(palette_index);
            self.set(at.0, at.1, at.2, id);
            if template.loot_chests.contains(&index) && CHESTS.contains(&short_name(self.blocks, id)) {
                random.next_u32();
            }
        }
        if foundation & FILL_BELOW_FLAG != 0 {
            self.fill_bottom(&piece.bounds, random)?;
        }
        if foundation & PLUG_HOLES_FLAG != 0 {
            self.plug_holes(&piece.bounds)?;
        }
        Ok(())
    }

    fn place_matching(&mut self, piece: &Part, element: &PoolEntry, seed: u32) -> Probed<()> {
        let EntryKind::Template { template, processors, .. } = &element.kind else { return Ok(()) };
        let base = piece.origin.1;
        let ground = |this: &Self, x: i32, z: i32, y: i32| (this.tops)(x, z) + y - base - 1;
        for layout in element.jigsaws(self.library, piece.origin, piece.rotation) {
            let (x, y, z) = layout.pos;
            if self.inside(x, z) {
                let at = ground(self, x, z, y);
                let block = self.jigsaw_block(processors, &layout, self.get(x, at, z)?, seed);
                self.set(x, at, z, block);
            }
        }
        let mut rule_random = (!processors.is_empty()).then(|| MersenneTwister::new(seed));
        for block in &template.blocks {
            let kind = self.library.kind(block.state);
            if kind != StateKind::Block {
                continue;
            }
            let (dx, dy, dz) = piece.rotation.apply((block.x as i32, block.y as i32, block.z as i32));
            let (x, y, z) = (piece.origin.0 + dx, piece.origin.1 + dy, piece.origin.2 + dz);
            if !self.inside(x, z) {
                continue;
            }
            let at = ground(self, x, z, y);
            let mut palette_index = self.library.rotated(block.state, piece.rotation as usize);
            if let Some(rule_random) = rule_random.as_mut() {
                let world = self.world_subject(self.get(x, at, z)?);
                palette_index = process(processors, palette_index, rule_random, &self.rule_palette(), &world, (0, 0, 0));
            }
            let id = self.palette.block(palette_index);
            self.set(x, at, z, id);
        }
        Ok(())
    }

    fn can_place(&self, bounds: &BoundingBox) -> Probed<bool> {
        if bounds.x0 >= bounds.x1 {
            return Ok(true);
        }
        let y = bounds.y0 - 1;
        let points = [
            (middle(bounds.x0, bounds.x1), y - 1, middle(bounds.z0, bounds.z1)),
            (bounds.x0, y, bounds.z0),
            (bounds.x0, y, bounds.z1),
            (bounds.x1, y, bounds.z1),
            (bounds.x1, y, bounds.z0),
        ];
        for &(x, y, z) in &points {
            if self.get(x, y, z)? != AIR {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn fill_for(&self, block: BlockId, (x, y, z): (i32, i32, i32)) -> Option<BlockId> {
        if self.blocks.is(block, Tag::Leaves) {
            return None;
        }
        let top = (self.top_material)(x, y, z);
        Some(if top == self.ids.village.grass_block { self.ids.dirt } else { top })
    }

    fn fill_bottom(&mut self, bounds: &BoundingBox, random: &mut MersenneTwister) -> Probed<()> {
        let y0 = bounds.y0;
        for x in bounds.x0..=bounds.x1 {
            for z in bounds.z0..=bounds.z1 {
                let top = self.get(x, y0, z)?;
                if top == AIR {
                    continue;
                }
                let Some(fill) = self.fill_for(top, (x, y0, z)) else { continue };
                let depth = random.next_int_bounded(FILL_DEPTH_CHOICES);
                let mut y = y0 - 1;
                while y > y0 - depth - FILL_MIN_DEPTH {
                    let below = self.get(x, y, z)?;
                    if (below != AIR && !self.chunk.is_fluid(below)) || !self.inside(x, z) || !self.in_world(y) {
                        break;
                    }
                    self.set(x, y, z, fill);
                    y -= 1;
                }
            }
        }
        Ok(())
    }

    fn plug_holes(&mut self, bounds: &BoundingBox) -> Probed<()> {
        let (middle_x, middle_z, y) = (middle(bounds.x0, bounds.x1), middle(bounds.z0, bounds.z1), bounds.y0 - 1);
        for x in middle_x - PLUG_RADIUS..=middle_x + PLUG_RADIUS {
            for z in middle_z - PLUG_RADIUS..=middle_z + PLUG_RADIUS {
                if !self.inside(x, z) {
                    continue;
                }
                let top = self.get(x, y, z)?;
                if !self.falls(top) {
                    continue;
                }
                let plug = self.plug_for(top);
                let mut below = y - 1;
                while below > self.chunk.min_y {
                    let block = self.get(x, below, z)?;
                    if !self.falls(block) {
                        if block == AIR || block == self.ids.water {
                            self.set(x, below, z, plug);
                        }
                        break;
                    }
                    below -= 1;
                }
            }
        }
        Ok(())
    }

    fn plug_for(&self, block: BlockId) -> BlockId {
        let village = &self.ids.village;
        if block == village.red_sand {
            village.red_sandstone
        } else if block == village.sand {
            village.sandstone
        } else {
            self.ids.stone
        }
    }
}
