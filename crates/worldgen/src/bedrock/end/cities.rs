use super::{BlockId, EndChunk};
use crate::shared::rng::mersenne::MersenneTwister;
use crate::shared::structure::{BoundingBox, Grid, PaletteBlocks, Rotation, Spread, StateKind, Template, TemplateLibrary};

pub const GRID: Grid = Grid::new(20, 11, 10_387_313, Spread::Triangular);
pub const REACH: i32 = 12;
const START_DISTANCE: i64 = 4096;
const MIN_HEIGHT: i32 = 60;
const DEPTH_LIMIT: i32 = 8;
const HOUSE_DEPTH_LIMIT: i32 = 9;
const FAT_TOWER_FLOORS: i32 = 2;
const TOWER_SECTION: i32 = 4;
const TOWER_TOP: i32 = 7;
const TOWER_BRIDGES: [(i32, (i32, i32, i32)); 4] = [(0, (1, -1, 0)), (1, (6, -1, 1)), (3, (0, -1, 5)), (2, (5, -1, 6))];
const FAT_TOWER_BRIDGES: [(i32, (i32, i32, i32)); 4] = [(0, (4, -1, 0)), (1, (12, -1, 4)), (3, (0, -1, 8)), (2, (8, -1, 12))];
const TOP_CORNERS: [usize; 4] = [12, 2, 2, 12];
const TOP_CORNERS_Z: [usize; 4] = [12, 12, 2, 2];
const CENTRE: usize = 7;

#[derive(Debug, Clone)]
pub struct Piece {
    pub name: &'static str,
    pub pos: (i32, i32, i32),
    pub rotation: Rotation,
    pub overwrite: bool,
    kind: i64,
    pub bounds: BoundingBox,
}

impl Piece {
    fn new(name: &'static str, pos: (i32, i32, i32), rotation: Rotation, overwrite: bool, templates: &TemplateLibrary) -> Self {
        let (size_x, size_y, size_z) = templates.get(name).map_or((0, 0, 0), |template| template.size);
        let (size_x, size_z) = if matches!(rotation, Rotation::Clockwise90 | Rotation::CounterClockwise90) {
            (size_z, size_x)
        } else {
            (size_x, size_z)
        };
        let (x0, z0, x1, z1) = match rotation {
            Rotation::None => (0, 0, size_x, size_z),
            Rotation::Clockwise90 => (-size_x, 0, 0, size_z),
            Rotation::Clockwise180 => (-size_x, -size_z, 0, 0),
            Rotation::CounterClockwise90 => (0, -size_z, size_x, 0),
        };
        let bounds = BoundingBox::new(pos.0 + x0, pos.1, pos.2 + z0, pos.0 + x1, pos.1 + size_y - 1, pos.2 + z1);
        Self {
            name,
            pos,
            rotation,
            overwrite,
            kind: 0,
            bounds,
        }
    }

    fn moved(mut self, offset: (i32, i32, i32)) -> Self {
        let (dx, dy, dz) = offset;
        self.pos = (self.pos.0 + dx, self.pos.1 + dy, self.pos.2 + dz);
        let bounds = &mut self.bounds;
        (bounds.x0, bounds.y0, bounds.z0, bounds.x1, bounds.y1, bounds.z1) = (bounds.x0 + dx, bounds.y0 + dy, bounds.z0 + dz, bounds.x1 + dx, bounds.y1 + dy, bounds.z1 + dz);
        self
    }
}

#[derive(Debug)]
pub struct City {
    pub pieces: Vec<Piece>,
    pub bounds: BoundingBox,
}

pub fn rotation(start: (i32, i32)) -> Rotation {
    Rotation::from_index((MersenneTwister::new(start.0.wrapping_add(start.1.wrapping_mul(10_387_313)) as u32).next_u32() & 3) as i32)
}

pub fn beyond_origin(chunk: (i32, i32)) -> bool {
    i64::from(chunk.0) * i64::from(chunk.0) + i64::from(chunk.1) * i64::from(chunk.1) > START_DISTANCE
}

pub fn start_height(top: &impl Fn(usize, usize) -> i32, rotation: Rotation) -> i32 {
    let index = rotation as usize;
    let (a, b, c, d) = (
        top(CENTRE, CENTRE),
        top(CENTRE, TOP_CORNERS_Z[index]),
        top(TOP_CORNERS[index], CENTRE),
        top(TOP_CORNERS[index], TOP_CORNERS_Z[index]),
    );
    a.min(b).min(c).min(d)
}

pub fn is_start(chunk: (i32, i32), island_height: f32, height: i32) -> bool {
    beyond_origin(chunk) && island_height.floor() as i16 >= 0 && height >= MIN_HEIGHT
}

pub fn generate(random: &mut MersenneTwister, start: (i32, i32), height: i32, templates: &TemplateLibrary) -> City {
    let rotation = rotation(start);
    let origin = (start.0 * 16 + 8, height, start.1 * 16 + 8);
    let pieces = generate_pieces(random, origin, rotation, templates);
    let bounds = pieces.iter().skip(1).fold(pieces[0].bounds, |bounds, piece| bounds.union(&piece.bounds));
    City { pieces, bounds }
}

pub struct Materials<'a> {
    pub templates: &'a TemplateLibrary,
    pub palette: &'a PaletteBlocks,
    pub frames: &'a [BlockId; 4],
    pub air: BlockId,
}

pub fn place(city: &City, chunk: (i32, i32), target: &mut EndChunk, materials: &Materials<'_>) {
    let column = BoundingBox::chunk(chunk.0, chunk.1, target.definition());
    for piece in city.pieces.iter().filter(|piece| piece.bounds.intersects(&column)) {
        let Some(template) = materials.templates.get(piece.name) else { continue };
        place_piece(piece, &template, chunk, target, materials);
    }
}

fn place_piece(piece: &Piece, template: &Template, chunk: (i32, i32), target: &mut EndChunk, materials: &Materials<'_>) {
    for (index, block) in template.blocks.iter().enumerate() {
        if let Some((_, marker)) = template.markers.iter().find(|(marked, _)| *marked == index) {
            if marker != "Elytra" {
                continue;
            }
            let (dx, dy, dz) = piece.rotation.apply((block.x as i32, block.y as i32, block.z as i32));
            let (x, y, z) = (piece.pos.0 + dx, piece.pos.1 + dy, piece.pos.2 + dz);
            if (x >> 4, z >> 4) == chunk && target.contains_y(y) {
                target.set(x & 15, y, z & 15, materials.frames[piece.rotation as usize]);
            }
            continue;
        }
        if materials.templates.kind(block.state) != StateKind::Block {
            continue;
        }
        if !piece.overwrite && materials.palette.block(materials.templates.rotated(block.state, 0)) == materials.air {
            continue;
        }
        let (dx, dy, dz) = piece.rotation.apply((block.x as i32, block.y as i32, block.z as i32));
        let (x, y, z) = (piece.pos.0 + dx, piece.pos.1 + dy, piece.pos.2 + dz);
        if (x >> 4, z >> 4) != chunk || !target.contains_y(y) {
            continue;
        }
        target.set(x & 15, y, z & 15, materials.palette.turned_block(block.state, piece.rotation as usize));
    }
}

#[derive(Clone, Copy)]
enum Generator {
    Tower,
    FatTower,
    Bridge,
    House,
}

fn turned(rotation: Rotation, by: i32) -> Rotation {
    Rotation::from_index(rotation as i32 + by)
}

fn add(list: &mut Vec<Piece>, parent: &Piece, offset: (i32, i32, i32), name: &'static str, rotation: Rotation, overwrite: bool, templates: &TemplateLibrary) -> usize {
    list.push(Piece::new(name, parent.pos, rotation, overwrite, templates).moved(parent.rotation.apply(offset)));
    list.len() - 1
}

struct Builder<'a> {
    random: &'a mut MersenneTwister,
    templates: &'a TemplateLibrary,
    ship: bool,
}

impl Builder<'_> {
    fn recurse(&mut self, generator: Generator, depth: i32, parent: &Piece, pos: (i32, i32, i32), list: &mut Vec<Piece>) -> bool {
        if depth > DEPTH_LIMIT {
            return false;
        }
        let mut local = Vec::new();
        if !self.generate(generator, depth, parent, pos, &mut local) {
            return false;
        }
        let kind = i64::from(self.random.next_int());
        for piece in &mut local {
            piece.kind = kind;
        }
        for piece in &local {
            if list.iter().find(|other| other.bounds.intersects(&piece.bounds)).is_some_and(|other| other.kind != parent.kind) {
                return false;
            }
        }
        list.extend(local);
        true
    }

    fn generate(&mut self, generator: Generator, depth: i32, parent: &Piece, pos: (i32, i32, i32), list: &mut Vec<Piece>) -> bool {
        match generator {
            Generator::Tower => self.tower(depth, parent, list),
            Generator::FatTower => self.fat_tower(depth, parent, list),
            Generator::Bridge => self.bridge(depth, parent, list),
            Generator::House => self.house(depth, parent, pos, list),
        }
    }

    fn tower(&mut self, depth: i32, parent: &Piece, list: &mut Vec<Piece>) -> bool {
        let rotation = parent.rotation;
        let (a, b) = (self.random.next_int_bounded(2), self.random.next_int_bounded(2));
        let mut top = add(list, parent, (3 + a, -3, 3 + b), "endcity/tower_base", rotation, true, self.templates);
        top = add(list, &list[top].clone(), (0, 7, 0), "endcity/tower_piece", rotation, true, self.templates);
        let floor_first = self.random.next_int_bounded(3) == 0;
        let floors = self.random.next_int_bounded(3);
        let mut floor = floor_first.then_some(top);
        for level in 0..=floors {
            top = add(list, &list[top].clone(), (0, TOWER_SECTION, 0), "endcity/tower_piece", rotation, true, self.templates);
            if level < floors && self.random.next_boolean() {
                floor = Some(top);
            }
        }
        if let Some(floor) = floor {
            let floor = list[floor].clone();
            for (turn, offset) in TOWER_BRIDGES {
                if self.random.next_boolean() {
                    let end = add(list, &floor, offset, "endcity/bridge_end", turned(rotation, turn), true, self.templates);
                    let end = list[end].clone();
                    self.recurse(Generator::Bridge, depth + 1, &end, (0, 0, 0), list);
                }
            }
        } else if depth != TOWER_TOP {
            let top = list[top].clone();
            return self.recurse(Generator::FatTower, depth + 1, &top, (0, 0, 0), list);
        }
        add(list, &list[top].clone(), (-1, 4, -1), "endcity/tower_top", rotation, true, self.templates);
        true
    }

    fn fat_tower(&mut self, depth: i32, parent: &Piece, list: &mut Vec<Piece>) -> bool {
        let rotation = parent.rotation;
        let mut top = add(list, parent, (-3, 4, -3), "endcity/fat_tower_base", rotation, true, self.templates);
        top = add(list, &list[top].clone(), (0, 4, 0), "endcity/fat_tower_middle", rotation, true, self.templates);
        let mut level = 0;
        while level < FAT_TOWER_FLOORS && self.random.next_int_bounded(3) != 0 {
            top = add(list, &list[top].clone(), (0, 8, 0), "endcity/fat_tower_middle", rotation, true, self.templates);
            let middle = list[top].clone();
            for (turn, offset) in FAT_TOWER_BRIDGES {
                if self.random.next_boolean() {
                    let end = add(list, &middle, offset, "endcity/bridge_end", turned(rotation, turn), true, self.templates);
                    let end = list[end].clone();
                    self.recurse(Generator::Bridge, depth + 1, &end, (0, 0, 0), list);
                }
            }
            level += 1;
        }
        add(list, &list[top].clone(), (-2, 8, -2), "endcity/fat_tower_top", rotation, true, self.templates);
        true
    }

    fn bridge(&mut self, depth: i32, parent: &Piece, list: &mut Vec<Piece>) -> bool {
        let rotation = parent.rotation;
        let length = self.random.next_int_bounded(4);
        let mut top = add(list, parent, (0, 0, -4), "endcity/bridge_piece", rotation, true, self.templates);
        list[top].kind = -1;
        let mut drop = 0;
        for _ in 0..=length {
            let last = list[top].clone();
            if self.random.next_boolean() {
                top = add(list, &last, (0, drop, -4), "endcity/bridge_piece", rotation, true, self.templates);
                drop = 0;
            } else {
                top = if self.random.next_boolean() {
                    add(list, &last, (0, drop, -4), "endcity/bridge_steep_stairs", rotation, true, self.templates)
                } else {
                    add(list, &last, (0, drop, -8), "endcity/bridge_gentle_stairs", rotation, true, self.templates)
                };
                drop = 4;
            }
        }
        let last = list[top].clone();
        if !self.ship && self.random.next_int_bounded(10 - depth) == 0 {
            let a = self.random.next_int_bounded(8);
            let b = self.random.next_int_bounded(10);
            add(list, &last, (a - 8, drop, b - 70), "endcity/ship", rotation, true, self.templates);
            self.ship = true;
        } else if !self.recurse(Generator::House, depth + 1, &last, (-3, drop + 1, -11), list) {
            return false;
        }
        let end = add(list, &last, (4, drop, 0), "endcity/bridge_end", turned(rotation, 2), true, self.templates);
        list[end].kind = -1;
        true
    }

    fn house(&mut self, depth: i32, parent: &Piece, pos: (i32, i32, i32), list: &mut Vec<Piece>) -> bool {
        if depth >= HOUSE_DEPTH_LIMIT {
            return false;
        }
        let rotation = parent.rotation;
        let mut top = add(list, parent, pos, "endcity/base_floor", rotation, true, self.templates);
        let kind = self.random.next_int_bounded(3);
        if kind == 0 {
            add(list, &list[top].clone(), (-1, 4, -1), "endcity/base_roof", rotation, true, self.templates);
            return true;
        }
        top = add(list, &list[top].clone(), (-1, 0, -1), "endcity/second_floor_2", rotation, false, self.templates);
        if kind == 1 {
            top = add(list, &list[top].clone(), (-1, 8, -1), "endcity/second_roof", rotation, false, self.templates);
        } else {
            top = add(list, &list[top].clone(), (-1, 4, -1), "endcity/third_floor_c", rotation, false, self.templates);
            top = add(list, &list[top].clone(), (-1, 8, -1), "endcity/third_roof", rotation, true, self.templates);
        }
        let top = list[top].clone();
        self.recurse(Generator::Tower, depth + 1, &top, (0, 0, 0), list);
        true
    }
}

fn generate_pieces(random: &mut MersenneTwister, origin: (i32, i32, i32), rotation: Rotation, templates: &TemplateLibrary) -> Vec<Piece> {
    let base = Piece::new("endcity/base_floor", origin, rotation, true, templates);
    let mut pieces = vec![base.clone()];
    let mut parent = base;
    for (offset, name, overwrite) in [
        ((-1, 0, -1), "endcity/second_floor", false),
        ((-1, 4, -1), "endcity/third_floor", false),
        ((-1, 8, -1), "endcity/third_roof", true),
    ] {
        let top = add(&mut pieces, &parent, offset, name, rotation, overwrite, templates);
        parent = pieces[top].clone();
    }
    let roof = parent;
    Builder { random, templates, ship: false }.recurse(Generator::Tower, 1, &roof, (0, 0, 0), &mut pieces);
    pieces
}
