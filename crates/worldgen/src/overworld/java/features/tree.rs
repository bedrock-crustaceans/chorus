use std::collections::HashSet;

use glam::IVec3;

use super::super::blocks::{self, BlockId, Blocks, State, text};
use super::super::carver::{cos, sin};
use super::super::proto::BlockEntity;
use super::super::random::RandomSource;
use super::super::tags::Tag;
use super::predicate::{BlockPredicate, Direction, Fluid, fluid_at};
use super::provider::{IntProvider, StateProvider};
use super::region::{Heightmap, Region};
use super::{Context, Feature, Shapes};
use chorus_block::block_id::{VINE, WATER};
use chorus_block::state::common::{PERSISTENT_BIT, PILLAR_AXIS};

#[derive(Default)]
pub struct PositionSet {
    order: Vec<IVec3>,
    seen: HashSet<IVec3>,
}

fn java_hash(pos: IVec3) -> i32 {
    let hash = pos.y.wrapping_add(pos.z.wrapping_mul(31)).wrapping_mul(31).wrapping_add(pos.x);
    hash ^ ((hash as u32) >> 16) as i32
}

impl PositionSet {
    pub fn insert(&mut self, pos: IVec3) {
        if self.seen.insert(pos) {
            self.order.push(pos);
        }
    }

    pub fn contains(&self, pos: IVec3) -> bool {
        self.seen.contains(&pos)
    }

    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    pub fn iteration_order(&self) -> Vec<IVec3> {
        let mut capacity = 16usize;
        while self.order.len() > capacity * 3 / 4 {
            capacity *= 2;
        }
        let mut ordered = self.order.clone();
        ordered.sort_by_key(|&pos| java_hash(pos) as u32 as usize & (capacity - 1));
        ordered
    }

    fn sorted_by_y(&self) -> Vec<IVec3> {
        let mut ordered = self.iteration_order();
        ordered.sort_by_key(|pos| pos.y);
        ordered
    }
}

pub fn shuffle<T>(items: &mut [T], random: &mut dyn RandomSource) {
    for i in (2..=items.len()).rev() {
        let swap = random.next_int_bounded(i as i32) as usize;
        items.swap(i - 1, swap);
    }
}

fn random_horizontal(random: &mut dyn RandomSource) -> Direction {
    Direction::HORIZONTAL[random.next_int_bounded(4) as usize]
}

fn pillar(axis: &'static str) -> State {
    blocks::state(PILLAR_AXIS, text(axis))
}

fn axis_of(direction: Direction) -> &'static str {
    match direction {
        Direction::East | Direction::West => "x",
        Direction::North | Direction::South => "z",
        _ => "y",
    }
}

fn manhattan(a: IVec3, b: IVec3) -> i32 {
    (a - b).abs().element_sum()
}

pub fn valid_tree_pos(region: &Region, pos: IVec3) -> bool {
    let block = region.get(pos);
    region.blocks.is_air(block) || region.blocks.is(block, Tag::ReplaceableByTrees)
}

fn is_water_source(blocks: &Blocks, block: BlockId) -> bool {
    fluid_at(blocks, block) == Some(Fluid::Water)
}

fn waterlogged_if_wet(blocks: &Blocks, state: BlockId, current: BlockId) -> BlockId {
    if is_water_source(blocks, current) { blocks.flooded(state) } else { state }
}

#[derive(Clone, Copy)]
pub enum FeatureSize {
    TwoLayers { limit: i32, lower: i32, upper: i32, min_clipped: Option<i32> },
    ThreeLayers { limit: i32, upper_limit: i32, lower: i32, middle: i32, upper: i32 },
}

impl FeatureSize {
    pub fn two(limit: i32, lower: i32, upper: i32) -> Self {
        Self::TwoLayers {
            limit,
            lower,
            upper,
            min_clipped: None,
        }
    }

    fn size_at(self, tree_height: i32, y: i32) -> i32 {
        match self {
            Self::TwoLayers { limit, lower, upper, .. } => {
                if y < limit {
                    lower
                } else {
                    upper
                }
            }
            Self::ThreeLayers {
                limit,
                upper_limit,
                lower,
                middle,
                upper,
            } => {
                if y < limit {
                    lower
                } else if y >= tree_height - upper_limit {
                    upper
                } else {
                    middle
                }
            }
        }
    }

    fn min_clipped(self) -> Option<i32> {
        match self {
            Self::TwoLayers { min_clipped, .. } => min_clipped,
            Self::ThreeLayers { .. } => None,
        }
    }
}

pub struct Attachment {
    pos: IVec3,
    radius_offset: i32,
    height_offset: i32,
    double_trunk: bool,
}

impl Attachment {
    fn new(pos: IVec3, radius_offset: i32, double_trunk: bool) -> Self {
        Self {
            pos,
            radius_offset,
            height_offset: 0,
            double_trunk,
        }
    }
}

pub enum TrunkKind {
    Straight,
    Forking,
    Giant,
    MegaJungle,
    DarkOak,
    Fancy,
    Bending {
        min_height_for_leaves: i32,
        bend_length: IntProvider,
    },
    UpwardsBranching {
        extra_branch_steps: IntProvider,
        branch_chance: f32,
        extra_branch_length: IntProvider,
        can_grow_through: Tag,
    },
    Cherry {
        branch_count: IntProvider,
        branch_horizontal_length: IntProvider,
        branch_start: (i32, i32),
        branch_end: IntProvider,
    },
    Poplar {
        trunk_above_branches: IntProvider,
        branch_amount: IntProvider,
    },
}

pub struct TrunkPlacer {
    pub base_height: i32,
    pub rand_a: i32,
    pub rand_b: i32,
    pub kind: TrunkKind,
}

pub enum FoliageKind {
    Blob {
        height: i32,
    },
    Bush {
        height: i32,
    },
    Fancy {
        height: i32,
    },
    Spruce {
        trunk_height: IntProvider,
    },
    Pine {
        height: IntProvider,
    },
    Acacia,
    DarkOak,
    MegaPine {
        crown_height: IntProvider,
    },
    MegaJungle {
        height: i32,
    },
    RandomSpread {
        height: IntProvider,
        attempts: i32,
    },
    Cherry {
        height: IntProvider,
        wide_bottom_hole: f32,
        corner_hole: f32,
        hanging: f32,
        hanging_extension: f32,
    },
    Poplar {
        height: IntProvider,
        side_hole: f32,
    },
}

pub struct FoliagePlacer {
    pub radius: IntProvider,
    pub offset: IntProvider,
    pub kind: FoliageKind,
}

pub struct MangroveRoots {
    pub trunk_offset_y: IntProvider,
    pub root: BlockId,
    pub above_root: Option<(BlockId, f32)>,
    pub can_grow_through: Tag,
    pub muddy_roots_in: Vec<u16>,
    pub muddy_roots: BlockId,
    pub max_root_width: i32,
    pub max_root_length: i32,
    pub random_skew_chance: f32,
}

pub enum Decorator {
    AlterGround(StateProvider),
    AttachedToLeaves {
        probability: f32,
        exclusion_xz: i32,
        exclusion_y: i32,
        provider: StateProvider,
        required_empty: i32,
        directions: Vec<Direction>,
    },
    AttachedToLogs {
        probability: f32,
        provider: StateProvider,
        directions: Vec<Direction>,
    },
    Beehive {
        probability: f32,
        nest: BlockId,
    },
    Cocoa {
        probability: f32,
        states: Vec<BlockId>,
    },
    CreakingHeart {
        probability: f32,
        heart: BlockId,
    },
    LeaveVine {
        probability: f32,
        vines: [BlockId; 4],
    },
    PaleMoss {
        leaves: f32,
        trunk: f32,
        ground: f32,
        patch: Box<Feature>,
        hanging: BlockId,
        tip: BlockId,
    },
    PlaceOnGround {
        tries: i32,
        radius: i32,
        height: i32,
        provider: StateProvider,
    },
    ShelfMushroom {
        probability: f32,
        states: Vec<BlockId>,
    },
    TrunkVine {
        vines: [BlockId; 4],
    },
}

#[derive(Default)]
struct Grown {
    roots: PositionSet,
    trunks: PositionSet,
    foliage: PositionSet,
}

pub struct Tree {
    pub trunk: StateProvider,
    pub trunk_placer: TrunkPlacer,
    pub foliage: StateProvider,
    pub foliage_placer: FoliagePlacer,
    pub roots: Option<MangroveRoots>,
    pub size: FeatureSize,
    pub decorators: Vec<Decorator>,
    pub ignore_vines: bool,
    pub below_trunk: StateProvider,
    pub vine: u16,
}

impl Tree {
    pub fn place(&self, ctx: &mut Context, random: &mut dyn RandomSource, origin: IVec3) -> bool {
        let mut grown = Grown::default();
        let placed = self.grow(ctx, random, origin, &mut grown);
        if !placed || (grown.trunks.is_empty() && grown.foliage.is_empty()) {
            return false;
        }
        let mut decorations = Vec::new();
        if !self.decorators.is_empty() {
            let logs = grown.trunks.sorted_by_y();
            let leaves = grown.foliage.sorted_by_y();
            let roots = grown.roots.sorted_by_y();
            let mut decoration = DecorationContext { logs, leaves, roots };
            ctx.region.start_recording();
            for decorator in &self.decorators {
                decorator.place(ctx, random, &mut decoration);
            }
            decorations = ctx.region.take_recording();
        }
        update_tree_edges(ctx, &grown, &decorations);
        true
    }

    fn grow(&self, ctx: &mut Context, random: &mut dyn RandomSource, origin: IVec3, grown: &mut Grown) -> bool {
        let tree_height = self.trunk_placer.tree_height(random);
        let foliage_height = self.foliage_placer.foliage_height(random, tree_height);
        let trunk_height = tree_height - foliage_height;
        let leaf_radius = self.foliage_placer.foliage_radius(random, trunk_height);
        let trunk_origin = match &self.roots {
            Some(roots) => origin + IVec3::Y * roots.trunk_offset_y.sample(random),
            None => origin,
        };
        let min_y = origin.y.min(trunk_origin.y);
        let max_y = origin.y.max(trunk_origin.y) + tree_height + 1;
        if min_y < ctx.region.min_y() + 1 || max_y > ctx.region.max_y() + 1 {
            return false;
        }
        let clipped = self.max_free_height(ctx.region, tree_height, trunk_origin);
        if clipped < tree_height && self.size.min_clipped().is_none_or(|min| clipped < min) {
            return false;
        }
        if let Some(roots) = &self.roots
            && !roots.place(ctx.region, random, origin, trunk_origin, grown)
        {
            return false;
        }
        let attachments = self.place_trunk(ctx.region, random, clipped, trunk_origin, grown);
        for attachment in &attachments {
            let offset = self.foliage_placer.offset.sample(random);
            self.create_foliage(ctx.region, random, attachment, foliage_height, leaf_radius, offset, grown);
        }
        true
    }

    fn max_free_height(&self, region: &Region, tree_height: i32, origin: IVec3) -> i32 {
        for y in 0..=tree_height + 1 {
            let r = self.size.size_at(tree_height, y);
            for x in -r..=r {
                for z in -r..=r {
                    let pos = origin + IVec3::new(x, y, z);
                    if !self.is_free(region, pos) || (!self.ignore_vines && region.blocks.entry(region.get(pos)).kind == self.vine) {
                        return y - 2;
                    }
                }
            }
        }
        tree_height
    }

    fn trunk_valid(&self, region: &Region, pos: IVec3) -> bool {
        if valid_tree_pos(region, pos) {
            return true;
        }
        match &self.trunk_placer.kind {
            TrunkKind::UpwardsBranching { can_grow_through, .. } => region.blocks.is(region.get(pos), *can_grow_through),
            _ => false,
        }
    }

    fn is_free(&self, region: &Region, pos: IVec3) -> bool {
        self.trunk_valid(region, pos) || region.blocks.is(region.get(pos), Tag::Logs)
    }

    fn set_trunk(region: &mut Region, grown: &mut Grown, pos: IVec3, state: BlockId) {
        grown.trunks.insert(pos);
        region.set(pos, state);
    }

    fn place_below(&self, region: &mut Region, random: &mut dyn RandomSource, pos: IVec3, grown: &mut Grown) {
        if let Some(state) = self.below_trunk.optional_state(region, random, pos) {
            Self::set_trunk(region, grown, pos, state);
        }
    }

    fn place_log_axis(&self, region: &mut Region, random: &mut dyn RandomSource, pos: IVec3, grown: &mut Grown, axis: Option<&'static str>) -> bool {
        if !self.trunk_valid(region, pos) {
            return false;
        }
        let mut state = self.trunk.state(region, random, pos);
        if let Some(axis) = axis {
            state = region.blocks.with(state, pillar(axis));
        }
        Self::set_trunk(region, grown, pos, state);
        true
    }

    fn place_log(&self, region: &mut Region, random: &mut dyn RandomSource, pos: IVec3, grown: &mut Grown) -> bool {
        self.place_log_axis(region, random, pos, grown, None)
    }

    fn place_log_if_free(&self, region: &mut Region, random: &mut dyn RandomSource, pos: IVec3, grown: &mut Grown) {
        if self.is_free(region, pos) {
            self.place_log(region, random, pos, grown);
        }
    }

    fn place_trunk(&self, region: &mut Region, random: &mut dyn RandomSource, height: i32, origin: IVec3, grown: &mut Grown) -> Vec<Attachment> {
        match &self.trunk_placer.kind {
            TrunkKind::Straight => {
                self.place_below(region, random, origin - IVec3::Y, grown);
                for y in 0..height {
                    self.place_log(region, random, origin + IVec3::Y * y, grown);
                }
                vec![Attachment::new(origin + IVec3::Y * height, 0, false)]
            }
            TrunkKind::Forking => self.forking_trunk(region, random, height, origin, grown),
            TrunkKind::Giant => self.giant_trunk(region, random, height, origin, grown),
            TrunkKind::MegaJungle => {
                let mut attachments = self.giant_trunk(region, random, height, origin, grown);
                let mut branch_height = height - 2 - random.next_int_bounded(4);
                while branch_height > height / 2 {
                    let angle = random.next_float() * std::f32::consts::TAU;
                    let (mut bx, mut bz) = (0, 0);
                    for b in 0..5 {
                        bx = (1.5 + cos(angle as f64) * b as f32) as i32;
                        bz = (1.5 + sin(angle as f64) * b as f32) as i32;
                        self.place_log(region, random, origin + IVec3::new(bx, branch_height - 3 + b / 2, bz), grown);
                    }
                    attachments.push(Attachment::new(origin + IVec3::new(bx, branch_height, bz), -2, false));
                    branch_height -= 2 + random.next_int_bounded(4);
                }
                attachments
            }
            TrunkKind::DarkOak => self.dark_oak_trunk(region, random, height, origin, grown),
            TrunkKind::Fancy => self.fancy_trunk(region, random, height, origin, grown),
            TrunkKind::Bending { min_height_for_leaves, bend_length } => {
                let direction = random_horizontal(random);
                let log_height = height - 1;
                let mut pos = origin;
                self.place_below(region, random, pos - IVec3::Y, grown);
                let mut attachments = Vec::new();
                for i in 0..=log_height {
                    if i + 1 >= log_height + random.next_int_bounded(2) {
                        pos += direction.offset();
                    }
                    if valid_tree_pos(region, pos) {
                        self.place_log(region, random, pos, grown);
                    }
                    if i >= *min_height_for_leaves {
                        attachments.push(Attachment::new(pos, 0, false));
                    }
                    pos += IVec3::Y;
                }
                let length = bend_length.sample(random);
                for _ in 0..=length {
                    if valid_tree_pos(region, pos) {
                        self.place_log(region, random, pos, grown);
                    }
                    attachments.push(Attachment::new(pos, 0, false));
                    pos += direction.offset();
                }
                attachments
            }
            TrunkKind::UpwardsBranching {
                extra_branch_steps,
                branch_chance,
                extra_branch_length,
                ..
            } => {
                let mut attachments = Vec::new();
                for height_pos in 0..height {
                    let current = origin.y + height_pos;
                    let log_pos = IVec3::new(origin.x, current, origin.z);
                    if self.place_log(region, random, log_pos, grown) && height_pos < height - 1 && random.next_float() < *branch_chance {
                        let direction = random_horizontal(random);
                        let length = extra_branch_length.sample(random);
                        let branch_pos = 0.max(length - extra_branch_length.sample(random) - 1);
                        let steps = extra_branch_steps.sample(random);
                        self.upwards_branch(region, random, height, &mut attachments, log_pos, current, direction, branch_pos, steps, grown);
                    }
                    if height_pos == height - 1 {
                        attachments.push(Attachment::new(IVec3::new(origin.x, current + 1, origin.z), 0, false));
                    }
                }
                attachments
            }
            TrunkKind::Cherry { .. } => self.cherry_trunk(region, random, height, origin, grown),
            TrunkKind::Poplar { trunk_above_branches, branch_amount } => {
                self.place_below(region, random, origin - IVec3::Y, grown);
                let up_to_branches = height - trunk_above_branches.sample(random);
                for y in 0..height {
                    let pos = origin + IVec3::Y * y;
                    self.place_log(region, random, pos, grown);
                    let mut directions = Direction::ALL;
                    shuffle(&mut directions, random);
                    let horizontal: Vec<Direction> = directions.into_iter().filter(|d| !matches!(d, Direction::Up | Direction::Down)).collect();
                    if up_to_branches - 1 == y {
                        for &direction in horizontal.iter().take(branch_amount.sample(random) as usize) {
                            self.place_log_axis(region, random, pos + direction.offset(), grown, Some(axis_of(direction)));
                        }
                    }
                }
                vec![Attachment::new(origin + IVec3::Y * up_to_branches, 0, false)]
            }
        }
    }

    fn forking_trunk(&self, region: &mut Region, random: &mut dyn RandomSource, height: i32, origin: IVec3, grown: &mut Grown) -> Vec<Attachment> {
        self.place_below(region, random, origin - IVec3::Y, grown);
        let mut attachments = Vec::new();
        let lean = random_horizontal(random);
        let lean_height = height - random.next_int_bounded(4) - 1;
        let mut lean_steps = 3 - random.next_int_bounded(3);
        let (mut tx, mut tz) = (origin.x, origin.z);
        let mut top = None;
        for yo in 0..height {
            let yy = origin.y + yo;
            if yo >= lean_height && lean_steps > 0 {
                tx += lean.offset().x;
                tz += lean.offset().z;
                lean_steps -= 1;
            }
            if self.place_log(region, random, IVec3::new(tx, yy, tz), grown) {
                top = Some(yy + 1);
            }
        }
        if let Some(y) = top {
            attachments.push(Attachment::new(IVec3::new(tx, y, tz), 1, false));
        }
        let (mut tx, mut tz) = (origin.x, origin.z);
        let branch = random_horizontal(random);
        if branch != lean {
            let mut yo = lean_height - random.next_int_bounded(2) - 1;
            let mut steps = 1 + random.next_int_bounded(3);
            let mut top = None;
            while yo < height && steps > 0 {
                if yo >= 1 {
                    let yy = origin.y + yo;
                    tx += branch.offset().x;
                    tz += branch.offset().z;
                    if self.place_log(region, random, IVec3::new(tx, yy, tz), grown) {
                        top = Some(yy + 1);
                    }
                }
                yo += 1;
                steps -= 1;
            }
            if let Some(y) = top {
                attachments.push(Attachment::new(IVec3::new(tx, y, tz), 0, false));
            }
        }
        attachments
    }

    fn giant_trunk(&self, region: &mut Region, random: &mut dyn RandomSource, height: i32, origin: IVec3, grown: &mut Grown) -> Vec<Attachment> {
        let below = origin - IVec3::Y;
        for offset in [IVec3::ZERO, IVec3::X, IVec3::Z, IVec3::new(1, 0, 1)] {
            self.place_below(region, random, below + offset, grown);
        }
        for hh in 0..height {
            self.place_log_if_free(region, random, origin + IVec3::new(0, hh, 0), grown);
            if hh < height - 1 {
                self.place_log_if_free(region, random, origin + IVec3::new(1, hh, 0), grown);
                self.place_log_if_free(region, random, origin + IVec3::new(1, hh, 1), grown);
                self.place_log_if_free(region, random, origin + IVec3::new(0, hh, 1), grown);
            }
        }
        vec![Attachment::new(origin + IVec3::Y * height, 0, true)]
    }

    fn dark_oak_trunk(&self, region: &mut Region, random: &mut dyn RandomSource, height: i32, origin: IVec3, grown: &mut Grown) -> Vec<Attachment> {
        let mut attachments = Vec::new();
        let below = origin - IVec3::Y;
        for offset in [IVec3::ZERO, IVec3::X, IVec3::Z, IVec3::new(1, 0, 1)] {
            self.place_below(region, random, below + offset, grown);
        }
        let lean = random_horizontal(random);
        let lean_height = height - random.next_int_bounded(4);
        let mut lean_steps = 2 - random.next_int_bounded(3);
        let (x, y, z) = (origin.x, origin.y, origin.z);
        let (mut tx, mut tz) = (x, z);
        let top = y + height - 1;
        for dy in 0..height {
            if dy >= lean_height && lean_steps > 0 {
                tx += lean.offset().x;
                tz += lean.offset().z;
                lean_steps -= 1;
            }
            let pos = IVec3::new(tx, y + dy, tz);
            let block = region.get(pos);
            if region.blocks.is_air(block) || region.blocks.is(block, Tag::Leaves) {
                self.place_log(region, random, pos, grown);
                self.place_log(region, random, pos + IVec3::X, grown);
                self.place_log(region, random, pos + IVec3::Z, grown);
                self.place_log(region, random, pos + IVec3::new(1, 0, 1), grown);
            }
        }
        attachments.push(Attachment::new(IVec3::new(tx, top, tz), 0, true));
        for ox in -1..=2 {
            for oz in -1..=2 {
                if (!(0..=1).contains(&ox) || !(0..=1).contains(&oz)) && random.next_int_bounded(3) <= 0 {
                    let length = random.next_int_bounded(3) + 2;
                    for branch_y in 0..length {
                        self.place_log(region, random, IVec3::new(x + ox, top - branch_y - 1, z + oz), grown);
                    }
                    attachments.push(Attachment::new(IVec3::new(x + ox, top, z + oz), 0, false));
                }
            }
        }
        attachments
    }

    fn fancy_trunk(&self, region: &mut Region, random: &mut dyn RandomSource, tree_height: i32, origin: IVec3, grown: &mut Grown) -> Vec<Attachment> {
        let height = tree_height + 2;
        let trunk_height = (height as f64 * 0.618).floor() as i32;
        self.place_below(region, random, origin - IVec3::Y, grown);
        let clusters_per_y = 1.min((1.382 + (height as f64 / 13.0).powi(2)).floor() as i32);
        let trunk_top = origin.y + trunk_height;
        let mut relative_y = height - 5;
        let mut coords = vec![(origin + IVec3::Y * relative_y, trunk_top)];
        while relative_y >= 0 {
            let shape = fancy_tree_shape(height, relative_y);
            if shape >= 0.0 {
                for _ in 0..clusters_per_y {
                    let radius = shape as f64 * (random.next_float() as f64 + 0.328);
                    let angle = (random.next_float() * 2.0) as f64 * std::f64::consts::PI;
                    let x = radius * angle.sin() + 0.5;
                    let z = radius * angle.cos() + 0.5;
                    let check_start = origin + IVec3::new(x.floor() as i32, relative_y - 1, z.floor() as i32);
                    let check_end = check_start + IVec3::Y * 5;
                    if self.make_limb(region, random, check_start, check_end, false, grown) {
                        let (dx, dz) = (origin.x - check_start.x, origin.z - check_start.z);
                        let branch_height = check_start.y as f64 - ((dx * dx + dz * dz) as f64).sqrt() * 0.381;
                        let branch_top = if branch_height > trunk_top as f64 { trunk_top } else { branch_height as i32 };
                        let base = IVec3::new(origin.x, branch_top, origin.z);
                        if self.make_limb(region, random, base, check_start, false, grown) {
                            coords.push((check_start, base.y));
                        }
                    }
                }
            }
            relative_y -= 1;
        }
        self.make_limb(region, random, origin, origin + IVec3::Y * trunk_height, true, grown);
        for &(end, branch_base) in &coords {
            let base = IVec3::new(origin.x, branch_base, origin.z);
            if base != end && fancy_trim(height, branch_base - origin.y) {
                self.make_limb(region, random, base, end, true, grown);
            }
        }
        coords
            .into_iter()
            .filter(|&(_, branch_base)| fancy_trim(height, branch_base - origin.y))
            .map(|(pos, _)| Attachment::new(pos, 0, false))
            .collect()
    }

    fn make_limb(&self, region: &mut Region, random: &mut dyn RandomSource, start: IVec3, end: IVec3, place: bool, grown: &mut Grown) -> bool {
        if !place && start == end {
            return true;
        }
        let delta = end - start;
        let steps = delta.abs().max_element();
        let (dx, dy, dz) = (delta.x as f32 / steps as f32, delta.y as f32 / steps as f32, delta.z as f32 / steps as f32);
        for i in 0..=steps {
            let pos = start + IVec3::new((0.5 + i as f32 * dx).floor() as i32, (0.5 + i as f32 * dy).floor() as i32, (0.5 + i as f32 * dz).floor() as i32);
            if place {
                let (xdiff, zdiff) = ((pos.x - start.x).abs(), (pos.z - start.z).abs());
                let max = xdiff.max(zdiff);
                let axis = if max > 0 { if xdiff == max { "x" } else { "z" } } else { "y" };
                self.place_log_axis(region, random, pos, grown, Some(axis));
            } else if !self.is_free(region, pos) {
                return false;
            }
        }
        true
    }

    #[allow(clippy::too_many_arguments)]
    fn upwards_branch(
        &self,
        region: &mut Region,
        random: &mut dyn RandomSource,
        height: i32,
        attachments: &mut Vec<Attachment>,
        log_pos: IVec3,
        current: i32,
        direction: Direction,
        branch_pos: i32,
        mut steps: i32,
        grown: &mut Grown,
    ) {
        let mut along = current + branch_pos;
        let (mut x, mut z) = (log_pos.x, log_pos.z);
        let mut index = branch_pos;
        while index < height && steps > 0 {
            if index >= 1 {
                let y = current + index;
                x += direction.offset().x;
                z += direction.offset().z;
                along = y;
                let pos = IVec3::new(x, y, z);
                if self.place_log(region, random, pos, grown) {
                    along += 1;
                }
                attachments.push(Attachment::new(pos, 0, false));
            }
            index += 1;
            steps -= 1;
        }
        if along - current > 1 {
            let foliage = IVec3::new(x, along, z);
            attachments.push(Attachment::new(foliage, 0, false));
            attachments.push(Attachment::new(foliage - IVec3::Y * 2, 0, false));
        }
    }

    fn cherry_trunk(&self, region: &mut Region, random: &mut dyn RandomSource, height: i32, origin: IVec3, grown: &mut Grown) -> Vec<Attachment> {
        let TrunkKind::Cherry {
            branch_count,
            branch_horizontal_length,
            branch_start,
            branch_end,
        } = &self.trunk_placer.kind
        else {
            unreachable!()
        };
        self.place_below(region, random, origin - IVec3::Y, grown);
        let first = 0.max(height - 1 + IntProvider::Uniform(branch_start.0, branch_start.1).sample(random));
        let mut second = 0.max(height - 1 + IntProvider::Uniform(branch_start.0, branch_start.1 - 1).sample(random));
        if second >= first {
            second += 1;
        }
        let count = branch_count.sample(random);
        let middle = count == 3;
        let both = count >= 2;
        let trunk_height = if middle {
            height
        } else if both {
            first.max(second) + 1
        } else {
            first + 1
        };
        for y in 0..trunk_height {
            self.place_log(region, random, origin + IVec3::Y * y, grown);
        }
        let mut attachments = Vec::new();
        if middle {
            attachments.push(Attachment::new(origin + IVec3::Y * trunk_height, 0, false));
        }
        let direction = random_horizontal(random);
        let side_axis = axis_of(direction);
        attachments.push(self.cherry_branch(
            region,
            random,
            height,
            origin,
            side_axis,
            direction,
            first,
            first < trunk_height - 1,
            branch_horizontal_length,
            branch_end,
            grown,
        ));
        if both {
            attachments.push(self.cherry_branch(
                region,
                random,
                height,
                origin,
                side_axis,
                direction.opposite(),
                second,
                second < trunk_height - 1,
                branch_horizontal_length,
                branch_end,
                grown,
            ));
        }
        attachments
    }

    #[allow(clippy::too_many_arguments)]
    fn cherry_branch(
        &self,
        region: &mut Region,
        random: &mut dyn RandomSource,
        height: i32,
        origin: IVec3,
        side_axis: &'static str,
        direction: Direction,
        offset: i32,
        middle_continues: bool,
        horizontal_length: &IntProvider,
        branch_end: &IntProvider,
        grown: &mut Grown,
    ) -> Attachment {
        let mut log_pos = origin + IVec3::Y * offset;
        let end_offset = height - 1 + branch_end.sample(random);
        let extend = middle_continues || end_offset < offset;
        let distance = horizontal_length.sample(random) + if extend { 1 } else { 0 };
        let end = origin + direction.offset() * distance + IVec3::Y * end_offset;
        for _ in 0..if extend { 2 } else { 1 } {
            log_pos += direction.offset();
            self.place_log_axis(region, random, log_pos, grown, Some(side_axis));
        }
        let vertical = if end.y > log_pos.y { IVec3::Y } else { IVec3::NEG_Y };
        loop {
            let distance = manhattan(log_pos, end);
            if distance == 0 {
                return Attachment::new(end + IVec3::Y, 0, false);
            }
            let chance = (end.y - log_pos.y).abs() as f32 / distance as f32;
            let grow_vertically = random.next_float() < chance;
            log_pos += if grow_vertically { vertical } else { direction.offset() };
            self.place_log_axis(region, random, log_pos, grown, if grow_vertically { None } else { Some(side_axis) });
        }
    }

    fn try_place_leaf(&self, region: &mut Region, random: &mut dyn RandomSource, pos: IVec3, grown: &mut Grown) -> bool {
        let current = region.get(pos);
        if region.blocks.flag(current, PERSISTENT_BIT) || !valid_tree_pos(region, pos) {
            return false;
        }
        let state = self.foliage.state(region, random, pos);
        let state = waterlogged_if_wet(region.blocks, state, current);
        grown.foliage.insert(pos);
        region.set(pos, state);
        true
    }

    fn skip(&self, random: &mut dyn RandomSource, dx: i32, y: i32, dz: i32, radius: i32, double_trunk: bool) -> bool {
        match &self.foliage_placer.kind {
            FoliageKind::Blob { .. } => dx == radius && dz == radius && (random.next_int_bounded(2) == 0 || y == 0),
            FoliageKind::Bush { .. } => dx == radius && dz == radius && random.next_int_bounded(2) == 0,
            FoliageKind::Fancy { .. } => (dx as f32 + 0.5).powi(2) + (dz as f32 + 0.5).powi(2) > (radius * radius) as f32,
            FoliageKind::Spruce { .. } | FoliageKind::Pine { .. } => dx == radius && dz == radius && radius > 0,
            FoliageKind::Acacia => {
                if y == 0 {
                    (dx > 1 || dz > 1) && dx != 0 && dz != 0
                } else {
                    dx == radius && dz == radius && radius > 0
                }
            }
            FoliageKind::DarkOak => {
                if y == -1 && !double_trunk {
                    dx == radius && dz == radius
                } else {
                    y == 1 && dx + dz > radius * 2 - 2
                }
            }
            FoliageKind::MegaPine { .. } | FoliageKind::MegaJungle { .. } => dx + dz >= 7 || dx * dx + dz * dz > radius * radius,
            FoliageKind::RandomSpread { .. } => false,
            FoliageKind::Cherry { wide_bottom_hole, corner_hole, .. } => {
                if y == -1 && (dx == radius || dz == radius) && random.next_float() < *wide_bottom_hole {
                    return true;
                }
                let corner = dx == radius && dz == radius;
                if radius > 2 {
                    corner || (dx + dz > radius * 2 - 2 && random.next_float() < *corner_hole)
                } else {
                    corner && random.next_float() < *corner_hole
                }
            }
            FoliageKind::Poplar { .. } => unreachable!(),
        }
    }

    fn skip_signed(&self, random: &mut dyn RandomSource, dx: i32, y: i32, dz: i32, radius: i32, double_trunk: bool) -> bool {
        if matches!(self.foliage_placer.kind, FoliageKind::DarkOak) && y == 0 && double_trunk && (dx == -radius || dx >= radius) && (dz == -radius || dz >= radius) {
            return true;
        }
        let (min_dx, min_dz) = if double_trunk {
            (dx.abs().min((dx - 1).abs()), dz.abs().min((dz - 1).abs()))
        } else {
            (dx.abs(), dz.abs())
        };
        self.skip(random, min_dx, y, min_dz, radius, double_trunk)
    }

    #[allow(clippy::too_many_arguments)]
    fn leaves_row(&self, region: &mut Region, random: &mut dyn RandomSource, origin: IVec3, radius: i32, y: i32, double_trunk: bool, grown: &mut Grown) {
        let offset = if double_trunk { 1 } else { 0 };
        for dx in -radius..=radius + offset {
            for dz in -radius..=radius + offset {
                if !self.skip_signed(random, dx, y, dz, radius, double_trunk) {
                    self.try_place_leaf(region, random, origin + IVec3::new(dx, y, dz), grown);
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn leaves_row_hanging(&self, region: &mut Region, random: &mut dyn RandomSource, origin: IVec3, radius: i32, y: i32, double_trunk: bool, chance: f32, extension: f32, grown: &mut Grown) {
        self.leaves_row(region, random, origin, radius, y, double_trunk, grown);
        let offset = if double_trunk { 1 } else { 0 };
        let log_pos = origin - IVec3::Y;
        for along in Direction::HORIZONTAL {
            let to_edge = along.clockwise();
            let edge_offset = if matches!(to_edge, Direction::East | Direction::South) { radius + offset } else { radius };
            let mut pos = origin + IVec3::new(0, y - 1, 0) + to_edge.offset() * edge_offset + along.offset() * -radius;
            let mut along_offset = -radius;
            while along_offset < radius + offset {
                let leaves_above = grown.foliage.contains(pos + IVec3::Y);
                if leaves_above && self.try_place_extension(region, random, chance, log_pos, pos, grown) {
                    self.try_place_extension(region, random, extension, log_pos, pos - IVec3::Y, grown);
                }
                along_offset += 1;
                pos += along.offset();
            }
        }
    }

    fn try_place_extension(&self, region: &mut Region, random: &mut dyn RandomSource, chance: f32, log_pos: IVec3, pos: IVec3, grown: &mut Grown) -> bool {
        if manhattan(pos, log_pos) >= 7 || random.next_float() > chance {
            return false;
        }
        self.try_place_leaf(region, random, pos, grown)
    }

    #[allow(clippy::too_many_arguments)]
    fn create_foliage(&self, region: &mut Region, random: &mut dyn RandomSource, attachment: &Attachment, foliage_height: i32, leaf_radius: i32, offset: i32, grown: &mut Grown) {
        let double_trunk = attachment.double_trunk;
        let height_with_offset = foliage_height + attachment.height_offset;
        let pos = attachment.pos;
        match &self.foliage_placer.kind {
            FoliageKind::Blob { .. } => {
                for yo in (offset - height_with_offset..=offset).rev() {
                    let radius = 0.max(leaf_radius + attachment.radius_offset - 1 - yo / 2);
                    self.leaves_row(region, random, pos, radius, yo, double_trunk, grown);
                }
            }
            FoliageKind::Bush { .. } => {
                for yo in (offset - height_with_offset..=offset).rev() {
                    let radius = leaf_radius + attachment.radius_offset - 1 - yo;
                    self.leaves_row(region, random, pos, radius, yo, double_trunk, grown);
                }
            }
            FoliageKind::Fancy { .. } => {
                for yo in (offset - foliage_height..=offset).rev() {
                    let radius = leaf_radius + if yo != offset && yo != offset - foliage_height { 1 } else { 0 };
                    self.leaves_row(region, random, pos, radius, yo, double_trunk, grown);
                }
            }
            FoliageKind::Spruce { .. } => {
                let mut radius = random.next_int_bounded(2);
                let mut max_radius = 1;
                let mut min_radius = 0;
                for yo in (-height_with_offset..=offset).rev() {
                    self.leaves_row(region, random, pos, radius, yo, double_trunk, grown);
                    if radius >= max_radius {
                        radius = min_radius;
                        min_radius = 1;
                        max_radius = (max_radius + 1).min(leaf_radius + attachment.radius_offset);
                    } else {
                        radius += 1;
                    }
                }
            }
            FoliageKind::Pine { .. } => {
                let mut radius = 0;
                for yo in (offset - height_with_offset..=offset).rev() {
                    self.leaves_row(region, random, pos, radius, yo, double_trunk, grown);
                    if radius >= 1 && yo == offset - height_with_offset + 1 {
                        radius -= 1;
                    } else if radius < leaf_radius + attachment.radius_offset {
                        radius += 1;
                    }
                }
            }
            FoliageKind::Acacia => {
                let foliage_pos = pos + IVec3::Y * offset;
                self.leaves_row(region, random, foliage_pos, leaf_radius + attachment.radius_offset, -1 - height_with_offset, double_trunk, grown);
                self.leaves_row(region, random, foliage_pos, leaf_radius - 1, -height_with_offset, double_trunk, grown);
                self.leaves_row(region, random, foliage_pos, leaf_radius + attachment.radius_offset - 1, 0, double_trunk, grown);
            }
            FoliageKind::DarkOak => {
                let foliage_pos = pos + IVec3::Y * offset;
                if double_trunk {
                    self.leaves_row(region, random, foliage_pos, leaf_radius + 2, -1, double_trunk, grown);
                    self.leaves_row(region, random, foliage_pos, leaf_radius + 3, 0, double_trunk, grown);
                    self.leaves_row(region, random, foliage_pos, leaf_radius + 2, 1, double_trunk, grown);
                    if random.next_boolean() {
                        self.leaves_row(region, random, foliage_pos, leaf_radius, 2, double_trunk, grown);
                    }
                } else {
                    self.leaves_row(region, random, foliage_pos, leaf_radius + 2, -1, double_trunk, grown);
                    self.leaves_row(region, random, foliage_pos, leaf_radius + 1, 0, double_trunk, grown);
                }
            }
            FoliageKind::MegaPine { .. } => {
                let mut previous = 0;
                for yy in pos.y - height_with_offset + offset..=pos.y + offset {
                    let yo = pos.y - yy;
                    let smooth = leaf_radius + attachment.radius_offset + (yo as f32 / height_with_offset as f32 * 3.5).floor() as i32;
                    let jagged = if yo > 0 && smooth == previous && (yy & 1) == 0 { smooth + 1 } else { smooth };
                    self.leaves_row(region, random, IVec3::new(pos.x, yy, pos.z), jagged, 0, double_trunk, grown);
                    previous = smooth;
                }
            }
            FoliageKind::MegaJungle { .. } => {
                let leaf_height = if double_trunk { foliage_height } else { 1 + random.next_int_bounded(2) } + attachment.height_offset;
                for yo in (offset - leaf_height..=offset).rev() {
                    let radius = leaf_radius + attachment.radius_offset + 1 - yo;
                    self.leaves_row(region, random, pos, radius, yo, double_trunk, grown);
                }
            }
            FoliageKind::RandomSpread { attempts, .. } => {
                for _ in 0..*attempts {
                    let dx = random.next_int_bounded(leaf_radius) - random.next_int_bounded(leaf_radius);
                    let dy = random.next_int_bounded(foliage_height) - random.next_int_bounded(foliage_height);
                    let dz = random.next_int_bounded(leaf_radius) - random.next_int_bounded(leaf_radius);
                    self.try_place_leaf(region, random, pos + IVec3::new(dx, dy, dz), grown);
                }
            }
            FoliageKind::Cherry { hanging, hanging_extension, .. } => {
                let foliage_pos = pos + IVec3::Y * offset;
                let radius = leaf_radius + attachment.radius_offset - 1;
                self.leaves_row(region, random, foliage_pos, radius - 2, height_with_offset - 3, double_trunk, grown);
                self.leaves_row(region, random, foliage_pos, radius - 1, height_with_offset - 4, double_trunk, grown);
                for y in (0..=height_with_offset - 5).rev() {
                    self.leaves_row(region, random, foliage_pos, radius, y, double_trunk, grown);
                }
                self.leaves_row_hanging(region, random, foliage_pos, radius, -1, double_trunk, *hanging, *hanging_extension, grown);
                self.leaves_row_hanging(region, random, foliage_pos, radius - 1, -2, double_trunk, *hanging, *hanging_extension, grown);
            }
            FoliageKind::Poplar { side_hole, .. } => self.poplar_foliage(
                region,
                random,
                pos + IVec3::Y * offset,
                leaf_radius + attachment.radius_offset - 1,
                height_with_offset,
                double_trunk,
                *side_hole,
                grown,
            ),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn poplar_foliage(&self, region: &mut Region, random: &mut dyn RandomSource, origin: IVec3, radius: i32, height: i32, double_trunk: bool, side_hole: f32, grown: &mut Grown) {
        let flip = random.next_boolean();
        let row = |tree: &Self, region: &mut Region, random: &mut dyn RandomSource, radius: i32, y: i32, grown: &mut Grown| {
            let offset = if double_trunk { 1 } else { 0 };
            let partial = height - 1 == y || height - 2 == y;
            for dx in -radius..=radius + offset {
                for dz in -radius..=radius + offset {
                    let cut = poplar_corner_cut(dx, dz, radius, partial, flip);
                    let (abs_dx, abs_dz) = (dx.abs(), dz.abs());
                    let skip = if partial && (abs_dx == radius || abs_dz == radius) {
                        true
                    } else {
                        let extra = if random.next_float() <= side_hole { 1 } else { 0 };
                        abs_dx + abs_dz > radius * 2 - (cut + extra)
                    };
                    if !skip {
                        tree.try_place_leaf(region, random, origin + IVec3::new(dx, y, dz), grown);
                    }
                }
            }
        };
        row(self, region, random, radius - 2, height - 1, grown);
        row(self, region, random, radius - 1, height - 2, grown);
        row(self, region, random, radius - 1, height - 3, grown);
        for y in (1..=height - 4).rev() {
            row(self, region, random, radius, y, grown);
        }
        let log_y = height - 4;
        let offset = if double_trunk { 1 } else { 0 };
        for dx in -radius..=radius + offset {
            for dz in -radius..=radius + offset {
                let (abs_dx, abs_dz) = (dx.abs(), dz.abs());
                let partial = height - 1 == log_y || height - 2 == log_y;
                let cut = poplar_corner_cut(dx, dz, radius, partial, flip);
                if abs_dx + abs_dz <= radius * 2 - (cut + 2) && ((abs_dz == 0 && radius - abs_dx >= 4) || (abs_dx == 0 && radius - abs_dz >= 4)) {
                    let pos = origin + IVec3::new(dx, log_y, dz);
                    let expected = self.foliage.state(region, random, pos);
                    if region.get(pos) == expected {
                        let state = self.trunk.state(region, random, pos);
                        let state = region.blocks.with(state, pillar(if abs_dz == 0 { "x" } else { "z" }));
                        grown.foliage.insert(pos);
                        region.set(pos, state);
                    }
                }
            }
        }
        row(self, region, random, radius - 1, 0, grown);
        row(self, region, random, (radius - 2).clamp(1, 2), -1, grown);
    }
}

const MAX_LEAF_DISTANCE: usize = 7;

fn update_tree_edges(ctx: &mut Context, grown: &Grown, decorations: &[IVec3]) {
    let everything = grown.roots.order.iter().chain(&grown.trunks.order).chain(&grown.foliage.order).chain(decorations);
    let Some((min, max)) = everything.fold(None, |bounds: Option<(IVec3, IVec3)>, &pos| Some(bounds.map_or((pos, pos), |(min, max)| (min.min(pos), max.max(pos))))) else {
        return;
    };
    let size = max - min + IVec3::ONE;
    let index = |pos: IVec3| {
        let local = pos - min;
        (local.cmpge(IVec3::ZERO).all() && local.cmplt(size).all()).then(|| ((local.x * size.y + local.y) * size.z + local.z) as usize)
    };
    let mut filled = vec![false; (size.x * size.y * size.z) as usize];
    for pos in decorations.iter().chain(&grown.roots.order) {
        if let Some(i) = index(*pos) {
            filled[i] = true;
        }
    }
    let blocks = ctx.region.blocks;
    let mut visited = vec![false; filled.len()];
    let mut layers: Vec<Vec<IVec3>> = vec![Vec::new(); MAX_LEAF_DISTANCE];
    layers[0] = grown.trunks.iteration_order();
    let mut distance = 0;
    while distance < MAX_LEAF_DISTANCE {
        let Some(pos) = layers[distance].pop() else {
            distance += 1;
            continue;
        };
        let Some(i) = index(pos) else { continue };
        if visited[i] {
            continue;
        }
        visited[i] = true;
        filled[i] = true;
        for direction in Direction::ALL {
            let neighbour = pos + direction.offset();
            let Some(j) = index(neighbour) else { continue };
            if filled[j] {
                continue;
            }
            let state = ctx.region.get(neighbour);
            let neighbour_distance = if blocks.is(state, Tag::Logs) {
                0
            } else if blocks.is(state, Tag::Leaves) {
                MAX_LEAF_DISTANCE
            } else {
                continue;
            };
            let next = neighbour_distance.min(distance + 1);
            if next < MAX_LEAF_DISTANCE {
                layers[next].push(neighbour);
                distance = distance.min(next);
            }
        }
    }
    let full = |x: i32, y: i32, z: i32| x >= 0 && y >= 0 && z >= 0 && x < size.x && y < size.y && z < size.z && filled[((x * size.y + y) * size.z + z) as usize];
    let mut faces = Vec::new();
    for x in 0..size.x {
        for y in 0..size.y {
            for z in 0..=size.z {
                faces_between(&mut faces, full(x, y, z - 1), full(x, y, z), IVec3::new(x, y, z), Direction::North, Direction::South);
            }
        }
    }
    for z in 0..size.z {
        for x in 0..size.x {
            for y in 0..=size.y {
                faces_between(&mut faces, full(x, y - 1, z), full(x, y, z), IVec3::new(x, y, z), Direction::Down, Direction::Up);
            }
        }
    }
    for y in 0..size.y {
        for z in 0..size.z {
            for x in 0..=size.x {
                faces_between(&mut faces, full(x - 1, y, z), full(x, y, z), IVec3::new(x, y, z), Direction::West, Direction::East);
            }
        }
    }
    let shapes = Shapes {
        carpet: &ctx.catalog.moss_carpet,
        water: ctx.region.water(),
    };
    for (direction, local) in faces {
        let pos = min + local;
        let neighbour = pos + direction.offset();
        let state = ctx.region.get(pos);
        let updated = shapes.update(ctx.region, pos, state, direction);
        if updated != state {
            ctx.region.set(pos, updated);
        }
        let neighbour_state = ctx.region.get(neighbour);
        let neighbour_updated = shapes.update(ctx.region, neighbour, neighbour_state, direction.opposite());
        if neighbour_updated != neighbour_state {
            ctx.region.set(neighbour, neighbour_updated);
        }
    }
}

fn faces_between(faces: &mut Vec<(Direction, IVec3)>, last: bool, current: bool, at: IVec3, negative: Direction, positive: Direction) {
    if !last && current {
        faces.push((negative, at));
    }
    if last && !current {
        faces.push((positive, at - positive.offset()));
    }
}

fn poplar_corner_cut(dx: i32, dz: i32, radius: i32, partial: bool, flip: bool) -> i32 {
    let small_corner = if flip {
        (dx > 0 && dz > 0) || (dz < 0 && dx < 0)
    } else {
        (dx > 0 && dz < 0) || (dz > 0 && dx < 0)
    };
    if small_corner {
        radius - 1
    } else if partial {
        radius + 1
    } else {
        radius
    }
}

fn fancy_tree_shape(height: i32, y: i32) -> f32 {
    if (y as f32) < height as f32 * 0.3 {
        return -1.0;
    }
    let radius = height as f32 / 2.0;
    let adjacent = radius - y as f32;
    let mut distance = (radius * radius - adjacent * adjacent).sqrt();
    if adjacent == 0.0 {
        distance = radius;
    } else if adjacent.abs() >= radius {
        return 0.0;
    }
    distance * 0.5
}

fn fancy_trim(height: i32, local_y: i32) -> bool {
    local_y as f64 >= height as f64 * 0.2
}

impl TrunkPlacer {
    fn tree_height(&self, random: &mut dyn RandomSource) -> i32 {
        self.base_height + random.next_int_bounded(self.rand_a + 1) + random.next_int_bounded(self.rand_b + 1)
    }
}

impl FoliagePlacer {
    fn foliage_height(&self, random: &mut dyn RandomSource, tree_height: i32) -> i32 {
        match &self.kind {
            FoliageKind::Blob { height } | FoliageKind::Bush { height } | FoliageKind::Fancy { height } | FoliageKind::MegaJungle { height } => *height,
            FoliageKind::Spruce { trunk_height } => 4.max(tree_height - trunk_height.sample(random)),
            FoliageKind::Pine { height } | FoliageKind::Cherry { height, .. } | FoliageKind::Poplar { height, .. } | FoliageKind::RandomSpread { height, .. } => height.sample(random),
            FoliageKind::Acacia => 0,
            FoliageKind::DarkOak => 4,
            FoliageKind::MegaPine { crown_height } => crown_height.sample(random),
        }
    }

    fn foliage_radius(&self, random: &mut dyn RandomSource, trunk_height: i32) -> i32 {
        let radius = self.radius.sample(random);
        match self.kind {
            FoliageKind::Pine { .. } => radius + random.next_int_bounded((trunk_height + 1).max(1)),
            _ => radius,
        }
    }
}

impl MangroveRoots {
    fn can_place(&self, region: &Region, pos: IVec3) -> bool {
        valid_tree_pos(region, pos) || region.blocks.is(region.get(pos), self.can_grow_through)
    }

    fn place(&self, region: &mut Region, random: &mut dyn RandomSource, origin: IVec3, trunk_origin: IVec3, grown: &mut Grown) -> bool {
        let mut column = origin;
        while column.y < trunk_origin.y {
            if !self.can_place(region, column) {
                return false;
            }
            column += IVec3::Y;
        }
        let mut positions = vec![trunk_origin - IVec3::Y];
        for direction in Direction::HORIZONTAL {
            let pos = trunk_origin + direction.offset();
            let mut in_direction = Vec::new();
            if !self.simulate(region, random, pos, direction, trunk_origin, &mut in_direction, 0) {
                return false;
            }
            positions.extend(in_direction);
            positions.push(trunk_origin + direction.offset());
        }
        for pos in positions {
            self.place_root(region, random, pos, grown);
        }
        true
    }

    #[allow(clippy::too_many_arguments)]
    fn simulate(&self, region: &Region, random: &mut dyn RandomSource, pos: IVec3, direction: Direction, origin: IVec3, positions: &mut Vec<IVec3>, layer: i32) -> bool {
        if layer == self.max_root_length || positions.len() as i32 > self.max_root_length {
            return false;
        }
        for next in self.potential(pos, direction, random, origin) {
            if self.can_place(region, next) {
                positions.push(next);
                if !self.simulate(region, random, next, direction, origin, positions, layer + 1) {
                    return false;
                }
            }
        }
        true
    }

    fn potential(&self, pos: IVec3, direction: Direction, random: &mut dyn RandomSource, origin: IVec3) -> Vec<IVec3> {
        let below = pos - IVec3::Y;
        let next_to = pos + direction.offset();
        let width = manhattan(pos, origin);
        if width > self.max_root_width - 3 && width <= self.max_root_width {
            if random.next_float() < self.random_skew_chance {
                vec![below, next_to - IVec3::Y]
            } else {
                vec![below]
            }
        } else if width > self.max_root_width || random.next_float() < self.random_skew_chance {
            vec![below]
        } else if random.next_boolean() {
            vec![next_to]
        } else {
            vec![below]
        }
    }

    fn place_root(&self, region: &mut Region, random: &mut dyn RandomSource, pos: IVec3, grown: &mut Grown) {
        let current = region.get(pos);
        if self.muddy_roots_in.contains(&region.blocks.entry(current).kind) {
            grown.roots.insert(pos);
            region.set(pos, self.muddy_roots);
            return;
        }
        if !self.can_place(region, pos) {
            return;
        }
        let state = waterlogged_if_wet(region.blocks, self.root, current);
        grown.roots.insert(pos);
        region.set(pos, state);
        if let Some((above_state, chance)) = self.above_root {
            let above = pos + IVec3::Y;
            if random.next_float() < chance && region.blocks.is_air(region.get(above)) {
                let state = waterlogged_if_wet(region.blocks, above_state, region.get(above));
                grown.roots.insert(above);
                region.set(above, state);
            }
        }
    }
}

struct DecorationContext {
    logs: Vec<IVec3>,
    leaves: Vec<IVec3>,
    roots: Vec<IVec3>,
}

impl DecorationContext {
    fn lowest_trunk_or_root(&self) -> Vec<IVec3> {
        if self.roots.is_empty() {
            self.logs.clone()
        } else if !self.logs.is_empty() && self.roots[0].y == self.logs[0].y {
            self.logs.iter().chain(self.roots.iter()).copied().collect()
        } else {
            self.roots.clone()
        }
    }
}

fn is_air(region: &Region, pos: IVec3) -> bool {
    region.blocks.is_air(region.get(pos))
}

fn hanging_vine(region: &mut Region, pos: IVec3, vine: BlockId) {
    region.set(pos, vine);
    let mut cursor = pos - IVec3::Y;
    let mut remaining = 4;
    while is_air(region, cursor) && remaining > 0 {
        region.set(cursor, vine);
        cursor -= IVec3::Y;
        remaining -= 1;
    }
}

const VINE_SIDES: [IVec3; 4] = [IVec3::NEG_X, IVec3::X, IVec3::NEG_Z, IVec3::Z];

impl Decorator {
    fn place(&self, ctx: &mut Context, random: &mut dyn RandomSource, decoration: &mut DecorationContext) {
        match self {
            Self::AlterGround(provider) => {
                let positions = decoration.lowest_trunk_or_root();
                let Some(first) = positions.first() else { return };
                let min_y = first.y;
                for pos in positions.iter().filter(|pos| pos.y == min_y) {
                    for corner in [IVec3::new(-1, 0, -1), IVec3::new(2, 0, -1), IVec3::new(-1, 0, 2), IVec3::new(2, 0, 2)] {
                        alter_circle(ctx.region, random, provider, *pos + corner);
                    }
                    for _ in 0..5 {
                        let placement = random.next_int_bounded(64);
                        let (xx, zz) = (placement % 8, placement / 8);
                        if xx == 0 || xx == 7 || zz == 0 || zz == 7 {
                            alter_circle(ctx.region, random, provider, *pos + IVec3::new(-3 + xx, 0, -3 + zz));
                        }
                    }
                }
            }
            Self::AttachedToLeaves {
                probability,
                exclusion_xz,
                exclusion_y,
                provider,
                required_empty,
                directions,
            } => {
                let mut blacklist = HashSet::new();
                let mut leaves = decoration.leaves.clone();
                shuffle(&mut leaves, random);
                for leaf in leaves {
                    let direction = directions[random.next_int_bounded(directions.len() as i32) as usize];
                    let placement = leaf + direction.offset();
                    if !blacklist.contains(&placement) && random.next_float() < *probability && (1..=*required_empty).all(|i| is_air(ctx.region, leaf + direction.offset() * i)) {
                        for x in -exclusion_xz..=*exclusion_xz {
                            for y in -exclusion_y..=*exclusion_y {
                                for z in -exclusion_xz..=*exclusion_xz {
                                    blacklist.insert(placement + IVec3::new(x, y, z));
                                }
                            }
                        }
                        let state = provider.state(ctx.region, random, placement);
                        ctx.region.set(placement, state);
                    }
                }
            }
            Self::AttachedToLogs { probability, provider, directions } => {
                let mut logs = decoration.logs.clone();
                shuffle(&mut logs, random);
                for log in logs {
                    let direction = directions[random.next_int_bounded(directions.len() as i32) as usize];
                    let placement = log + direction.offset();
                    if random.next_float() <= *probability && is_air(ctx.region, placement) {
                        let state = provider.state(ctx.region, random, placement);
                        ctx.region.set(placement, state);
                    }
                }
            }
            Self::Beehive { probability, nest } => {
                let (logs, leaves) = (&decoration.logs, &decoration.leaves);
                if logs.is_empty() || random.next_float() >= *probability {
                    return;
                }
                let hive_y = match leaves.first() {
                    Some(leaf) => (leaf.y - 1).max(logs[0].y + 1),
                    None => (logs[0].y + 1 + random.next_int_bounded(3)).min(logs[logs.len() - 1].y),
                };
                let mut placements: Vec<IVec3> = logs
                    .iter()
                    .filter(|pos| pos.y == hive_y)
                    .flat_map(|&pos| [Direction::East, Direction::South, Direction::West].map(|d| pos + d.offset()))
                    .collect();
                if placements.is_empty() {
                    return;
                }
                shuffle(&mut placements, random);
                if let Some(&pos) = placements.iter().find(|&&pos| is_air(ctx.region, pos) && is_air(ctx.region, pos + IVec3::Z)) {
                    ctx.region.set(pos, *nest);
                    ctx.region.set_block_entity(pos, BlockEntity::Beehive);
                    for _ in 0..2 + random.next_int_bounded(2) {
                        random.next_int_bounded(599);
                    }
                }
            }
            Self::Cocoa { probability, states } => {
                if random.next_float() >= *probability {
                    return;
                }
                let Some(first) = decoration.logs.first() else { return };
                let tree_y = first.y;
                for pos in decoration.logs.iter().filter(|pos| pos.y - tree_y <= 2) {
                    for (index, direction) in Direction::HORIZONTAL.into_iter().enumerate() {
                        if random.next_float() <= 0.25 {
                            let cocoa = *pos + direction.opposite().offset();
                            if is_air(ctx.region, cocoa) {
                                let age = random.next_int_bounded(3) as usize;
                                ctx.region.set(cocoa, states[age * 4 + index]);
                            }
                        }
                    }
                }
            }
            Self::CreakingHeart { probability, heart } => {
                if decoration.logs.is_empty() || random.next_float() >= *probability {
                    return;
                }
                let mut placements = decoration.logs.clone();
                shuffle(&mut placements, random);
                let blocks = ctx.region.blocks;
                if let Some(&pos) = placements.iter().find(|&&pos| Direction::ALL.iter().all(|d| blocks.is(ctx.region.get(pos + d.offset()), Tag::Logs))) {
                    ctx.region.set(pos, *heart);
                }
            }
            Self::LeaveVine { probability, vines } => {
                for leaf in decoration.leaves.clone() {
                    for (side, vine) in VINE_SIDES.iter().zip(vines) {
                        if random.next_float() < *probability {
                            let pos = leaf + *side;
                            if is_air(ctx.region, pos) {
                                hanging_vine(ctx.region, pos, *vine);
                            }
                        }
                    }
                }
            }
            Self::PaleMoss {
                leaves,
                trunk,
                ground,
                patch,
                hanging,
                tip,
            } => {
                let mut logs = decoration.logs.clone();
                shuffle(&mut logs, random);
                let Some(origin) = logs.iter().copied().reduce(|best, pos| if pos.y < best.y { pos } else { best }) else {
                    return;
                };
                if random.next_float() < *ground {
                    let recorded = ctx.region.take_recording();
                    patch.place(ctx, random, origin + IVec3::Y);
                    ctx.region.resume_recording(recorded);
                }
                for pos in decoration.logs.clone() {
                    if random.next_float() < *trunk && is_air(ctx.region, pos - IVec3::Y) {
                        moss_hanger(ctx.region, random, pos - IVec3::Y, *hanging, *tip);
                    }
                }
                for pos in decoration.leaves.clone() {
                    if random.next_float() < *leaves && is_air(ctx.region, pos - IVec3::Y) {
                        moss_hanger(ctx.region, random, pos - IVec3::Y, *hanging, *tip);
                    }
                }
            }
            Self::PlaceOnGround { tries, radius, height, provider } => {
                let positions = decoration.lowest_trunk_or_root();
                let Some(&origin) = positions.first() else { return };
                let (mut min, mut max) = (origin, origin);
                for pos in positions.iter().filter(|pos| pos.y == origin.y) {
                    min = min.min(*pos);
                    max = max.max(*pos);
                }
                let min = IVec3::new(min.x - radius, origin.y - height, min.z - radius);
                let max = IVec3::new(max.x + radius, origin.y + height, max.z + radius);
                for _ in 0..*tries {
                    let x = random.next_int_between_inclusive(min.x, max.x);
                    let y = random.next_int_between_inclusive(min.y, max.y);
                    let z = random.next_int_between_inclusive(min.z, max.z);
                    let pos = IVec3::new(x, y, z);
                    let above = pos + IVec3::Y;
                    let above_block = ctx.region.get(above);
                    let blocks = ctx.region.blocks;
                    if (blocks.is_air(above_block) || blocks.name_of(above_block) == VINE)
                        && blocks.entry(ctx.region.get(pos)).full
                        && ctx.region.height(Heightmap::MotionBlockingNoLeaves, x, z) <= above.y
                    {
                        let state = provider.state(ctx.region, random, above);
                        ctx.region.set(above, state);
                    }
                }
            }
            Self::ShelfMushroom { probability, states } => {
                if random.next_float() >= *probability {
                    return;
                }
                let logs = &decoration.logs;
                if logs.is_empty() {
                    return;
                }
                if logs[0].y == logs[logs.len() - 1].y {
                    let directions = if logs[0].x != logs[logs.len() - 1].x {
                        [Direction::North, Direction::South]
                    } else {
                        [Direction::East, Direction::West]
                    };
                    for &log in logs {
                        for facing in directions {
                            if random.next_float() <= 0.25 {
                                let pos = log + facing.offset();
                                if shelf_replaceable(ctx.region, pos) && !adjacent_shelf(ctx.region, pos, states) && !adjacent_shelf(ctx.region, log, states) {
                                    place_shelf(ctx.region, random, pos, facing, states);
                                }
                            }
                        }
                    }
                } else {
                    let first = random_horizontal(random);
                    let directions = [first, first.clockwise()];
                    let base_y = logs[0].y;
                    for &log in logs {
                        let dy = log.y - base_y;
                        if !(1..=4).contains(&dy) {
                            continue;
                        }
                        for facing in directions {
                            if random.next_float() <= 0.25 {
                                let pos = log + facing.offset();
                                if shelf_replaceable(ctx.region, pos) && !states.contains(&ctx.region.get(pos - IVec3::Y)) {
                                    place_shelf(ctx.region, random, pos, facing, states);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            Self::TrunkVine { vines } => {
                for log in decoration.logs.clone() {
                    for (side, vine) in VINE_SIDES.iter().zip(vines) {
                        if random.next_int_bounded(3) > 0 {
                            let pos = log + *side;
                            if is_air(ctx.region, pos) {
                                ctx.region.set(pos, *vine);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn alter_circle(region: &mut Region, random: &mut dyn RandomSource, provider: &StateProvider, center: IVec3) {
    for xx in -2i32..=2 {
        for zz in -2i32..=2 {
            if xx.abs() == 2 && zz.abs() == 2 {
                continue;
            }
            let pos = center + IVec3::new(xx, 0, zz);
            for dy in (-3..=2).rev() {
                let cursor = pos + IVec3::Y * dy;
                if let Some(state) = provider.optional_state(region, random, cursor) {
                    region.set(cursor, state);
                    break;
                }
                if !is_air(region, cursor) && dy < 0 {
                    break;
                }
            }
        }
    }
}

fn moss_hanger(region: &mut Region, random: &mut dyn RandomSource, mut pos: IVec3, hanging: BlockId, tip: BlockId) {
    while is_air(region, pos - IVec3::Y) && random.next_float() >= 0.5 {
        region.set(pos, hanging);
        pos -= IVec3::Y;
    }
    region.set(pos, tip);
}

fn shelf_replaceable(region: &Region, pos: IVec3) -> bool {
    let water_near = [IVec3::ZERO, IVec3::X, IVec3::NEG_X, IVec3::NEG_Z, IVec3::Z]
        .iter()
        .any(|side| region.blocks.name_of(region.get(pos + *side)) == WATER);
    region.blocks.is(region.get(pos), Tag::Replaceable) && !water_near
}

fn adjacent_shelf(region: &Region, pos: IVec3, states: &[BlockId]) -> bool {
    Direction::HORIZONTAL.iter().any(|d| states.contains(&region.get(pos + d.offset())))
}

fn place_shelf(region: &mut Region, random: &mut dyn RandomSource, pos: IVec3, facing: Direction, states: &[BlockId]) {
    let age = random.next_int_bounded(2) as usize;
    let index = Direction::HORIZONTAL.iter().position(|&d| d == facing).unwrap_or(0);
    region.set(pos, states[age * 4 + index]);
}

pub struct FallenTree {
    pub trunk: StateProvider,
    pub length: IntProvider,
    pub stump_decorators: Vec<Decorator>,
    pub log_decorators: Vec<Decorator>,
}

impl FallenTree {
    pub fn place(&self, ctx: &mut Context, random: &mut dyn RandomSource, origin: IVec3) -> bool {
        let stump = self.place_log(ctx.region, random, origin, None);
        self.decorate(ctx, random, vec![stump], &self.stump_decorators);
        let direction = random_horizontal(random);
        let length = self.length.sample(random) - 2;
        let mut start = origin + direction.offset() * (2 + random.next_int_bounded(2));
        start += IVec3::Y;
        for _ in 0..6 {
            if valid_tree_pos(ctx.region, start) && solid_below(ctx.region, start) {
                break;
            }
            start -= IVec3::Y;
        }
        let mut gap = 0;
        let mut cursor = start;
        for _ in 0..length {
            if !valid_tree_pos(ctx.region, cursor) {
                return true;
            }
            if solid_below(ctx.region, cursor) {
                gap = 0;
            } else {
                gap += 1;
                if gap > 2 {
                    return true;
                }
            }
            cursor += direction.offset();
        }
        let mut logs = PositionSet::default();
        let mut cursor = start;
        for _ in 0..length {
            logs.insert(self.place_log(ctx.region, random, cursor, Some(axis_of(direction))));
            cursor += direction.offset();
        }
        self.decorate(ctx, random, logs.sorted_by_y(), &self.log_decorators);
        true
    }

    fn place_log(&self, region: &mut Region, random: &mut dyn RandomSource, pos: IVec3, axis: Option<&'static str>) -> IVec3 {
        let mut state = self.trunk.state(region, random, pos);
        if let Some(axis) = axis {
            state = region.blocks.with(state, pillar(axis));
        }
        region.set(pos, state);
        region.mark_above(pos);
        pos
    }

    fn decorate(&self, ctx: &mut Context, random: &mut dyn RandomSource, logs: Vec<IVec3>, decorators: &[Decorator]) {
        if decorators.is_empty() {
            return;
        }
        let mut decoration = DecorationContext {
            logs,
            leaves: Vec::new(),
            roots: Vec::new(),
        };
        for decorator in decorators {
            decorator.place(ctx, random, &mut decoration);
        }
    }
}

fn solid_below(region: &Region, pos: IVec3) -> bool {
    region.blocks.entry(region.get(pos - IVec3::Y)).full
}

pub struct HugeMushroom {
    pub red: bool,
    pub cap: Vec<BlockId>,
    pub stem: BlockId,
    pub radius: i32,
    pub can_place_on: BlockPredicate,
}

impl HugeMushroom {
    pub fn place(&self, ctx: &mut Context, random: &mut dyn RandomSource, origin: IVec3) -> bool {
        let mut height = random.next_int_bounded(3) + 4;
        if random.next_int_bounded(12) == 0 {
            height *= 2;
        }
        if !self.valid(ctx.region, origin, height) {
            return false;
        }
        self.make_cap(ctx.region, origin, height);
        for dy in 0..height {
            self.place_block(ctx.region, origin + IVec3::Y * dy, self.stem);
        }
        true
    }

    fn radius_for(&self, height: i32, yo: i32) -> i32 {
        if self.red {
            if (yo < height && yo >= height - 3) || yo == height { self.radius } else { 0 }
        } else if yo <= 3 {
            0
        } else {
            self.radius
        }
    }

    fn valid(&self, region: &Region, origin: IVec3, height: i32) -> bool {
        if origin.y < region.min_y() + 1 || origin.y + height + 1 > region.max_y() {
            return false;
        }
        if !self.can_place_on.test(region, origin - IVec3::Y) {
            return false;
        }
        for dy in 0..=height {
            let radius = self.radius_for(-1, dy);
            for dx in -radius..=radius {
                for dz in -radius..=radius {
                    let block = region.get(origin + IVec3::new(dx, dy, dz));
                    if !region.blocks.is_air(block) && !region.blocks.is(block, Tag::Leaves) {
                        return false;
                    }
                }
            }
        }
        true
    }

    fn place_block(&self, region: &mut Region, pos: IVec3, state: BlockId) {
        let current = region.get(pos);
        if region.blocks.is_air(current) || region.blocks.is(current, Tag::ReplaceableByMushrooms) {
            region.set(pos, state);
        }
    }

    fn cap_state(&self, up: bool, west: bool, east: bool, north: bool, south: bool) -> BlockId {
        let index = (up as usize) << 4 | (west as usize) << 3 | (east as usize) << 2 | (north as usize) << 1 | south as usize;
        self.cap[index]
    }

    fn make_cap(&self, region: &mut Region, origin: IVec3, height: i32) {
        let r = self.radius;
        if self.red {
            for dy in height - 3..=height {
                let radius = if dy < height { r } else { r - 1 };
                let center = r - 2;
                for dx in -radius..=radius {
                    for dz in -radius..=radius {
                        let x_edge = dx == -radius || dx == radius;
                        let z_edge = dz == -radius || dz == radius;
                        if dy >= height || x_edge != z_edge {
                            let state = self.cap_state(dy >= height - 1, dx < -center, dx > center, dz < -center, dz > center);
                            self.place_block(region, origin + IVec3::new(dx, dy, dz), state);
                        }
                    }
                }
            }
        } else {
            for dx in -r..=r {
                for dz in -r..=r {
                    let (min_x, max_x, min_z, max_z) = (dx == -r, dx == r, dz == -r, dz == r);
                    let (x_edge, z_edge) = (min_x || max_x, min_z || max_z);
                    if x_edge && z_edge {
                        continue;
                    }
                    let west = min_x || (z_edge && dx == 1 - r);
                    let east = max_x || (z_edge && dx == r - 1);
                    let north = min_z || (x_edge && dz == 1 - r);
                    let south = max_z || (x_edge && dz == r - 1);
                    let state = self.cap_state(true, west, east, north, south);
                    self.place_block(region, origin + IVec3::new(dx, height, dz), state);
                }
            }
        }
    }
}
