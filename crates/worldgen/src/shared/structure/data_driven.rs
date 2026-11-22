use std::sync::{Arc, OnceLock};

use serde_json::Value;

use super::builder::{Built, Region, StructureBuilder, StructureContext, covers, erase};
use super::entities::{entities_at, report_structure_entities};
use super::feature::FeatureSite;
use super::layout::{Capped, EntryKind, Palette, Part, PoolCatalogue, Processor, Sample, StartHeight, StartSpec, VariantSet, forked_at, process_first, shuffle, start_random, wide_position_seed};
use super::layout::{bare, identifier};
use super::palette::PaletteBlocks;
use super::reaching::V2_REACH;
use super::template::{StateKind, Template, TemplateLibrary};
use super::{Adjustments, BoundingBox, Probed, Reaching, StartProbe, StructureKind, Structures};
use crate::shared::blocks::{BlockId, Blocks};
use crate::shared::chunk::proto::ProtoChunk;
use crate::shared::rng::random::RandomSource;
use crate::shared::{CHUNK_MASK, CHUNK_WIDTH};

const POOL_CATALOGUE: &str = "v2";
const NAMESPACE_LENGTH: usize = "minecraft:".len();
const STRUCTURE_VOID: &str = "structure_void";
const AIR: &str = "air";

pub(super) struct OwnedSpec {
    pool: String,
    max_depth: i32,
    horizontal: i32,
    vertical: i32,
    height: StartHeight,
    project: bool,
    padding: i32,
    aliases: Option<String>,
}

fn absolute(value: &Value) -> Option<i32> {
    value["absolute"].as_i64().map(|level| level as i32)
}

impl OwnedSpec {
    fn from_document(document: &Value) -> Option<Self> {
        let distance = &document["max_distance_from_center"];
        let (horizontal, vertical) = match distance.as_i64() {
            Some(reach) => (reach as i32, reach as i32),
            None => (distance["horizontal"].as_i64()? as i32, distance["vertical"].as_i64()? as i32),
        };
        let start = &document["start_height"];
        let height = match start["type"].as_str()? {
            "constant" => StartHeight::Constant(absolute(&start["value"])?),
            "uniform" => StartHeight::Uniform(absolute(&start["min_inclusive"])?, absolute(&start["max_inclusive"])?),
            _ => return None,
        };
        Some(Self {
            pool: bare(document["start_pool"].as_str()?).to_owned(),
            max_depth: document["max_depth"].as_i64()? as i32,
            horizontal,
            vertical,
            height,
            project: document["heightmap_projection"].as_str().is_some_and(|projection| projection != "none") || document["project_start_to_heightmap"].as_bool().unwrap_or(false),
            padding: document["dimension_padding"].as_i64().unwrap_or(0) as i32,
            aliases: document.get("pool_aliases").map(|_| bare(identifier(document)).to_owned()),
        })
    }

    fn spec(&self) -> StartSpec<'_> {
        StartSpec {
            pool: &self.pool,
            max_depth: self.max_depth,
            horizontal: self.horizontal,
            vertical: self.vertical,
            height: match self.height {
                StartHeight::Constant(level) => StartHeight::Constant(level),
                StartHeight::Uniform(low, high) => StartHeight::Uniform(low, high),
            },
            project: self.project,
            padding: self.padding,
            aliases: self.aliases.as_deref(),
        }
    }
}

pub struct V2Structure {
    pub pieces: Vec<Part>,
}

struct Cell {
    at: (i32, i32, i32),
    index: u16,
    id: BlockId,
}

fn variant_spec(variant: &VariantSet) -> StartSpec<'_> {
    StartSpec {
        pool: &variant.pool,
        max_depth: variant.max_depth,
        horizontal: variant.horizontal,
        vertical: variant.horizontal,
        height: StartHeight::Constant(0),
        project: true,
        padding: variant.padding,
        aliases: None,
    }
}

impl Structures {
    pub fn v2_layout(&self) -> Option<&PoolCatalogue> {
        self.data.catalogue(POOL_CATALOGUE)
    }

    fn v2_biome(&self, probe: &dyn StartProbe, catalogue: &PoolCatalogue, spec: &StartSpec, chunk: (i32, i32)) -> Probed<i32> {
        let height = |x: i32, z: i32| self.raw_height(probe, x, z);
        let (_, (x, y, z), _) = spec.start_part(catalogue, &mut start_random(self.seed, chunk), chunk, &height)?;
        Ok(probe.biome(x, y, z))
    }

    fn picked_variant<'c>(&self, probe: &dyn StartProbe, catalogue: &'c PoolCatalogue, set: &str, chunk: (i32, i32)) -> Probed<Option<&'c VariantSet>> {
        let Some(listed) = catalogue.variants.get(set) else { return Ok(None) };
        let mut variants: Vec<&VariantSet> = listed.iter().collect();
        let mut total: i32 = variants.iter().map(|variant| variant.weight).sum();
        let mut random = start_random(self.seed, chunk);
        while !variants.is_empty() {
            let mut left = random.next_int_bounded(total);
            let picked = variants
                .iter()
                .position(|variant| {
                    left -= variant.weight;
                    left < 0
                })
                .unwrap_or(variants.len() - 1);
            let variant = variants[picked];
            if variant.biomes.contains(&self.v2_biome(probe, catalogue, &variant_spec(variant), chunk)?) {
                return Ok(Some(variant));
            }
            total -= variant.weight;
            variants.remove(picked);
        }
        Ok(None)
    }

    pub fn v2_structure(&self, probe: &dyn StartProbe, kind: StructureKind, start: (i32, i32)) -> Probed<Option<Arc<V2Structure>>> {
        Ok(self.starts_at(probe, start)?.get::<V2Structure>(kind))
    }

    pub(super) fn place_v2(&self, probe: &dyn StartProbe, blocks: &Blocks, starts: &[(StructureKind, (i32, i32))], chunk: &mut ProtoChunk) -> Probed<()> {
        let (Some(palette), Some(library), Some(catalogue)) = (self.blocks.palette.as_ref(), self.data.library.as_ref(), self.v2_layout()) else {
            return Ok(());
        };
        let (chunk_x, chunk_z) = (chunk.min_x().div_euclid(CHUNK_WIDTH), chunk.min_z().div_euclid(CHUNK_WIDTH));
        let bounds = BoundingBox::chunk(chunk_x, chunk_z, chunk.definition());
        let rule_palette = Palette {
            entries: library.entries(),
            default_states: &catalogue.default_states,
        };
        for &(kind, start) in starts {
            let Some(structure) = self.v2_structure(probe, kind, start)? else { continue };
            for part in structure.pieces.iter().filter(|part| part.bounds.intersects(&bounds)) {
                let EntryKind::Template { template, processors, capped, .. } = &catalogue.entries[part.entry].kind else {
                    continue;
                };
                let entities = entities_at(template, part.origin, part.rotation, false);
                report_structure_entities(self.registry.name(kind), &entities, |x, z| bounds.contains(x.floor() as i32, bounds.y0, z.floor() as i32));
                let clip = capped.is_empty().then_some(&bounds);
                let mut cells = self.template_cells(palette, library, template, part, clip, chunk, &rule_palette, processors, blocks);
                if !capped.is_empty() {
                    self.apply_capped(palette, capped, &mut cells, part.origin, chunk, &rule_palette, blocks);
                }
                for cell in cells.iter().filter(|cell| bounds.contains(cell.at.0, cell.at.1, cell.at.2)) {
                    let (local_x, local_z) = (cell.at.0 & CHUNK_MASK, cell.at.2 & CHUNK_MASK);
                    let id = if chunk.get(local_x, cell.at.1, local_z) == self.blocks.water {
                        blocks.flooded(cell.id)
                    } else {
                        cell.id
                    };
                    chunk.set(local_x, cell.at.1, local_z, id);
                }
            }
        }
        Ok(())
    }

    fn template_cells(
        &self,
        palette: &PaletteBlocks,
        library: &TemplateLibrary,
        template: &Template,
        part: &Part,
        clip: Option<&BoundingBox>,
        chunk: &ProtoChunk,
        rule_palette: &Palette,
        processors: &[Processor],
        blocks: &Blocks,
    ) -> Vec<Cell> {
        let rotation = part.rotation as usize;
        let mut cells = Vec::with_capacity(template.blocks.len());
        for (index, block) in template.blocks.iter().enumerate() {
            let kind = library.kind(block.state);
            let (dx, dy, dz) = part.rotation.apply((block.x as i32, block.y as i32, block.z as i32));
            let at = (part.origin.0 + dx, part.origin.1 + dy, part.origin.2 + dz);
            if clip.is_some_and(|clip| !clip.contains(at.0, at.1, at.2)) || !(chunk.min_y..=chunk.max_y()).contains(&at.1) {
                continue;
            }
            let (palette_index, mut id) = match kind {
                StateKind::StructureBlock => continue,
                StateKind::Jigsaw => {
                    let Some(jigsaw) = template.jigsaws.iter().find(|jigsaw| jigsaw.block == index) else { continue };
                    let state = library.rotated(jigsaw.final_state, rotation);
                    (state, palette.block(state))
                }
                StateKind::Block => (library.rotated(block.state, rotation), palette.turned_block(block.state, rotation)),
            };
            if library.entry(palette_index).name == STRUCTURE_VOID {
                continue;
            }
            let mut index = palette_index;
            if !processors.is_empty() {
                let processed = process_first(processors, palette_index, wide_position_seed(at), rule_palette, &Self::world_sample(chunk, blocks, at));
                if processed != palette_index {
                    id = palette.block(processed);
                    index = processed;
                }
            }
            cells.push(Cell { at, index, id });
        }
        cells
    }

    fn world_sample<'b>(chunk: &ProtoChunk, blocks: &'b Blocks, at: (i32, i32, i32)) -> Sample<'b> {
        let (local_x, local_z) = (at.0 - chunk.min_x(), at.2 - chunk.min_z());
        let inside = (0..CHUNK_WIDTH).contains(&local_x) && (0..CHUNK_WIDTH).contains(&local_z);
        let name = if inside { &blocks.name_of(chunk.get(local_x, at.1, local_z))[NAMESPACE_LENGTH..] } else { AIR };
        Sample { name, default_state: true }
    }

    fn apply_capped(&self, palette: &PaletteBlocks, capped: &[Capped], cells: &mut [Cell], origin: (i32, i32, i32), chunk: &ProtoChunk, rule_palette: &Palette, blocks: &Blocks) {
        for cap in capped {
            let mut order: Vec<usize> = (0..cells.len()).collect();
            shuffle(&mut forked_at(self.seed, origin), &mut order);
            let limit = cap.limit.min(cells.len());
            let mut changed = 0;
            for position in order {
                if changed == limit {
                    break;
                }
                let cell = &mut cells[position];
                let output = process_first(&cap.rules, cell.index, wide_position_seed(cell.at), rule_palette, &Self::world_sample(chunk, blocks, cell.at));
                if output != cell.index {
                    cell.index = output;
                    cell.id = palette.block(output);
                    changed += 1;
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
pub enum StartRule {
    Avoid(&'static str),
    Require(&'static str),
    Variants(&'static str),
}

pub struct JigsawStructureBuilder {
    identifier: String,
    kind: OnceLock<StructureKind>,
    rule: StartRule,
    starts_without_data: bool,
    connects: bool,
    spec: OnceLock<Option<OwnedSpec>>,
}

impl JigsawStructureBuilder {
    pub fn new(identifier: impl Into<String>, rule: StartRule) -> Self {
        Self {
            identifier: identifier.into(),
            kind: OnceLock::new(),
            rule,
            starts_without_data: false,
            connects: true,
            spec: OnceLock::new(),
        }
    }

    pub fn starting_without_data(mut self) -> Self {
        self.starts_without_data = true;
        self
    }

    pub fn without_connections(mut self) -> Self {
        self.connects = false;
        self
    }

    pub fn trial_chambers() -> Self {
        Self::built_in(StructureKind::TrialChambers, StartRule::Avoid("deep_dark"))
            .starting_without_data()
            .without_connections()
    }

    pub fn trail_ruins() -> Self {
        Self::built_in(StructureKind::TrailRuins, StartRule::Require("trail_ruins"))
    }

    pub fn abandoned_camp() -> Self {
        Self::built_in(StructureKind::AbandonedCamp, StartRule::Variants("abandoned_camp"))
    }

    fn built_in(kind: StructureKind, rule: StartRule) -> Self {
        let builder = Self::new(format!("{kind:?}"), rule);
        let _ = builder.kind.set(kind);
        builder
    }

    fn kind(&self, structures: &Structures) -> StructureKind {
        *self
            .kind
            .get_or_init(|| structures.registry().registered_kind(&self.identifier).expect("a jigsaw builder is used after it is registered"))
    }

    pub(super) fn spec(&self, structures: &Structures) -> Option<StartSpec<'_>> {
        self.spec
            .get_or_init(|| {
                let wanted = structures.registry().identifier(self.kind(structures));
                let document = structures.data.documents.structures.iter().find(|document| identifier(document) == wanted)?;
                OwnedSpec::from_document(document)
            })
            .as_ref()
            .map(OwnedSpec::spec)
    }

    fn start_allowed(&self, structures: &Structures, probe: &dyn StartProbe, chunk: (i32, i32)) -> Probed<bool> {
        let Some(catalogue) = structures.v2_layout() else { return Ok(self.starts_without_data) };
        match self.rule {
            StartRule::Avoid(tag) => match self.spec(structures) {
                Some(spec) => Ok(!structures.tag(tag).contains(&structures.v2_biome(probe, catalogue, &spec, chunk)?)),
                None => Ok(false),
            },
            StartRule::Require(tag) => match self.spec(structures) {
                Some(spec) => Ok(structures.tag(tag).contains(&structures.v2_biome(probe, catalogue, &spec, chunk)?)),
                None => Ok(false),
            },
            StartRule::Variants(set) => Ok(structures.picked_variant(probe, catalogue, set, chunk)?.is_some()),
        }
    }

    fn assemble(&self, structures: &Structures, probe: &dyn StartProbe, start: (i32, i32)) -> Probed<Option<V2Structure>> {
        let (Some(catalogue), Some(library)) = (structures.v2_layout(), structures.data.library.as_ref()) else {
            return Ok(None);
        };
        let height = |x: i32, z: i32| structures.raw_height(probe, x, z);
        let mut random = start_random(structures.seed, start);
        let pieces = match self.rule {
            StartRule::Variants(set) => match structures.picked_variant(probe, catalogue, set, start)? {
                Some(variant) => variant_spec(variant).assemble(catalogue, library, &mut random, structures.seed, start, probe.definition(), &height)?,
                None => Vec::new(),
            },
            _ => {
                let Some(spec) = self.spec(structures) else { return Ok(None) };
                spec.assemble(catalogue, library, &mut random, structures.seed, start, probe.definition(), &height)?
            }
        };
        Ok(Some(V2Structure { pieces }))
    }
}

impl StructureBuilder for JigsawStructureBuilder {
    fn is_start(&self, context: &StructureContext, _kind: StructureKind, chunk: (i32, i32)) -> Probed<bool> {
        self.start_allowed(context.structures, context.probe, chunk)
    }

    fn build(&self, context: &StructureContext, _kind: StructureKind, chunk: (i32, i32)) -> Probed<Option<Built>> {
        Ok(self.assemble(context.structures, context.probe, chunk)?.map(erase))
    }

    fn reaches(&self, context: &StructureContext, kind: StructureKind, start: (i32, i32), area: &BoundingBox) -> Probed<bool> {
        Ok(context
            .structures
            .v2_structure(context.probe, kind, start)?
            .is_some_and(|structure| covers(structure.pieces.iter().map(|piece| &piece.bounds), area)))
    }

    fn regions(&self, context: &StructureContext, _: &Blocks, kind: StructureKind, start: (i32, i32)) -> Probed<Vec<Region>> {
        if !self.connects {
            return Ok(Vec::new());
        }
        Ok(context
            .structures
            .v2_structure(context.probe, kind, start)?
            .map(|structure| {
                structure
                    .pieces
                    .iter()
                    .map(|piece| Region {
                        bounds: piece.bounds,
                        template: true,
                        settled_walls: false,
                        keep_template_post: true,
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    fn adjust(&self, context: &StructureContext, chunk: (i32, i32), adjustments: &mut Adjustments) -> Probed<()> {
        context.structures.adjust_v2(context.probe, self.kind(context.structures), chunk, adjustments)
    }

    fn contains(&self, context: &StructureContext, chunk: (i32, i32), at: (i32, i32, i32), with_boxes: bool) -> Probed<bool> {
        if !with_boxes {
            return Ok(false);
        }
        context.structures.in_v2_box(context.probe, self.kind(context.structures), chunk, at)
    }

    fn near_window(&self, context: &StructureContext, chunk: (i32, i32), margin: i32) -> Probed<()> {
        let structures = context.structures;
        let kind = self.kind(structures);
        structures.starts_near(context.probe, kind, chunk, structures.registry().reach(kind) + V2_REACH + margin)?;
        Ok(())
    }

    fn feature_sites(&self, context: &StructureContext, chunk: (i32, i32), area: &BoundingBox, sites: &mut Vec<FeatureSite>) -> Probed<()> {
        context.structures.v2_feature_sites(context.probe, self.kind(context.structures), chunk, area, sites)
    }

    fn place(&self, context: &StructureContext, blocks: &Blocks, reaching: &Reaching, chunk: &mut ProtoChunk) -> Probed<()> {
        let own = self.kind(context.structures);
        let starts: Vec<(StructureKind, (i32, i32))> = reaching.starts.iter().copied().filter(|(kind, _)| *kind == own).collect();
        context.structures.place_v2(context.probe, blocks, &starts, chunk)
    }

    fn neighbour_rank(&self) -> Option<u8> {
        Some(3)
    }
}
