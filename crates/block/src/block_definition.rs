use crate::block_permutation::BlockPermutation;
use crate::component::block_component::BlockComponent;
use crate::component::block_components::BlockComponents;
use crate::state::block_state::{BlockState, BlockStateDefinition};
use atomicow::CowArc;
use chorus_core::utils::identifier::Identifier;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Debug)]
pub struct BlockDefinition {
    pub identifier: CowArc<'static, str>,
    pub states: CowArc<'static, [CowArc<'static, BlockStateDefinition>]>,
    pub components: CowArc<'static, [CowArc<'static, dyn BlockComponent>]>,
    pub permutations: CowArc<'static, [CowArc<'static, BlockPermutationDefinition>]>,
}

#[macro_export]
macro_rules! const_block {
    (
        identifier: $identifier:expr,
        states: [$($state:expr),* $(,)?],
        components: [$($component:expr),* $(,)?],
        permutations: [$($perm:expr),* $(,)?]$(,)?
    ) => {{
        $crate::block_definition::BlockDefinition {
            identifier: atomicow::CowArc::Static($identifier),

            states: atomicow::CowArc::Static(&[
                $(atomicow::CowArc::Static(&$state)),*
            ]),

            components: atomicow::CowArc::Static(&[
                $(atomicow::CowArc::Static(&$component)),*
            ]),

            permutations: atomicow::CowArc::Static(&[
                $(atomicow::CowArc::Static(&$perm)),*
            ]),
        }
    }};
}

impl BlockDefinition {
    pub fn new(
        identifier: impl AsRef<str>,
        states: impl IntoIterator<Item = BlockStateDefinition>,
        components: impl IntoIterator<Item = Arc<dyn BlockComponent>>,
        permutations: impl IntoIterator<Item = BlockPermutationDefinition>,
    ) -> Self {
        Self {
            identifier: CowArc::Owned(Arc::from(identifier.as_ref())),
            states: CowArc::Owned(states.into_iter().map(|v| CowArc::Owned(Arc::new(v))).collect::<Vec<_>>().into()),
            components: CowArc::Owned(components.into_iter().map(CowArc::Owned).collect::<Vec<_>>().into()),
            permutations: CowArc::Owned(permutations.into_iter().map(|p| CowArc::Owned(Arc::new(p))).collect::<Vec<_>>().into()),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        Identifier::validate(&self.identifier)?;

        let size: usize = self.states.iter().fold(1, |acc, s| acc * s.values_len());
        if size > u16::MAX as usize {
            return Err(format!("found {size} permutations, which exceeds the limit of 65535 permutations",));
        }

        let mut seen = HashSet::new();
        for state in self.states.as_ref() {
            let ident = state.identifier().as_ref();
            if !seen.insert(ident) {
                return Err(format!("found duplicate state identifier {:?}", ident));
            }
            state.validate()?;
        }
        Ok(())
    }

    pub fn generate(&self) -> (i32, HashMap<i32, BlockPermutation>, HashMap<i32, BlockComponents>) {
        let state_combinations = self.states.iter().fold(vec![HashMap::new()], |acc, state| {
            acc.into_iter()
                .flat_map(|map| {
                    state.get_values().into_iter().map(move |val| {
                        let mut m = map.clone();
                        m.insert(state.identifier().clone(), val);
                        m
                    })
                })
                .collect()
        });

        let permutations: Vec<BlockPermutation> = state_combinations.into_iter().map(move |states| BlockPermutation::new(self, states)).collect();

        let default = permutations.iter().find(|p| p.get_index() == 0).unwrap().get_hash();

        // permutations that match the same overrides get the same components, so build each set once
        let mut shared: HashMap<Vec<bool>, BlockComponents> = HashMap::new();
        let components: HashMap<i32, BlockComponents> = permutations
            .iter()
            .map(|permutation| {
                let matched: Vec<bool> = self.permutations.iter().map(|p| (p.condition)(permutation.get_states())).collect();
                let components = shared
                    .entry(matched)
                    .or_insert_with_key(|matched| {
                        let mut components = BlockComponents::new();

                        for c in self.components.as_ref() {
                            components.insert(c.clone());
                        }

                        for (p, _) in self.permutations.iter().zip(matched).filter(|(_, matched)| **matched) {
                            for c in p.components.as_ref() {
                                components.insert(c.clone());
                            }
                        }

                        components
                    })
                    .clone();

                (permutation.get_hash(), components)
            })
            .collect();

        let permutations: HashMap<i32, BlockPermutation> = permutations.into_iter().map(|p| (p.get_hash(), p)).collect();

        (default, permutations, components)
    }
}

impl From<BlockDefinition> for CowArc<'_, BlockDefinition> {
    fn from(def: BlockDefinition) -> Self {
        CowArc::Owned(def.into())
    }
}

#[derive(Debug)]
pub struct BlockPermutationDefinition {
    pub condition: fn(&HashMap<CowArc<'static, str>, BlockState>) -> bool,
    pub components: CowArc<'static, [CowArc<'static, dyn BlockComponent>]>,
}

#[macro_export]
macro_rules! const_permutation {
    (
        condition: $cond:expr,
        components: [$($component:expr),* $(,)?]
    ) => {{
        $crate::block_definition::BlockPermutationDefinition {
            condition: $cond,
            components: atomicow::CowArc::Static(&[
                $(atomicow::CowArc::Static(&$component)),*
            ]),
        }
    }};
}

impl BlockPermutationDefinition {
    pub fn new(condition: fn(&HashMap<CowArc<'static, str>, BlockState>) -> bool, components: impl IntoIterator<Item = Arc<dyn BlockComponent>>) -> Self {
        Self {
            condition,
            components: CowArc::Owned(components.into_iter().map(CowArc::Owned).collect::<Vec<_>>().into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::mineable_component::MineableComponent;
    use crate::r#impl::acacia_log::ACACIA_LOG;
    use crate::r#impl::air::AIR;
    use crate::r#impl::grass_block::GRASS_BLOCK;

    fn lever_like() -> BlockDefinition {
        BlockDefinition::new(
            "test:lever",
            [
                BlockStateDefinition::new_bool("open_bit", false),
                BlockStateDefinition::new_int("level", 0, 2),
                BlockStateDefinition::new_enum("kind", ["a", "b"]),
            ],
            [],
            [],
        )
    }

    #[test]
    fn a_stateless_block_generates_one_permutation() {
        let (default, permutations, components) = GRASS_BLOCK.generate();
        assert_eq!(permutations.len(), 1);
        assert_eq!(components.len(), 1);
        let permutation = &permutations[&default];
        assert_eq!(permutation.get_identifier(), "minecraft:grass_block");
        assert_eq!(permutation.get_index(), 0);
        assert!(permutation.get_states().is_empty());
    }

    #[test]
    fn a_stateless_block_hash_is_stable_across_generations() {
        let first = AIR.generate().0;
        assert_eq!(first, AIR.generate().0);
        assert_eq!(first, -604749536);
    }

    #[test]
    fn a_block_with_one_enum_state_generates_one_permutation_per_value() {
        let (default, permutations, components) = ACACIA_LOG.generate();
        assert_eq!(permutations.len(), 3);
        assert_eq!(components.len(), 3);
        assert_eq!(permutations[&default].get_state_value("pillar_axis"), Some(&BlockState::from("y")));

        let mut indices: Vec<u16> = permutations.values().map(|p| p.get_index()).collect();
        indices.sort_unstable();
        assert_eq!(indices, [0, 1, 2]);
    }

    #[test]
    fn combinatorial_states_multiply_and_every_hash_is_unique() {
        let definition = lever_like();
        let (default, permutations, components) = definition.generate();
        assert_eq!(permutations.len(), 2 * 3 * 2);
        assert_eq!(components.len(), 12);

        let mut indices: Vec<u16> = permutations.values().map(|p| p.get_index()).collect();
        indices.sort_unstable();
        assert_eq!(indices, (0..12).collect::<Vec<u16>>());
        assert_eq!(permutations[&default].get_index(), 0);
        for (hash, permutation) in &permutations {
            assert_eq!(*hash, permutation.get_hash());
        }
    }

    #[test]
    fn generation_is_repeatable() {
        let definition = lever_like();
        let (default_a, permutations_a, _) = definition.generate();
        let (default_b, permutations_b, _) = definition.generate();
        assert_eq!(default_a, default_b);

        let mut a: Vec<_> = permutations_a.keys().copied().collect();
        let mut b: Vec<_> = permutations_b.keys().copied().collect();
        a.sort_unstable();
        b.sort_unstable();
        assert_eq!(a, b);
    }

    #[test]
    fn the_default_permutation_uses_the_first_value_of_every_state() {
        let definition = lever_like();
        let (default, permutations, _) = definition.generate();
        let states = permutations[&default].get_states();
        assert_eq!(states["open_bit"], false);
        assert_eq!(states["level"], 0);
        assert_eq!(states["kind"], "a");
    }

    #[test]
    fn conditional_components_apply_only_to_matching_permutations() {
        let definition = BlockDefinition::new(
            "test:conditional",
            [BlockStateDefinition::new_bool("lit", false)],
            [],
            [BlockPermutationDefinition::new(
                |states| states["lit"] == true,
                [Arc::new(MineableComponent::hardness(5.0)) as Arc<dyn BlockComponent>],
            )],
        );
        let (_, permutations, components) = definition.generate();

        for (hash, permutation) in &permutations {
            let lit = permutation.get_states()["lit"] == true;
            assert_eq!(components[hash].get::<MineableComponent>().is_some(), lit);
        }
    }

    #[test]
    fn validation_rejects_duplicate_state_names_and_oversized_blocks() {
        let duplicate = BlockDefinition::new("test:dup", [BlockStateDefinition::new_bool("a", false), BlockStateDefinition::new_bool("a", true)], [], []);
        assert!(duplicate.validate().is_err());

        let huge = BlockDefinition::new("test:huge", [BlockStateDefinition::new_int("a", 0, 255), BlockStateDefinition::new_int("b", 0, 255)], [], []);
        assert!(huge.validate().is_err());

        assert!(lever_like().validate().is_ok());
        assert!(AIR.validate().is_ok());
    }
}
