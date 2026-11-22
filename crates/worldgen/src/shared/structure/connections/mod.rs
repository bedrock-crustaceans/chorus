mod family;
mod rules;

pub use family::Connections;

use super::builder::Region;
use super::plan::ColumnHeights;
use super::{BoundingBox, Probed, Reaching, StartProbe, StructureBuilder, StructureContext, Structures};
use crate::shared::CHUNK_WIDTH;
use crate::shared::blocks::{BlockId, Blocks};
use crate::shared::chunk::proto::ProtoChunk;
use chorus_level::dimension_definition::DimensionDefinition;
use family::Kind;
use rules::{POST_BIT, Surroundings};
use std::sync::Arc;

struct WithoutFeatures<'a>(&'a dyn StartProbe);

impl StartProbe for WithoutFeatures<'_> {
    fn preliminary_surface(&self, chunk: (i32, i32)) -> i32 {
        self.0.preliminary_surface(chunk)
    }

    fn surface_at_quart(&self, quart: (i32, i32)) -> i32 {
        self.0.surface_at_quart(quart)
    }

    fn biome(&self, x: i32, y: i32, z: i32) -> i32 {
        self.0.biome(x, y, z)
    }

    fn sea_level(&self) -> i32 {
        self.0.sea_level()
    }

    fn definition(&self) -> DimensionDefinition {
        self.0.definition()
    }

    fn lowest_column_top(&self, chunk: (i32, i32)) -> Probed<i32> {
        self.0.lowest_column_top(chunk)
    }

    fn chunk(&self, chunk_x: i32, chunk_z: i32) -> Probed<Option<ProtoChunk>> {
        self.0.chunk(chunk_x, chunk_z)
    }

    fn column_heights(&self, chunk: (i32, i32)) -> Probed<std::sync::Arc<ColumnHeights>> {
        self.0.column_heights(chunk)
    }

    fn supplied_heights(&self, chunk: (i32, i32)) -> Option<std::sync::Arc<ColumnHeights>> {
        self.0.supplied_heights(chunk)
    }

    fn starts_at(&self, cell: (i32, i32)) -> Option<std::sync::Arc<super::Starts>> {
        self.0.starts_at(cell)
    }

    fn remember_starts(&self, cell: (i32, i32), starts: std::sync::Arc<super::Starts>) {
        self.0.remember_starts(cell, starts)
    }
}

fn later_neighbours((local_x, local_z): (i32, i32)) -> Vec<(i32, i32)> {
    let mut found = Vec::new();
    if local_x == 0 {
        found.push((-1, 0));
    }
    if local_z == CHUNK_WIDTH - 1 {
        found.push((0, 1));
    }
    if local_x == 0 && local_z == CHUNK_WIDTH - 1 {
        found.push((-1, 1));
    }
    found
}

fn grown(bounds: &BoundingBox) -> BoundingBox {
    BoundingBox::new(bounds.x0 - 1, bounds.y0, bounds.z0 - 1, bounds.x1 + 1, bounds.y1, bounds.z1 + 1)
}

impl Structures {
    fn connection_regions(&self, probe: &dyn StartProbe, blocks: &Blocks, reaching: &Reaching) -> Probed<Vec<Region>> {
        let context = StructureContext { structures: self, probe };
        let mut regions = Vec::new();
        for &(kind, start) in &reaching.starts {
            if let Some(index) = self.builder_index(kind) {
                regions.extend(self.builders[index].regions(&context, blocks, kind, start)?);
            }
        }
        Ok(regions)
    }

    fn border_has_connector(&self, chunk: &ProtoChunk, side: usize) -> bool {
        let (dx, dz) = family::SIDES[side];
        let x_range = if dx == 0 {
            0..CHUNK_WIDTH
        } else if dx < 0 {
            0..1
        } else {
            CHUNK_WIDTH - 1..CHUNK_WIDTH
        };
        let z_range = if dz == 0 {
            0..CHUNK_WIDTH
        } else if dz < 0 {
            0..1
        } else {
            CHUNK_WIDTH - 1..CHUNK_WIDTH
        };
        for x in x_range {
            for z in z_range.clone() {
                for y in chunk.min_y..=chunk.max_y() {
                    if self.blocks.connections.member(chunk.get(x, y, z)).is_some() {
                        return true;
                    }
                }
            }
        }
        false
    }
    fn place_neighbour(&self, probe: &dyn StartProbe, blocks: &Blocks, reaching: &Reaching, chunk: &mut ProtoChunk) -> Probed<()> {
        let context = StructureContext { structures: self, probe };
        let mut ranked: Vec<(u8, &Arc<dyn StructureBuilder>)> = self.builders.iter().filter_map(|builder| Some((builder.neighbour_rank()?, builder))).collect();
        ranked.sort_by_key(|&(rank, _)| rank);
        for (_, builder) in ranked {
            builder.place(&context, blocks, reaching, chunk)?;
        }
        Ok(())
    }

    fn load_neighbours(&self, probe: &dyn StartProbe, blocks: &Blocks, chunk: &ProtoChunk) -> Probed<[Option<ProtoChunk>; 4]> {
        let (chunk_x, chunk_z) = (chunk.min_x().div_euclid(CHUNK_WIDTH), chunk.min_z().div_euclid(CHUNK_WIDTH));
        let mut beside: [Option<ProtoChunk>; 4] = [None, None, None, None];
        for (side, slot) in beside.iter_mut().enumerate() {
            if !self.border_has_connector(chunk, side) {
                continue;
            }
            let (dx, dz) = family::SIDES[side];
            let mut scratch = match probe.chunk(chunk_x + dx, chunk_z + dz)? {
                Some(supplied) => supplied,
                None => chunk.empty_beside(dx, dz),
            };
            if let Some(next) = self.reaching(probe, (chunk_x + dx, chunk_z + dz))? {
                self.place_neighbour(&WithoutFeatures(probe), blocks, &next, &mut scratch)?;
            }
            *slot = Some(scratch);
        }
        Ok(beside)
    }

    pub(super) fn reconnect(&self, probe: &dyn StartProbe, blocks: &Blocks, reaching: &Reaching, chunk: &mut ProtoChunk) -> Probed<()> {
        let regions = self.connection_regions(probe, blocks, reaching)?;
        if regions.is_empty() {
            return Ok(());
        }
        let beside = self.load_neighbours(probe, blocks, chunk)?;
        let surroundings = Surroundings {
            chunk: &*chunk,
            ids: &self.blocks,
            blocks,
            connections: &self.blocks.connections,
            palette: self.blocks.palette.as_deref(),
            beside: &beside,
            world: &|x, y, z| probe.staged_block(x, y, z),
        };
        let (min_x, min_z) = (chunk.min_x(), chunk.min_z());
        let mut updates: Vec<(i32, i32, i32, BlockId)> = Vec::new();
        let mut settled_walls: Vec<(i32, i32, i32)> = Vec::new();
        for region in &regions {
            let bounds = region.bounds;
            for x in bounds.x0.max(min_x)..=bounds.x1.min(min_x + CHUNK_WIDTH - 1) {
                for z in bounds.z0.max(min_z)..=bounds.z1.min(min_z + CHUNK_WIDTH - 1) {
                    for y in bounds.y0.max(chunk.min_y)..=bounds.y1.min(chunk.max_y()) {
                        let block = chunk.get(x - min_x, y, z - min_z);
                        let Some(connector) = surroundings.connections.member(block) else { continue };
                        let code = match connector.kind {
                            Kind::Fence | Kind::NetherFence | Kind::Pane => surroundings.side_code(connector.kind, (x, y, z))?,
                            Kind::Stairs => surroundings.corner_code((x, y, z), connector)?,
                            Kind::Wall if region.settled_walls => {
                                settled_walls.push((x, y, z));
                                continue;
                            }
                            Kind::Wall if region.template => {
                                let code = surroundings.wall_code((x, y, z))?;
                                if region.keep_template_post { (code & !POST_BIT) | (connector.code & POST_BIT) } else { code }
                            }
                            _ => continue,
                        };
                        if code == connector.code {
                            continue;
                        }
                        let Some(joined) = surroundings.connections.recoded(connector, code) else { continue };
                        updates.push((x - min_x, y, z - min_z, joined));
                    }
                }
            }
        }
        settled_walls.sort_unstable_by_key(|&(x, y, z)| (y, x, z));
        settled_walls.dedup();
        let alone = Surroundings {
            beside: &[None, None, None, None],
            ..surroundings
        };
        let rejoined = |(x, z): (i32, i32)| {
            later_neighbours((x - min_x, z - min_z)).into_iter().any(|(dx, dz)| {
                let neighbour = BoundingBox::chunk(min_x.div_euclid(CHUNK_WIDTH) + dx, min_z.div_euclid(CHUNK_WIDTH) + dz, chunk.definition());
                regions
                    .iter()
                    .any(|region| region.settled_walls && region.bounds.intersects(&neighbour) && grown(&region.bounds).contains(x, region.bounds.y0, z))
            })
        };
        let mut first_pass: std::collections::HashMap<(i32, i32, i32), u16> = std::collections::HashMap::new();
        for &(x, y, z) in &settled_walls {
            let code = if rejoined((x, z)) { alone.wall_code((x, y, z))? } else { surroundings.wall_code((x, y, z))? };
            first_pass.insert((x, y, z), code);
        }
        for &(x, y, z) in &settled_walls {
            let Some(connector) = surroundings.connections.member(chunk.get(x - min_x, y, z - min_z)) else {
                continue;
            };
            let code = if rejoined((x, z)) {
                surroundings.wall_code_settled((x, y, z), first_pass.get(&(x, y + 1, z)).copied())?
            } else {
                first_pass[&(x, y, z)]
            };
            if code != connector.code
                && let Some(joined) = surroundings.connections.recoded(connector, code)
            {
                updates.push((x - min_x, y, z - min_z, joined));
            }
        }
        for (x, y, z, block) in updates {
            chunk.set(x, y, z, block);
        }
        Ok(())
    }
}
