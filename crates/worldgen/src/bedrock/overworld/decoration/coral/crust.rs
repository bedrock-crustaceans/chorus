use super::super::aquatic::is_solid_blocking;
use super::super::ore::Draw;
use super::ROTATION_COUNT;
use super::kinds::Coral;
use super::reef::Reef;
use crate::shared::blocks::{BlockId, BlockTable};
use crate::shared::chunk::region::Region;
use crate::shared::structure::Rotation;
use crate::shared::structure::{StateKind, StructureData, TemplateLibrary};
use chorus_block::block_id::*;
use glam::IVec3;
use std::sync::LazyLock;

const CRUST_DEPTH: i32 = 9;

const SIDE_SPREAD_REACH: i32 = 9;

const SIDE_SPREAD_GAP: i32 = 3;

const CLUSTER_TRIES: i32 = 4;

const CLUSTER_EXTRA_TRIES: i32 = 5;

const OUTCROP_KINDS: usize = 6;

const ROTATIONS: usize = 4;

const CRUST_TEMPLATE_COUNT: usize = 11;

const CRUST_COUNT: usize = 5;

const PROBE: [[[i32; 2]; ROTATIONS]; CRUST_COUNT] = [
    [[6, 6], [-7, 6], [-7, -7], [6, -7]],
    [[6, 6], [-6, 6], [-7, -7], [6, -7]],
    [[4, 4], [-4, 4], [-4, -4], [4, -4]],
    [[4, 4], [-4, 4], [-4, -4], [4, -4]],
    [[3, 6], [-7, 3], [-2, -7], [7, -3]],
];

const OUTCROP_OFFSET: [[[i32; 3]; ROTATIONS]; OUTCROP_KINDS] = [
    [[-5, 0, -2], [3, 0, -5], [5, 0, 2], [-2, 0, 5]],
    [[-7, -2, -3], [3, -2, -6], [6, -2, 3], [-2, -2, 5]],
    [[-4, 0, -3], [3, 0, -4], [4, 0, 3], [-3, 0, 4]],
    [[-3, 0, -3], [3, 0, -3], [3, 0, 3], [-3, 0, 3]],
    [[-2, 0, -2], [2, 0, -2], [2, 0, 2], [-2, 0, 2]],
    [[-5, 0, -5], [6, 0, -6], [5, 0, 5], [-5, 0, 5]],
];

const CRUST_BLOCKS: [&str; CRUST_COUNT] = [DEAD_BUBBLE_CORAL_BLOCK, ANDESITE, DIORITE, GRANITE, SAND];

#[derive(Debug)]
struct TemplateBlock {
    x: i32,
    y: i32,
    z: i32,
    kind: u8,
}

#[derive(Debug)]
struct Crust {
    blocks: Vec<TemplateBlock>,
    markers: Vec<IVec3>,
}

const CRUST_TEMPLATES: [&str; CRUST_TEMPLATE_COUNT] = [
    "coralcrust/crust1",
    "coralcrust/crust2",
    "coralcrust/crust3",
    "coralcrust/crust4",
    "coralcrust/crust5",
    "coralcrust/outcropping1",
    "coralcrust/outcropping2",
    "coralcrust/outcropping3",
    "coralcrust/outcropping4",
    "coralcrust/outcropping5",
    "coralcrust/outcropping6",
];

fn crust(library: &TemplateLibrary, name: &str) -> Crust {
    let template = library.get(name).expect("a coral crust template");
    let mut blocks = Vec::new();
    for block in &template.blocks {
        if library.kind(block.state) != StateKind::Block {
            continue;
        }
        let block_name = format!("minecraft:{}", library.entry(library.rotated(block.state, 0)).name);
        let kind = CRUST_BLOCKS.iter().position(|crust| *crust == block_name).expect("a coral crust block");
        blocks.push(TemplateBlock {
            x: block.x as i32,
            y: block.y as i32,
            z: block.z as i32,
            kind: kind as u8,
        });
    }
    let markers = template
        .markers
        .iter()
        .map(|(index, _)| {
            let block = template.blocks[*index];
            IVec3::new(block.x as i32, block.y as i32, block.z as i32)
        })
        .collect();
    Crust { blocks, markers }
}

fn templates() -> &'static [Crust] {
    static CELL: LazyLock<Vec<Crust>> = LazyLock::new(|| {
        let data = StructureData::load().expect("the coral crust templates are part of the structure data");
        let library = data.library.as_ref().expect("the structure data holds the templates");
        CRUST_TEMPLATES.iter().map(|name| crust(library, name)).collect()
    });
    &CELL
}

fn probe(size: usize, rotation: usize) -> [i32; 2] {
    PROBE[size][rotation]
}

fn placed_offset(size: usize, rotation: usize) -> [i32; 2] {
    let [x, z] = probe(size, rotation);
    [-x, -z]
}

fn spread_offset<D: Draw>(random: &mut D) -> i32 {
    let reach = random.below(SIDE_SPREAD_REACH);
    if random.below(2) == 0 { -SIDE_SPREAD_GAP - reach } else { reach + SIDE_SPREAD_GAP }
}

#[derive(Debug)]
pub struct CoralCrust {
    coral: Coral,
    blocks: [BlockId; CRUST_COUNT],
}

impl CoralCrust {
    pub fn new(table: &mut BlockTable) -> Self {
        Self {
            coral: Coral::new(table),
            blocks: std::array::from_fn(|index| table.name(CRUST_BLOCKS[index])),
        }
    }

    pub fn place<D: Draw>(&self, region: &mut Region, random: &mut D, origin: IVec3) -> bool {
        let ceiling = region.sea_level() - CRUST_DEPTH;
        let size = random.below(CRUST_COUNT as i32) as usize;
        let rotation = random.below(ROTATION_COUNT) as usize;
        let [probe_x, probe_z] = probe(size, rotation);
        let mut pos = origin + IVec3::new(probe_x, 0, probe_z);
        while pos.y < ceiling && !self.coral.water(region, pos) {
            pos.y += 1;
        }
        if pos.y >= ceiling || !is_solid_blocking(region, pos - IVec3::Y) || self.coral.is_live_coral(region, pos - IVec3::Y) {
            return false;
        }
        let [back_x, back_z] = placed_offset(size, rotation);
        let mut crusts = vec![(size, rotation, pos + IVec3::new(back_x, 0, back_z))];
        if random.below(CLUSTER_TRIES) != 0 {
            let tries = random.below(CLUSTER_EXTRA_TRIES) + CLUSTER_TRIES;
            for _ in 0..tries {
                let size = random.below(CRUST_COUNT as i32) as usize;
                let rotation = random.below(ROTATION_COUNT) as usize;
                let x = spread_offset(random);
                let z = spread_offset(random);
                let [probe_x, probe_z] = probe(size, rotation);
                let target = IVec3::new(x + pos.x + probe_x, pos.y, z + pos.z + probe_z);
                if is_solid_blocking(region, target - IVec3::Y) {
                    let [back_x, back_z] = placed_offset(size, rotation);
                    crusts.push((size, rotation, IVec3::new(target.x + back_x, target.y, target.z + back_z)));
                }
            }
        }
        if crusts.len() == 1 {
            let kind = random.below(OUTCROP_KINDS as i32) as usize;
            let [offset_x, offset_y, offset_z] = OUTCROP_OFFSET[kind][rotation];
            let origin = pos + IVec3::new(offset_x, offset_y, offset_z);
            self.place_template(region, random, &templates()[CRUST_COUNT + kind], origin, rotation);
        } else {
            for (size, rotation, origin) in crusts {
                self.place_template(region, random, &templates()[size], origin, rotation);
            }
        }
        true
    }

    fn place_template<D: Draw>(&self, region: &mut Region, random: &mut D, template: &Crust, origin: IVec3, rotation: usize) {
        let turn = Rotation::from_index(rotation as i32);
        for block in &template.blocks {
            let (dx, dy, dz) = turn.apply((block.x, block.y, block.z));
            region.set(origin + IVec3::new(dx, dy, dz), self.blocks[block.kind as usize]);
        }
        let markers: Vec<IVec3> = template
            .markers
            .iter()
            .map(|marker| {
                let (dx, dy, dz) = turn.apply((marker.x, marker.y, marker.z));
                origin + IVec3::new(dx, dy, dz)
            })
            .collect();
        let mut reef = Reef::new(&self.coral, region, random);
        reef.coral_base(&markers);
        reef.decorate();
    }
}
