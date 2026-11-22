use super::entities::entities_at;
use super::palette::PaletteBlocks;
use super::plan::Plan;
use super::template::{StateKind, Template, TemplateLibrary};
use crate::shared::rng::mersenne::MersenneTwister;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rotation {
    None = 0,
    Clockwise90 = 1,
    Clockwise180 = 2,
    CounterClockwise90 = 3,
}

impl Rotation {
    pub const ALL: [Rotation; 4] = [Rotation::None, Rotation::Clockwise90, Rotation::Clockwise180, Rotation::CounterClockwise90];

    pub fn from_index(index: i32) -> Self {
        Self::ALL[index.rem_euclid(4) as usize]
    }

    pub fn apply_point(self, (x, y, z): (f64, f64, f64)) -> (f64, f64, f64) {
        match self {
            Rotation::None => (x, y, z),
            Rotation::Clockwise90 => (1.0 - z, y, x),
            Rotation::Clockwise180 => (1.0 - x, y, 1.0 - z),
            Rotation::CounterClockwise90 => (z, y, 1.0 - x),
        }
    }

    pub fn apply(self, (x, y, z): (i32, i32, i32)) -> (i32, i32, i32) {
        match self {
            Rotation::None => (x, y, z),
            Rotation::Clockwise90 => (-z, y, x),
            Rotation::Clockwise180 => (-x, y, -z),
            Rotation::CounterClockwise90 => (z, y, -x),
        }
    }
}

pub struct Library<'a> {
    pub templates: &'a TemplateLibrary,
    pub palette: &'a PaletteBlocks,
}

#[derive(Clone, Copy)]
pub struct Wood {
    pub families: &'static [[&'static str; 6]],
    pub remap: [Option<u8>; 6],
}

#[derive(Clone, Copy, Default)]
pub struct Options {
    pub mirror: bool,
    pub ignore_air: bool,
    pub flood: bool,
    pub wood: Option<Wood>,
    pub integrity: Option<(f32, u32)>,
}

impl Library<'_> {
    pub(crate) fn mirror(&self, index: u16, rotation: Rotation) -> u16 {
        self.palette.mirrored(index, matches!(rotation, Rotation::None | Rotation::Clockwise180))
    }

    fn rewood(&self, index: u16, wood: &Wood) -> u16 {
        let name = self.palette.name(index);
        for family in wood.families {
            if let Some(kind) = family.iter().position(|member| *member == name) {
                return match wood.remap[kind] {
                    Some(to) => self.palette.find(family[to as usize], self.palette.states(index), self.palette.is_waterlogged(index)).unwrap_or(index),
                    None => index,
                };
            }
        }
        index
    }

    pub fn place(&self, plan: &mut Plan, template: &Template, origin: (i32, i32, i32), rotation: Rotation, options: Options) -> usize {
        let water = plan.ids.water;
        let mut chests = 0;
        let mut integrity = options.integrity.filter(|&(chance, _)| chance < 1.0).map(|(chance, seed)| (chance, MersenneTwister::new(seed)));
        for (index, block) in template.blocks.iter().enumerate() {
            if self.templates.kind(block.state) != StateKind::Block {
                continue;
            }
            if options.ignore_air && self.palette.is_air(self.templates.rotated(block.state, Rotation::None as usize)) {
                continue;
            }
            if let Some((chance, random)) = integrity.as_mut()
                && *chance < random.next_float()
            {
                continue;
            }
            let local = (block.x as i32, block.y as i32, block.z as i32);
            let local = if options.mirror { (local.0, local.1, -local.2) } else { local };
            let (dx, dy, dz) = rotation.apply(local);
            let at = (origin.0 + dx, origin.1 + dy, origin.2 + dz);
            let mut palette_index = self.templates.rotated(block.state, rotation as usize);
            if options.mirror {
                palette_index = self.mirror(palette_index, rotation);
            }
            if let Some(wood) = &options.wood {
                palette_index = self.rewood(palette_index, wood);
            }
            let mut id = self.palette.block(palette_index);
            if options.flood && at.1 < plan.sea_level() {
                id = if self.palette.is_air(palette_index) { water } else { self.palette.soaked(palette_index) };
            }
            plan.set(at.0, at.1, at.2, id);
            chests += usize::from(template.loot_chests.contains(&index));
        }
        plan.entities.extend(entities_at(template, origin, rotation, options.mirror));
        chests
    }
}

pub fn markers_at(template: &Template, origin: (i32, i32, i32), rotation: Rotation) -> Vec<((i32, i32, i32), &str)> {
    let mut placed: Vec<((i32, i32, i32), &str)> = template
        .markers
        .iter()
        .map(|(index, data)| {
            let block = template.blocks[*index];
            let (dx, dy, dz) = rotation.apply((block.x as i32, block.y as i32, block.z as i32));
            ((origin.0 + dx, origin.1 + dy, origin.2 + dz), data.as_str())
        })
        .collect();
    placed.sort_by_key(|((x, y, z), _)| (*y, *z, *x));
    placed
}
