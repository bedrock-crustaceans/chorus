//! Declarative macros for the server stuff, mostly
//!
//! Expansions name their types absolutely, so both `chorus_world` and `chorus` alias themselves
//! with `extern crate self as ...` to stay usable from inside their own crate.

#[macro_export]
macro_rules! const_block {
    (
        identifier: $identifier:expr,
        states: [$($state:expr),* $(,)?],
        components: [$($component:expr),* $(,)?],
        permutations: [$($perm:expr),* $(,)?]$(,)?
    ) => {{
        ::chorus_world::block::block_definition::BlockDefinition {
            identifier: ::atomicow::CowArc::Static($identifier),

            states: ::atomicow::CowArc::Static(&[
                $(::atomicow::CowArc::Static(&$state)),*
            ]),

            components: ::atomicow::CowArc::Static(&[
                $(::atomicow::CowArc::Static(&$component)),*
            ]),

            permutations: ::atomicow::CowArc::Static(&[
                $(::atomicow::CowArc::Static(&$perm)),*
            ]),
        }
    }};
}

#[macro_export]
macro_rules! const_permutation {
    (
        condition: $cond:expr,
        components: [$($component:expr),* $(,)?]
    ) => {{
        ::chorus_world::block::block_definition::BlockPermutationDefinition {
            condition: $cond,
            components: ::atomicow::CowArc::Static(&[
                $(::atomicow::CowArc::Static(&$component)),*
            ]),
        }
    }};
}

#[macro_export]
macro_rules! const_bool {
    ($identifier:expr, $v:expr) => {{
        ::chorus_world::block::state::block_state::BlockStateDefinition::Bool {
            identifier: ::atomicow::CowArc::Static($identifier),
            default: $v,
        }
    }};
}

#[macro_export]
macro_rules! const_int {
    ($identifier:expr, $min:expr, $max:expr) => {{
        ::chorus_world::block::state::block_state::BlockStateDefinition::Int {
            identifier: ::atomicow::CowArc::Static($identifier),
            min: $min,
            max: $max,
        }
    }};
}

#[macro_export]
macro_rules! const_enum {
    ($identifier:expr, [$($v:expr),* $(,)?]) => {{
        ::chorus_world::block::state::block_state::BlockStateDefinition::Enum {
            identifier: ::atomicow::CowArc::Static($identifier),
            values: ::atomicow::CowArc::Static(&[
                $(::atomicow::CowArc::Static($v)),*
            ]),
        }
    }};
}

#[macro_export]
macro_rules! const_command {
    (
        name: $name:expr,
        description: $description:expr,
        aliases: [$($alias:expr),* $(,)?],
        permission: $permission:expr,
        overloads: [$($overload:expr),* $(,)?],
        execute: $execute:expr$(,)?
    ) => {{
        ::chorus::command::command_definition::CommandDefinition {
            name: ::atomicow::CowArc::Static($name),
            description: ::atomicow::CowArc::Static($description),
            aliases: ::atomicow::CowArc::Static(&[
                $(::atomicow::CowArc::Static(&$alias)),*
            ]),
            permission: $permission,
            overloads: ::atomicow::CowArc::Static(&[$($overload),*]),
            execute: $execute,
        }
    }};
}
