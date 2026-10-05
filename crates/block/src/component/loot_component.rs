use crate::component::block_component::BlockComponent;

/// What a block drops when a player breaks it in survival. Blocks without it drop themselves.
#[derive(Clone, Debug)]
pub struct LootComponent {
    pub item: Option<&'static str>,
}

impl BlockComponent for LootComponent {}

impl LootComponent {
    pub const fn item(item: &'static str) -> Self {
        Self { item: Some(item) }
    }

    pub const fn nothing() -> Self {
        Self { item: None }
    }
}
