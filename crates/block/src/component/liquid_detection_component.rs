use crate::component::block_component::BlockComponent;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiquidType {
    Water,
    Lava,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiquidTouch {
    Blocking,
    Broken,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LiquidRule {
    pub liquid_type: LiquidType,
    pub can_contain_liquid: bool,
    pub on_liquid_touches: LiquidTouch,
}

impl LiquidRule {
    pub const fn containing(liquid_type: LiquidType) -> Self {
        Self {
            liquid_type,
            can_contain_liquid: true,
            on_liquid_touches: LiquidTouch::Blocking,
        }
    }

    pub const fn containing_and_broken(liquid_type: LiquidType) -> Self {
        Self {
            liquid_type,
            can_contain_liquid: true,
            on_liquid_touches: LiquidTouch::Broken,
        }
    }

    pub const fn broken(liquid_type: LiquidType) -> Self {
        Self {
            liquid_type,
            can_contain_liquid: false,
            on_liquid_touches: LiquidTouch::Broken,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LiquidDetectionComponent {
    pub detection_rules: &'static [LiquidRule],
}

impl BlockComponent for LiquidDetectionComponent {}

impl LiquidDetectionComponent {
    pub const CONTAINS_WATER: Self = Self {
        detection_rules: &[LiquidRule::containing(LiquidType::Water)],
    };
    pub const CONTAINS_AND_BROKEN_BY_WATER: Self = Self {
        detection_rules: &[LiquidRule::containing_and_broken(LiquidType::Water)],
    };
    pub const CONTAINS_AND_BROKEN_BY_WATER_AND_LAVA: Self = Self {
        detection_rules: &[LiquidRule::containing_and_broken(LiquidType::Water), LiquidRule::broken(LiquidType::Lava)],
    };
    pub const BROKEN_BY_WATER: Self = Self {
        detection_rules: &[LiquidRule::broken(LiquidType::Water)],
    };
    pub const BROKEN_BY_LAVA: Self = Self {
        detection_rules: &[LiquidRule::broken(LiquidType::Lava)],
    };
    pub const BROKEN_BY_WATER_AND_LAVA: Self = Self {
        detection_rules: &[LiquidRule::broken(LiquidType::Water), LiquidRule::broken(LiquidType::Lava)],
    };

    pub fn rule(&self, liquid_type: LiquidType) -> Option<&LiquidRule> {
        self.detection_rules.iter().find(|rule| rule.liquid_type == liquid_type)
    }

    pub fn can_contain(&self, liquid_type: LiquidType) -> bool {
        self.rule(liquid_type).is_some_and(|rule| rule.can_contain_liquid)
    }

    pub fn touches(&self, liquid_type: LiquidType) -> LiquidTouch {
        self.rule(liquid_type).map_or(LiquidTouch::Blocking, |rule| rule.on_liquid_touches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_liquid_without_a_rule_is_blocked() {
        assert_eq!(LiquidDetectionComponent::BROKEN_BY_WATER.touches(LiquidType::Lava), LiquidTouch::Blocking);
        assert_eq!(LiquidDetectionComponent::CONTAINS_WATER.touches(LiquidType::Lava), LiquidTouch::Blocking);
    }

    #[test]
    fn a_liquid_with_a_breaking_rule_breaks_the_block() {
        assert_eq!(LiquidDetectionComponent::BROKEN_BY_WATER.touches(LiquidType::Water), LiquidTouch::Broken);
        assert_eq!(LiquidDetectionComponent::BROKEN_BY_WATER_AND_LAVA.touches(LiquidType::Lava), LiquidTouch::Broken);
        assert_eq!(LiquidDetectionComponent::BROKEN_BY_LAVA.touches(LiquidType::Lava), LiquidTouch::Broken);
    }

    #[test]
    fn a_containing_rule_blocks_the_liquid_and_holds_it() {
        assert_eq!(LiquidDetectionComponent::CONTAINS_WATER.touches(LiquidType::Water), LiquidTouch::Blocking);
        assert!(LiquidDetectionComponent::CONTAINS_WATER.can_contain(LiquidType::Water));
    }

    #[test]
    fn only_containing_rules_can_contain_a_liquid() {
        assert!(!LiquidDetectionComponent::CONTAINS_WATER.can_contain(LiquidType::Lava));
        assert!(!LiquidDetectionComponent::BROKEN_BY_WATER.can_contain(LiquidType::Water));
        assert!(LiquidDetectionComponent::CONTAINS_AND_BROKEN_BY_WATER.can_contain(LiquidType::Water));
        assert!(LiquidDetectionComponent::CONTAINS_AND_BROKEN_BY_WATER_AND_LAVA.can_contain(LiquidType::Water));
        assert!(!LiquidDetectionComponent::CONTAINS_AND_BROKEN_BY_WATER_AND_LAVA.can_contain(LiquidType::Lava));
    }
}
