use super::super::placed::Placed;
use super::super::world::{biome_id_at, read};
use super::BlockSet;
use super::support::{holds_face, is_open, is_water_source};
use crate::shared::blocks::BlockId;
use crate::shared::chunk::region::Region;
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::Probed;
use crate::shared::survival::Direction;
use glam::IVec3;
use serde_json::Value;
use std::sync::Arc;

const WALL_ORDER: [Direction; 4] = [Direction::North, Direction::West, Direction::South, Direction::East];
const SURFACE_CEILING: &str = "ceiling";
const SURFACE_FLOOR: &str = "floor";
const SURFACE_HORIZONTAL: &str = "random_horizontal";
const SURFACE_VERTICAL: &str = "random_vertical";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    Ceiling,
    Floor,
    RandomHorizontal,
    RandomVertical,
}

impl Surface {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            SURFACE_CEILING => Some(Self::Ceiling),
            SURFACE_FLOOR => Some(Self::Floor),
            SURFACE_HORIZONTAL => Some(Self::RandomHorizontal),
            SURFACE_VERTICAL => Some(Self::RandomVertical),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct Snap {
    pub feature: Arc<Placed>,
    pub range: i32,
    pub surface: Surface,
    pub allow_air: bool,
    pub allow_water: bool,
    pub allow_solid: bool,
    pub embed: bool,
    pub allowed: BlockSet,
}

impl Snap {
    pub fn parse(feature: Arc<Placed>, body: &'static Value, allowed: BlockSet) -> Option<Self> {
        let flag = |key: &str, default: bool| body.get(key).and_then(Value::as_bool).unwrap_or(default);
        Some(Self {
            feature,
            range: body.get("vertical_search_range")?.as_i64()? as i32,
            surface: Surface::parse(body.get("surface")?.as_str()?)?,
            allow_air: flag("allow_air_placement", true),
            allow_water: flag("allow_underwater_placement", false),
            allow_solid: flag("allow_non_air_placement", false),
            embed: flag("embed_in_surface", false),
            allowed,
        })
    }

    pub fn place(&self, region: &mut Region, random: &mut MersenneTwister, pos: IVec3) -> Probed<bool> {
        let snapped = match self.surface {
            Surface::RandomVertical => {
                let mut order = WALL_ORDER;
                for i in 1..order.len() {
                    order.swap(i, (random.next_u32() % (i as u32 + 1)) as usize);
                }
                order.iter().find_map(|&side| self.find(region, pos, side))
            }
            surface => {
                let ceiling = match surface {
                    Surface::Ceiling => true,
                    Surface::Floor => false,
                    _ => random.next_boolean(),
                };
                let open = is_open(region.blocks, read(region, pos));
                let direction = if ceiling == open { Direction::Up } else { Direction::Down };
                self.find(region, pos, direction)
            }
        };
        match snapped {
            Some(target) => self.feature.place(region, random, target),
            None => Ok(false),
        }
    }

    fn find(&self, region: &Region, pos: IVec3, direction: Direction) -> Option<IVec3> {
        let blocks = region.blocks;
        let origin = read(region, pos);
        let solid = !is_open(blocks, origin);
        let open = |block: BlockId| (self.allow_air && blocks.is_air(block)) || (self.allow_water && is_water_source(blocks, block));
        let inside = |block: BlockId| if solid { self.allow_solid && !is_open(blocks, block) } else { open(block) };
        let edge = |block: BlockId| {
            if !self.allowed.is_empty() {
                self.allowed.contains(blocks, block)
            } else if solid {
                open(block)
            } else {
                holds_face(blocks, block)
            }
        };
        if !inside(origin) {
            return None;
        }
        let biome = biome_id_at(region, pos);
        let step = direction.offset();
        let mut steps = 0;
        while steps < self.range - 1 {
            let next = pos + step * (steps + 1);
            if biome_id_at(region, next) != biome || !inside(read(region, next)) {
                break;
            }
            steps += 1;
        }
        let beyond = pos + step * (steps + 1);
        edge(read(region, beyond)).then(|| pos + step * (steps + i32::from(solid != self.embed)))
    }
}
