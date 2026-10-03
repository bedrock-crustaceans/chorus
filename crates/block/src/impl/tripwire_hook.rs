use crate::block_definition::BlockDefinition;
use crate::block_id;
use crate::component::light_dampening_component::LightDampeningComponent;
use crate::component::solid_component::SolidComponent;
use crate::component::transparent_component::TransparentComponent;
use crate::const_block;
use crate::state::common::{ATTACHED_BIT, DIRECTION, POWERED_BIT};

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
