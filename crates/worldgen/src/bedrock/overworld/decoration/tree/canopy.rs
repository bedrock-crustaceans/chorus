use super::draw::{Range, pick_weighted};
use super::mega::place_mega_canopy;
use super::roofed::place_roofed;
use super::site::{block, put};
use super::spec::{Canopy, SimpleCanopy};
use super::{Anchor, Tree};
use crate::bedrock::overworld::decoration::ore::Draw;
use crate::shared::blocks::BlockId;
use crate::shared::chunk::region::Region;
use glam::IVec3;

impl Canopy {
    pub fn place<D: Draw>(&self, tree: &Tree, region: &mut Region, random: &mut D, anchor: &Anchor) {
        let at = anchor.at;
        match self {
            Self::Simple(simple) => simple.place(tree, region, random, at),
            &Self::Acacia { leaf, size, simplify } => acacia(tree, region, at, leaf, size, simplify),
            &Self::Pine { leaf, height, base_radius } => {
                let height = height.draw(random);
                let top_cap = random.below(height + 1) + base_radius;
                pine(tree, region, at, leaf, height, top_cap);
            }
            &Self::Spruce {
                leaf,
                lower_offset,
                upper_offset,
                max_radius,
            } => spruce(tree, region, random, at, leaf, (lower_offset, upper_offset, max_radius)),
            &Self::Fancy { leaf, height, radius } => fancy(tree, region, at, leaf, height, radius),
            Self::Mega { .. } | Self::MegaPine { .. } => place_mega_canopy(self, tree, region, random, at),
            Self::Roofed { .. } => place_roofed(self, tree, region, random, at),
            Self::Mangrove(canopy) => canopy.place(tree, region, random, anchor),
            Self::Cherry(canopy) => canopy.place(tree, region, random, at),
            Self::RandomSpread { leaves, height, radius, attempts } => random_spread(tree, region, random, &anchor.candidates, leaves, [*height, *radius], *attempts),
            Self::Poplar(canopy) => canopy.place(tree, region, random, at),
        }
    }
}

fn random_spread<D: Draw>(tree: &Tree, region: &mut Region, random: &mut D, logs: &[IVec3], leaves: &[(BlockId, i32)], [height, radius]: [i32; 2], attempts: i32) {
    let height = Range::constant(height).draw_inclusive_always(random);
    let radius = Range::constant(radius).draw_inclusive_always(random);
    for &log in logs {
        for _ in 0..attempts {
            let leaf = pick_weighted(leaves, random);
            let mut jitter = |bound: i32| {
                let first = random.below(bound) - bound;
                first + random.below(bound) + 1
            };
            let offset = IVec3::new(jitter(radius), jitter(height), jitter(radius));
            if tree.valid_position(region, log + offset) {
                put(region, log + offset, leaf);
            }
        }
    }
}

fn fill(region: &mut Region, pos: IVec3, leaf: BlockId, gate: impl Fn(&Region, BlockId) -> bool) {
    if gate(region, block(region, pos)) {
        put(region, pos, leaf);
    }
}

fn ring(region: &mut Region, at: IVec3, radius: i32, cut_at_zero: bool, leaf: BlockId, gate: &impl Fn(&Region, BlockId) -> bool) {
    for dx in -radius..=radius {
        for dz in -radius..=radius {
            if (cut_at_zero || radius != 0) && dx.abs() == radius && dz.abs() == radius {
                continue;
            }
            fill(region, at + IVec3::new(dx, 0, dz), leaf, gate);
        }
    }
}

impl SimpleCanopy {
    fn slope(&self, dy: i32) -> i32 {
        let inverse = if self.rise != 0 { 1.0 / self.rise as f32 } else { 0.0 };
        ((self.run * dy) as f32 * inverse) as i32
    }

    fn cut_corner<D: Draw>(&self, layer: i32, random: &mut D) -> bool {
        let Some(last) = self.variation_chance.len().checked_sub(1) else { return false };
        self.variation_chance[(layer as usize).min(last)].roll(random)
    }

    fn place<D: Draw>(&self, tree: &Tree, region: &mut Region, random: &mut D, at: IVec3) {
        let (low, high) = self.offset;
        let top = self.slope(high);
        for dy in low..=high {
            let radius = top + self.min_width - self.slope(dy);
            if radius < 0 {
                continue;
            }
            for x in at.x - radius..=at.x + radius {
                for z in at.z - radius..=at.z + radius {
                    if (x - at.x).abs() == radius && (z - at.z).abs() == radius && self.cut_corner(dy - low, random) {
                        continue;
                    }
                    fill(region, IVec3::new(x, at.y + dy, z), self.leaf, |region, found| tree.foliage.leaf_replaceable(region.blocks, found));
                }
            }
        }
        let Some(decoration) = &self.decoration else { return };
        for dy in low..=high {
            let radius = self.slope(high - dy) + self.min_width;
            for x in at.x - radius..=at.x + radius {
                for z in at.z - radius..=at.z + radius {
                    let pos = IVec3::new(x, at.y + dy, z);
                    if block(region, pos) == self.leaf {
                        decoration.place(region, &tree.foliage, random, pos);
                    }
                }
            }
        }
    }
}

fn acacia(tree: &Tree, region: &mut Region, at: IVec3, leaf: BlockId, size: i32, simplify: bool) {
    let gate = |region: &Region, found: BlockId| tree.may_replace.allows(region.blocks, found);
    let wide = size.max(1);
    if simplify {
        for dx in 1 - wide..=wide - 1 {
            for dz in 1 - wide..=wide - 1 {
                fill(region, at + IVec3::new(dx, 1, dz), leaf, gate);
            }
        }
    } else {
        let narrow = size.max(2);
        for dx in 2 - narrow..=narrow - 2 {
            for dz in 2 - narrow..=narrow - 2 {
                fill(region, at + IVec3::new(dx, 1, dz), leaf, gate);
            }
        }
        for (dx, dz) in [(wide - 1, 0), (1 - wide, 0), (0, wide - 1), (0, 1 - wide)] {
            fill(region, at + IVec3::new(dx, 1, dz), leaf, gate);
        }
    }
    ring(region, at, size, true, leaf, &gate);
}

fn pine(tree: &Tree, region: &mut Region, at: IVec3, leaf: BlockId, height: i32, top_cap: i32) {
    let gate = |region: &Region, found: BlockId| tree.may_replace.allows(region.blocks, found);
    let mut radius = 0;
    let mut dy = 1;
    loop {
        ring(region, at + IVec3::Y * dy, radius, false, leaf, &gate);
        let grown = if radius < top_cap { radius + 1 } else { radius };
        let done = dy - 1 == -height;
        radius = if radius > 0 && dy == 2 - height { radius - 1 } else { grown };
        dy -= 1;
        if done {
            break;
        }
    }
}

fn spruce<D: Draw>(tree: &Tree, region: &mut Region, random: &mut D, at: IVec3, leaf: BlockId, ranges: (Range, Range, Range)) {
    let (lower_offset, upper_offset, max_radius) = ranges;
    let mut ground = at.y;
    while !tree.may_grow_on.allows(region.blocks, block(region, IVec3::new(at.x, ground - 1, at.z))) {
        if ground <= region.min_y() {
            return;
        }
        ground -= 1;
    }
    let lower = lower_offset.draw(random);
    let upper = upper_offset.draw(random);
    let cap = max_radius.draw(random);
    let mut radius = random.below(max_radius.max - max_radius.min);
    let layers = upper + at.y - lower - ground;
    if layers <= -1 {
        return;
    }
    let mut width_cap = max_radius.max - max_radius.min - 1;
    let mut reset_to = 0;
    let gate = |region: &Region, found: BlockId| tree.foliage.leaf_replaceable(region.blocks, found);
    for dy in 0..=layers {
        ring(region, IVec3::new(at.x, at.y + upper - dy, at.z), radius, false, leaf, &gate);
        let new_cap = if width_cap < cap { width_cap + 1 } else { cap };
        let growing = radius < width_cap;
        radius = if growing { radius + 1 } else { reset_to };
        if !growing {
            width_cap = new_cap;
            reset_to = 1;
        }
    }
}

fn fancy(tree: &Tree, region: &mut Region, at: IVec3, leaf: BlockId, height: i32, radius: i32) {
    let gate = |region: &Region, found: BlockId| tree.may_replace.allows(region.blocks, found);
    for dy in 0..height {
        let layer = if dy == 0 || dy == height - 1 { radius - 1 } else { radius };
        let limit = (layer * layer) as f64;
        for dx in -layer..=layer {
            let near = dx.abs() as f32 as f64 + 0.5;
            for dz in -layer..=layer {
                let far = dz.abs() as f64 + 0.5;
                if near * near + far * far <= limit {
                    fill(region, at + IVec3::new(dx, dy, dz), leaf, gate);
                }
            }
        }
    }
}
