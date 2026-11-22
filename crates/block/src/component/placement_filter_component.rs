use crate::component::block_component::BlockComponent;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlacementFace {
    Down,
    Up,
    North,
    South,
    East,
    West,
}

#[derive(Clone, Copy, Debug)]
pub struct PlacementRule {
    pub allowed_faces: &'static [PlacementFace],
    pub block_filter: &'static [&'static str],
}

#[derive(Clone, Debug)]
pub struct PlacementFilterComponent {
    pub conditions: &'static [PlacementRule],
    pub denied: &'static [&'static str],
}

impl BlockComponent for PlacementFilterComponent {}

impl PlacementFilterComponent {
    pub const fn new(conditions: &'static [PlacementRule]) -> Self {
        Self { conditions, denied: &[] }
    }

    pub const fn with_denied(conditions: &'static [PlacementRule], denied: &'static [&'static str]) -> Self {
        Self { conditions, denied }
    }

    pub fn denies(&self, block: &str) -> bool {
        self.denied.contains(&block)
    }

    pub fn allows(&self, face: PlacementFace, block: &str) -> bool {
        self.conditions.iter().any(|rule| rule.allowed_faces.contains(&face) && rule.block_filter.contains(&block))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ON_DIRT: &[PlacementRule] = &[PlacementRule {
        allowed_faces: &[PlacementFace::Up],
        block_filter: &["minecraft:dirt"],
    }];

    #[test]
    fn a_rule_matches_its_face_and_blocks() {
        let filter = PlacementFilterComponent::new(ON_DIRT);
        assert!(filter.allows(PlacementFace::Up, "minecraft:dirt"));
        assert!(!filter.allows(PlacementFace::Down, "minecraft:dirt"));
        assert!(!filter.allows(PlacementFace::Up, "minecraft:stone"));
    }

    #[test]
    fn a_filter_without_conditions_allows_nothing() {
        let filter = PlacementFilterComponent::new(&[]);
        assert!(!filter.allows(PlacementFace::Up, "minecraft:dirt"));
    }

    #[test]
    fn denied_blocks_are_listed_separately_from_allowed_ones() {
        let filter = PlacementFilterComponent::with_denied(ON_DIRT, &["minecraft:magma"]);
        assert!(filter.denies("minecraft:magma"));
        assert!(!filter.denies("minecraft:dirt"));
        assert!(filter.allows(PlacementFace::Up, "minecraft:dirt"));
        assert!(!PlacementFilterComponent::new(ON_DIRT).denies("minecraft:magma"));
    }
}
