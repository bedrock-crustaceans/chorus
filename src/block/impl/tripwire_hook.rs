use crate::block::block_definition::BlockDefinition;
use crate::block::block_id;
use crate::block::component::light_dampening_component::LightDampeningComponent;
use crate::block::component::solid_component::SolidComponent;
use crate::block::component::transparent_component::TransparentComponent;
use crate::block::state::common::{ATTACHED_BIT, DIRECTION, POWERED_BIT};
use crate::const_block;

pub const TRIPWIRE_HOOK: BlockDefinition = const_block! {
    identifier: block_id::TRIPWIRE_HOOK,
    states: [ATTACHED_BIT, DIRECTION, POWERED_BIT],
    components: [
        SolidComponent { solid: false },
        TransparentComponent { transparent: true },
        LightDampeningComponent { dampening: 1 },
    ],
    permutations: [],
};
